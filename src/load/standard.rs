use std::{
    fs::File,
    io::{Read, Seek},
    path::Path,
    time::Instant,
};

use super::{LoadStats, Loader, LoaderConfig, utils};
use crate::{Result, error};
use crate::{
    StatementData,
    ast::Statement,
    syn::{Parser, Token},
};
use indexmap::IndexMap;
use logos::Logos;
use snafu::ensure;

/// Maximum unclosed `{`/`[` count in the raw source text. Nesting depth in
/// the parsed AST can never exceed this (every recursion level — block,
/// table, or array — needs a literal opening delimiter), so it is a safe
/// upper bound for choosing the parse strategy.
fn raw_nesting_depth(source: &str) -> usize {
    let mut depth = 0usize;
    let mut max = 0usize;
    for c in source.chars() {
        match c {
            '{' | '[' => {
                depth += 1;
                max = max.max(depth);
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max
}

/// Standard loader for BarkML files with enhanced capabilities
///
/// This loader supports multiple methodologies for reading and combining BarkML files:
/// - Loading individual files with validation
/// - Loading directories of files with filtering
/// - Merging multiple files into a single module with conflict resolution
/// - Importing files as separate modules with namespace management
/// - Auto-discovering files in search paths with caching
/// - Performance monitoring and statistics collection
/// - Robust error handling and recovery
///
/// The loader can be configured through LoaderConfig to handle various scenarios
/// and provides detailed statistics about the loading process.
pub struct StandardLoader {
    /// Map of module names to their corresponding statements
    modules: IndexMap<String, Statement>,

    /// Configuration for the loader
    config: LoaderConfig,

    /// Statistics about the loading process
    stats: LoadStats,

    /// Cache of parsed files to avoid re-parsing
    file_cache: IndexMap<std::path::PathBuf, Statement>,
}

impl Default for StandardLoader {
    /// Create a new loader with the default configuration
    fn default() -> Self {
        Self::new(LoaderConfig::default())
    }
}

impl StandardLoader {
    /// Creates a new StandardLoader with the specified configuration
    pub fn new(config: LoaderConfig) -> Self {
        Self {
            modules: IndexMap::new(),
            config,
            stats: LoadStats::new(),
            file_cache: IndexMap::new(),
        }
    }

    /// Creates a new StandardLoader with a builder pattern
    pub fn builder() -> StandardLoaderBuilder {
        StandardLoaderBuilder::new()
    }

    /// Gets the current loading statistics
    pub fn stats(&self) -> &LoadStats {
        &self.stats
    }

    /// Clears the file cache to free memory
    pub fn clear_cache(&mut self) {
        self.file_cache.clear();
    }

    /// Gets the number of cached files
    pub fn cache_size(&self) -> usize {
        self.file_cache.len()
    }

    /// Merges a source statement into the stored module `dest_key` transactionally
    ///
    /// # Transaction boundary
    ///
    /// One source-to-destination merge is atomic. The fallible merge runs against
    /// a staged copy of only the affected destination module; on success the staged
    /// copy is swapped in (an infallible commit), and on failure the stored module
    /// is left completely unchanged — values, children, insertion order, labels,
    /// metadata, and UIDs. The source is never modified. If no module exists under
    /// `dest_key`, the source is inserted directly, which is trivially atomic.
    ///
    /// This guarantee covers every path that merges modules: `add_module`,
    /// `import`, `add_file` (including cache hits), and directory APIs through
    /// them. Directory APIs (`import_dir`, `add_dir`) are atomic **per file**:
    /// a later file's failure retains earlier successful file merges; there is
    /// no whole-directory rollback.
    ///
    /// Caching and statistics are not part of the rollback: a successfully
    /// parsed source may remain in the file cache after a conflicting merge,
    /// and `files_processed`/`modules_created` count parses and module
    /// creations respectively, not merge outcomes.
    fn merge_module_transactional(&mut self, dest_key: &str, source: &Statement) -> Result<()> {
        match self.modules.get_mut(dest_key) {
            Some(existing) => {
                // Stage once: run the fallible merge on a copy of the affected
                // module only, then commit by swapping it in.
                let mut staged = existing.clone();
                Self::merge_into_staged(&mut staged, source, self.config.allow_collisions)?;
                self.modules.insert(dest_key.to_string(), staged);
                Ok(())
            }
            None => {
                self.modules.insert(dest_key.to_string(), source.clone());
                self.stats.modules_created += 1;
                Ok(())
            }
        }
    }

    /// Recursively merges `right` into `staged`, respecting the collision policy
    ///
    /// This is the fallible half of the transactional merge in
    /// [`merge_module_transactional`](Self::merge_module_transactional): it
    /// mutates `staged` destructively and may fail partway, leaving `staged`
    /// partially merged. Callers must only ever pass a disposable staged copy
    /// and publish it on `Ok`.
    ///
    /// # Arguments
    ///
    /// * `staged` - The staged statement to merge into (modified in-place)
    /// * `right` - The source statement to merge from (never modified)
    /// * `allow_collisions` - Whether to allow collisions (overwrite on conflict)
    ///
    /// # Returns
    ///
    /// Ok(()) if the merge was successful, or an error if there was a collision
    /// and collisions are not allowed
    fn merge_into_staged(
        staged: &mut Statement,
        right: &Statement,
        allow_collisions: bool,
    ) -> Result<()> {
        match &right.data {
            StatementData::Group(right_stmts) | StatementData::Labeled(_, right_stmts) => {
                match &mut staged.data {
                    StatementData::Group(left_stmts) | StatementData::Labeled(_, left_stmts) => {
                        // Pre-allocate capacity for better performance
                        let additional_capacity =
                            right_stmts.len().saturating_sub(left_stmts.len());
                        if additional_capacity > 0 {
                            left_stmts.reserve(additional_capacity);
                        }

                        // Merge each child statement
                        for (key, value) in right_stmts {
                            if let Some(target) = left_stmts.get_mut(key) {
                                // Recursive merge for existing keys
                                Self::merge_into_staged(target, value, allow_collisions)?;
                            } else {
                                // Simple insert for new keys
                                left_stmts.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    _ => {
                        // Type mismatch - replace if collisions are allowed
                        ensure!(
                            allow_collisions,
                            error::CollisionSnafu {
                                left_id: staged.id.clone(),
                                left_location: staged.meta.location.clone(),
                                right_id: right.id.clone(),
                                right_location: right.meta.location.clone()
                            }
                        );
                        *staged = right.clone();
                    }
                }
            }
            StatementData::Single(_) => {
                // Value collision - replace if allowed
                ensure!(
                    allow_collisions,
                    error::CollisionSnafu {
                        left_id: staged.id.clone(),
                        left_location: staged.meta.location.clone(),
                        right_id: right.id.clone(),
                        right_location: right.meta.location.clone()
                    }
                );
                *staged = right.clone();
            }
        }
        Ok(())
    }

    /// Parses a BarkML source with caching and error preservation
    ///
    /// `name` is the logical module identity; `path` is the physical source
    /// path when one exists (file loads). Typed parse/semantic errors are
    /// wrapped in [`error::Error::Load`] when a physical path is known, and
    /// forwarded untouched for in-memory sources. Genuine read failures
    /// (including invalid UTF-8, which fails `read_to_string`) stay `Io`, as
    /// does the documented empty-source policy.
    fn parse_file<R>(&mut self, name: &str, code: &mut R, path: Option<&Path>) -> Result<Statement>
    where
        R: Read + Seek,
    {
        let start_time = Instant::now();

        let display_name = path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| name.to_string());
        let mut module_code = String::new();

        code.read_to_string(&mut module_code)
            .map_err(|e| error::Error::Io {
                reason: format!("Failed to read file '{}': {}", display_name, e),
            })?;

        // Validate the content is not empty
        if module_code.trim().is_empty() {
            return Err(error::Error::Io {
                reason: format!(
                    "File '{}' is empty or contains only whitespace",
                    display_name
                ),
            });
        }

        let wrap = |e: error::Error| match path {
            Some(p) => error::Error::Load {
                path: p.to_path_buf(),
                module: name.to_string(),
                source: Box::new(e),
            },
            None => e,
        };

        // Parsing is deeply recursive in nesting depth. Shallow documents
        // (the common case) parse inline on the caller's thread; deeper ones
        // get a dedicated large-stack thread so they fail with a typed
        // RecursionLimit error instead of a stack overflow, without paying a
        // thread spawn per parsed module. The raw-text delimiter scan only
        // ever overestimates nesting (delimiters in strings/comments), never
        // underestimates it, so the inline path stays safely shallow.
        let module = if raw_nesting_depth(&module_code) < 12 {
            let lexer = Token::lexer(&module_code);
            let mut parser = Parser::new(name, lexer);
            parser.parse()
        } else {
            let source_name = name.to_string();
            std::thread::Builder::new()
                .stack_size(16 * 1024 * 1024)
                .spawn(move || {
                    let lexer = Token::lexer(&module_code);
                    let mut parser = Parser::new(&source_name, lexer);
                    parser.parse()
                })
                .map_err(|e| error::Error::Io {
                    reason: format!("Failed to spawn parse thread: {}", e),
                })?
                .join()
                .map_err(|_| error::Error::Io {
                    reason: "Parse thread panicked".to_string(),
                })?
        }
        .map_err(wrap)?;

        // Update statistics
        self.stats.files_processed += 1;
        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;

        // Validate if configured to do so
        if self.config.validate_on_load {
            module.validate().map_err(wrap)?;
        }

        Ok(module)
    }

    /// Add a module with the given name to this loader with enhanced error handling
    pub fn add_module<R>(
        &mut self,
        name: &str,
        code: &mut R,
        filename: Option<String>,
    ) -> Result<&mut Self>
    where
        R: Read + Seek,
    {
        // In-memory source: no filesystem path is invented; the optional
        // filename keeps its role as the source name recorded in locations.
        let source_name = filename.as_deref().unwrap_or(name);
        let module = self.parse_file(source_name, code, None)?;

        self.merge_module_transactional(name, &module)?;

        Ok(self)
    }

    /// Add a single file to this loader as a new module with validation
    pub fn import<P>(&mut self, path: P) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        utils::validate_path(path)?;

        let name = utils::basename(path)?;

        // Check cache first (merge is transactional either way)
        if let Some(cached_module) = self.file_cache.get(path).cloned() {
            self.merge_module_transactional(&name, &cached_module)?;
            return Ok(self);
        }

        // Open and read the file
        let mut file = File::open(path).map_err(|e| error::Error::Io {
            reason: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        // Parse and cache the module
        let module = self.parse_file(&name, &mut file, Some(path))?;
        self.file_cache.insert(path.to_path_buf(), module.clone());

        self.merge_module_transactional(&name, &module)?;

        Ok(self)
    }

    /// Add a single file to this loader and merge it into the main module
    pub fn add_file<P>(&mut self, path: P) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        utils::validate_path(path)?;

        let _ = utils::basename(path)?; // retains Basename error behavior

        // Check cache first (merge is transactional either way)
        if let Some(cached_module) = self.file_cache.get(path).cloned() {
            self.merge_module_transactional("main", &cached_module)?;
            return Ok(self);
        }

        // Open and read the file
        let mut file = File::open(path).map_err(|e| error::Error::Io {
            reason: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        // Parse and cache the module; module identity is "main" but the
        // physical path stays the file's own for diagnostics
        let module = self.parse_file("main", &mut file, Some(path))?;
        self.file_cache.insert(path.to_path_buf(), module.clone());

        self.merge_module_transactional("main", &module)?;

        Ok(self)
    }

    /// Add a directory to this loader and import all files as individual modules
    ///
    /// # Transaction boundary
    ///
    /// Each file is imported atomically: if one file's merge fails, that file is
    /// fully unmerged, but earlier successful imports are retained. There is no
    /// whole-directory rollback. Files are processed in sorted order.
    pub fn import_dir<P>(&mut self, path: P) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        utils::validate_path(path)?;

        let files = utils::discover_files(path)?;

        for file in &files {
            self.import(file)?;
        }

        Ok(self)
    }

    /// Add a directory to this loader and merge all files into the main module
    ///
    /// # Transaction boundary
    ///
    /// Each file's merge into `main` is atomic: if one file fails, that file is
    /// fully unmerged, but earlier successful file merges are retained in `main`.
    /// There is no whole-directory rollback. Files are processed in sorted order.
    pub fn add_dir<P>(&mut self, path: P) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        utils::validate_path(path)?;

        let files = utils::discover_files(path)?;

        for file in &files {
            self.add_file(file)?;
        }

        Ok(self)
    }

    /// Load the main module with auto-discovery in search paths
    pub fn main<P>(&mut self, name: &str, search_paths: Vec<P>) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        // Try each search path in order
        for path in &search_paths {
            let base_path = path.as_ref();

            // Try .bml file first (append rather than with_extension,
            // which would mangle dotted module names like `app.conf`)
            let file_path = base_path.join(format!("{name}.bml"));
            if file_path.exists() && file_path.is_file() {
                return self.add_file(file_path);
            }

            // Try .d directory
            let dir_path = base_path.join(format!("{name}.d"));
            if dir_path.exists() && dir_path.is_dir() {
                return self.add_dir(dir_path);
            }
        }

        // Not found in any search path
        Err(error::Error::Search {
            name: name.to_string(),
            search_paths: search_paths
                .iter()
                .map(|x| x.as_ref().to_path_buf())
                .collect(),
        })
    }

    /// Gets all module names currently loaded
    pub fn module_names(&self) -> Vec<&String> {
        self.modules.keys().collect()
    }

    /// Gets a specific module by name
    pub fn get_module(&self, name: &str) -> Option<&Statement> {
        self.modules.get(name)
    }

    /// Checks if a module exists
    pub fn has_module(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Removes a module by name
    pub fn remove_module(&mut self, name: &str) -> Option<Statement> {
        self.modules.shift_remove(name)
    }
}

impl Loader for StandardLoader {
    fn is_resolution_enabled(&self) -> bool {
        self.config.resolve_macros
    }

    fn max_recursion_depth(&self) -> usize {
        self.config.max_recursion_depth
    }

    fn skip_macro_resolution(&mut self) -> Result<&mut Self> {
        self.config.resolve_macros = false;
        Ok(self)
    }

    fn read(&self) -> Result<Statement> {
        self.modules
            .get("main")
            .cloned()
            .ok_or(error::Error::NoMain)
    }
}

/// Builder for StandardLoader with fluent interface
pub struct StandardLoaderBuilder {
    config: LoaderConfig,
}

impl StandardLoaderBuilder {
    pub fn new() -> Self {
        Self {
            config: LoaderConfig::default(),
        }
    }

    pub fn resolve_macros(mut self, resolve: bool) -> Self {
        self.config.resolve_macros = resolve;
        self
    }

    pub fn allow_collisions(mut self, allow: bool) -> Self {
        self.config.allow_collisions = allow;
        self
    }

    pub fn max_recursion_depth(mut self, depth: usize) -> Self {
        self.config.max_recursion_depth = depth;
        self
    }

    pub fn validate_on_load(mut self, validate: bool) -> Self {
        self.config.validate_on_load = validate;
        self
    }

    pub fn add_search_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config.search_paths.push(path.as_ref().to_path_buf());
        self
    }

    pub fn build(self) -> StandardLoader {
        StandardLoader::new(self.config)
    }
}

impl Default for StandardLoaderBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Location, Metadata, Statement, Value};
    use indexmap::IndexMap;
    use semver::Version;

    #[test]
    fn test_builder_pattern() {
        let loader = StandardLoader::builder()
            .resolve_macros(false)
            .allow_collisions(true)
            .validate_on_load(true)
            .build();

        assert!(!loader.config.resolve_macros);
        assert!(loader.config.allow_collisions);
        assert!(loader.config.validate_on_load);
    }

    #[test]
    fn test_module_management() {
        let mut loader = StandardLoader::default();

        // Initially no modules
        assert_eq!(loader.module_names().len(), 0);
        assert!(!loader.has_module("test"));

        // Add a module manually for testing
        let module = Statement::new_module("test", IndexMap::new(), Metadata::default());
        loader.modules.insert("test".to_string(), module);

        assert_eq!(loader.module_names().len(), 1);
        assert!(loader.has_module("test"));
        assert!(loader.get_module("test").is_some());

        // Remove module
        let removed = loader.remove_module("test");
        assert!(removed.is_some());
        assert!(!loader.has_module("test"));
    }

    #[test]
    fn test_statistics() {
        let loader = StandardLoader::default();
        let stats = loader.stats();

        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.modules_created, 0);
        assert_eq!(stats.macros_resolved, 0);
    }

    #[test]
    fn test_cache_management() {
        let mut loader = StandardLoader::default();

        assert_eq!(loader.cache_size(), 0);

        // Add something to cache manually for testing
        let module = Statement::new_module("test", IndexMap::new(), Metadata::default());
        loader.file_cache.insert("test.bml".into(), module);

        assert_eq!(loader.cache_size(), 1);

        loader.clear_cache();
        assert_eq!(loader.cache_size(), 0);
    }

    // Include the original tests for compatibility
    #[test]
    pub fn load_single() {
        let expected = Statement::new_module(
            ".",
            IndexMap::from([
                (
                    "tire".into(),
                    Statement::new_assign(
                        "tire",
                        None,
                        Value::new_version(
                            Version::new(1, 0, 0),
                            Metadata {
                                location: Location::default(),
                                comment: None,
                                label: Some("Test".into()),
                            },
                        ),
                        Metadata::default(),
                    )
                    .unwrap(),
                ),
                (
                    "section-1".into(),
                    Statement::new_block(
                        "section-1",
                        Vec::new(),
                        IndexMap::from([
                            (
                                "number".into(),
                                Statement::new_assign(
                                    "number",
                                    None,
                                    Value::new_int(4, Metadata::default()),
                                    Metadata {
                                        location: Location::default(),
                                        comment: Some("Documentation".into()),
                                        label: None,
                                    },
                                )
                                .unwrap(),
                            ),
                            (
                                "floating".into(),
                                Statement::new_assign(
                                    "floating",
                                    Some(crate::ValueType::F32),
                                    Value::new_f32(3.14, Metadata::default()),
                                    Metadata::default(),
                                )
                                .unwrap(),
                            ),
                            (
                                "versioning".into(),
                                Statement::new_assign(
                                    "versioning",
                                    None,
                                    Value::new_version(
                                        Version::parse("1.2.3-beta.6").unwrap(),
                                        Metadata::default(),
                                    ),
                                    Metadata::default(),
                                )
                                .unwrap(),
                            ),
                            (
                                "requires".into(),
                                Statement::new_assign(
                                    "requires",
                                    None,
                                    Value::new_require(
                                        semver::VersionReq::parse("^1.3.3").unwrap(),
                                        Metadata::default(),
                                    ),
                                    Metadata::default(),
                                )
                                .unwrap(),
                            ),
                            (
                                "strings".into(),
                                Statement::new_assign(
                                    "strings",
                                    None,
                                    Value::new_string("hello world".into(), Metadata::default()),
                                    Metadata::default(),
                                )
                                .unwrap(),
                            ),
                        ]),
                        Metadata::default(),
                    ),
                ),
            ]),
            Metadata::default(),
        );

        let mut loader = StandardLoader::default();
        let result = loader
            .add_module(
                "main",
                &mut std::io::Cursor::new(include_str!("../../examples/simple.bml")),
                None,
            )
            .unwrap()
            .load()
            .unwrap();

        assert_eq!(result, expected);
    }

    #[test]
    pub fn load_multiple() {
        let mut loader = StandardLoader::default();
        loader
            .add_module(
                "main",
                &mut std::io::Cursor::new(include_str!("../../examples/append.d/00-first.bml")),
                None,
            )
            .unwrap()
            .add_module(
                "main",
                &mut std::io::Cursor::new(include_str!("../../examples/append.d/01-second.bml")),
                None,
            )
            .unwrap();

        let result = loader.load().unwrap();
        assert!(result.find_by_path("section-1.number").is_some());
        assert!(result.find_by_path("section-2.number").is_some());
    }

    #[test]
    fn layered_merge_respects_collision_setting() {
        use std::io::Cursor;

        let base = b"name = \"base\"\n";
        let overlay = b"name = \"overlay\"\n";

        // Permissive merge of two separate sources succeeds (overlay wins)
        let mut loader = StandardLoader::builder().allow_collisions(true).build();
        loader
            .add_module("main", &mut Cursor::new(base.to_vec()), None)
            .unwrap();
        loader
            .add_module("main", &mut Cursor::new(overlay.to_vec()), None)
            .unwrap();
        let merged = loader.load().unwrap();
        assert!(merged.find_by_path("name").is_some());

        // Strict merge of the same clean sources fails with a merge collision
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut Cursor::new(base.to_vec()), None)
            .unwrap();
        let err = loader
            .add_module("main", &mut Cursor::new(overlay.to_vec()), None)
            .err()
            .unwrap();
        assert!(matches!(err, error::Error::Collision { .. }), "got: {err}");
    }

    #[test]
    fn duplicate_inside_one_source_fails_regardless_of_collision_setting() {
        use std::io::Cursor;

        let source = b"count = 1\ncount = 2\n";
        for allow in [true, false] {
            let mut loader = StandardLoader::builder().allow_collisions(allow).build();
            let err = loader
                .add_module("main", &mut Cursor::new(source.to_vec()), None)
                .err()
                .unwrap_or_else(|| panic!("duplicate must fail with allow_collisions={allow}"));
            // the typed duplicate diagnosis is forwarded through loading
            assert!(
                matches!(err, error::Error::DuplicateDeclaration { .. }),
                "got: {err}"
            );
        }
    }

    // ---- transactional merge regressions ----

    use std::io::Cursor;

    /// Full structural + metadata equality, unlike `PartialEq for Statement`
    /// which ignores uid/type_/meta. Debug output of IndexMap preserves
    /// insertion order, so this also pins exact child ordering.
    fn assert_fully_identical(a: &Statement, b: &Statement) {
        assert_eq!(
            format!("{a:?}"),
            format!("{b:?}"),
            "destination changed: metadata, uid, or ordering diverged"
        );
    }

    fn child_order(s: &Statement) -> Vec<String> {
        match &s.data {
            StatementData::Group(m) | StatementData::Labeled(_, m) => m.keys().cloned().collect(),
            StatementData::Single(_) => vec![],
        }
    }

    fn cursor(s: &str) -> Cursor<Vec<u8>> {
        Cursor::new(s.as_bytes().to_vec())
    }

    fn snapshot_main(loader: &StandardLoader) -> Statement {
        loader.get_module("main").expect("main must exist").clone()
    }

    #[test]
    fn failed_merge_preserves_destination_completely() {
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("a = 1\nb = 2\nnest { x = 1 }"), None)
            .unwrap();
        let before = snapshot_main(&loader);

        // new child first, nested addition second, conflicting value last
        let err = loader
            .add_module(
                "main",
                &mut cursor("newkey = 9\nnest { y = 2 }\na = 5"),
                None,
            )
            .err()
            .expect("conflicting merge must fail");
        assert!(matches!(err, error::Error::Collision { .. }), "got: {err}");

        let after = snapshot_main(&loader);
        assert_fully_identical(&before, &after);
        assert_eq!(child_order(&after), child_order(&before));
    }

    #[test]
    fn deep_conflict_and_shape_mismatches_are_atomic() {
        let mut loader = StandardLoader::default();
        loader
            .add_module(
                "main",
                &mut cursor("root = 1\nlvl1 { lvl2 { deep = 1\nother = 2 } }"),
                None,
            )
            .unwrap();
        let before = snapshot_main(&loader);

        // conflict several levels down, after root and nested additions
        let err = loader
            .add_module(
                "main",
                &mut cursor("newroot = 2\nlvl1 { newchild = 1\nlvl2 { newdeep = 3\ndeep = 9 } }"),
                None,
            )
            .err()
            .expect("deep conflict must fail");
        match err {
            error::Error::Collision {
                left_id, right_id, ..
            } => {
                assert_eq!(left_id, "deep");
                assert_eq!(right_id, "deep");
            }
            other => panic!("expected Collision, got: {other}"),
        }
        assert_fully_identical(&before, &snapshot_main(&loader));

        // group-over-value shape mismatch
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("val = 1"), None)
            .unwrap();
        let before = snapshot_main(&loader);
        let err = loader
            .add_module("main", &mut cursor("val { a = 1 }"), None)
            .err()
            .expect("value-over-group mismatch must fail");
        match err {
            error::Error::Collision {
                left_id, right_id, ..
            } => {
                assert_eq!(left_id, "val");
                assert_eq!(right_id, "val");
            }
            other => panic!("expected Collision, got: {other}"),
        }
        assert_fully_identical(&before, &snapshot_main(&loader));

        // value-over-group shape mismatch
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("grp { a = 1 }"), None)
            .unwrap();
        let before = snapshot_main(&loader);
        assert!(
            loader
                .add_module("main", &mut cursor("grp = 5"), None)
                .is_err()
        );
        assert_fully_identical(&before, &snapshot_main(&loader));
    }

    #[test]
    fn retry_after_failure_leaves_no_leftovers() {
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("a = 1"), None)
            .unwrap();
        let before = snapshot_main(&loader);

        let failing = "newkey = 9\na = 5";
        let source = failing.as_bytes().to_vec();
        assert!(
            loader
                .add_module("main", &mut Cursor::new(source.clone()), None)
                .is_err()
        );
        // source not consumed/mutated by failure
        assert_eq!(source, failing.as_bytes().to_vec());

        assert_fully_identical(&before, &snapshot_main(&loader));

        // corrected retry succeeds and contains nothing from the failed attempt
        loader
            .add_module("main", &mut cursor("newkey = 9\nsafe = 2"), None)
            .unwrap();
        let after = snapshot_main(&loader);
        assert_eq!(child_order(&after), vec!["a", "newkey", "safe"]);
    }

    #[test]
    fn successful_merge_semantics_preserved() {
        // disjoint merge preserves children and order
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("a = 1\nb = 2\nnest { x = 1 }"), None)
            .unwrap();
        loader
            .add_module("main", &mut cursor("nest { y = 2 }\nc = 3"), None)
            .unwrap();
        let m = snapshot_main(&loader);
        assert_eq!(child_order(&m), vec!["a", "b", "nest", "c"]);
        assert_eq!(
            child_order(m.children().find(|s| s.id == "nest").unwrap()),
            vec!["x", "y"]
        );

        // collision-enabled overwrite (including shape mismatch) retains unrelated children
        let mut loader = StandardLoader::builder().allow_collisions(true).build();
        loader
            .add_module("main", &mut cursor("a = 1\nkeep = 2\nval = 1"), None)
            .unwrap();
        loader
            .add_module("main", &mut cursor("a = 5\nval { x = 2 }\nnew = 6"), None)
            .unwrap();
        let m = snapshot_main(&loader);
        assert_eq!(child_order(&m), vec!["a", "keep", "val", "new"]);
        assert!(m.find_by_path("a").is_some());
        assert!(m.find_by_path("val.x").is_some());

        // equal-value duplicate across layers is still a collision when disabled
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("a = 1"), None)
            .unwrap();
        assert!(
            loader
                .add_module("main", &mut cursor("a = 1"), None)
                .is_err()
        );
    }

    #[test]
    fn failed_merge_does_not_create_modules_or_count_them() {
        let mut loader = StandardLoader::default();
        loader
            .add_module("main", &mut cursor("a = 1"), None)
            .unwrap();
        let created = loader.stats().modules_created;
        assert!(
            loader
                .add_module("main", &mut cursor("a = 2"), None)
                .is_err()
        );
        assert_eq!(loader.stats().modules_created, created);
        assert_eq!(loader.module_names().len(), 1);
    }

    #[test]
    fn file_and_directory_merges_are_atomic() {
        let dir = std::env::temp_dir().join(format!(
            "barkml-txn-{}-{:?}",
            std::process::id(),
            std::time::Instant::now()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file_a = dir.join("00-a.bml");
        let file_b = dir.join("01-b.bml");
        std::fs::write(&file_a, "shared = 1\na_only = 2").unwrap();
        std::fs::write(&file_b, "new = 3\nshared = 5").unwrap();

        // add_file: failed second file leaves only the first file's keys
        let mut loader = StandardLoader::default();
        loader.add_file(&file_a).unwrap();
        let before = loader.read().unwrap();
        assert!(loader.add_file(&file_b).is_err());
        let after = loader.read().unwrap();
        assert_fully_identical(&before, &after);
        assert!(after.find_by_path("a_only").is_some());
        assert!(after.find_by_path("new").is_none());

        // add_dir: per-file atomicity, prior successful merges retained
        let mut loader = StandardLoader::default();
        assert!(loader.add_dir(&dir).is_err());
        let m = loader.read().unwrap();
        assert!(m.find_by_path("a_only").is_some());
        assert!(m.find_by_path("shared").is_some());
        assert!(m.find_by_path("new").is_none());

        // cache-hit path: re-adding a cached file under strict mode must still
        // collide-check and leave the destination untouched
        let mut loader = StandardLoader::default();
        loader.add_file(&file_a).unwrap();
        assert_eq!(loader.cache_size(), 1);
        let before = loader.read().unwrap();
        assert!(loader.add_file(&file_a).is_err());
        assert_fully_identical(&before, &loader.read().unwrap());

        // import exercises the same transactional path for named modules:
        // re-importing the same basename merges (cache hit) and must collide-check
        let mut loader = StandardLoader::default();
        loader.import(&file_a).unwrap();
        let before = loader.get_module("00-a").unwrap().clone();
        assert!(loader.import(&file_a).is_err());
        assert_fully_identical(&before, loader.get_module("00-a").unwrap());

        std::fs::remove_dir_all(&dir).ok();
    }

    // ---- structured error preservation regressions (docs/13) ----

    use std::error::Error as StdError;

    fn direct_parse(src: &str, module: &str) -> crate::Result<Statement> {
        let mut parser = Parser::new(module, Token::lexer(src));
        parser.parse()
    }

    /// Malformed fixtures: unexpected token, EOF inside an open block,
    /// and a lexical failure.
    const MALFORMED: &[&str] = &["x = 1 }\n", "a {\n", "@ = 1\n"];

    fn temp_fixture(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "barkml-err-{}-{:?}",
            std::process::id(),
            std::time::Instant::now()
        ));
        let file = dir.join(name);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, content).unwrap();
        file
    }

    #[test]
    fn loader_errors_match_direct_parse() {
        for src in MALFORMED {
            let direct = direct_parse(src, "main").expect_err("must fail to parse");
            assert!(
                !matches!(direct, error::Error::Io { .. }),
                "fixture must be a parse error, got: {direct}"
            );

            // in-memory loading forwards the identical typed error
            let mut loader = StandardLoader::default();
            let err = loader
                .add_module("main", &mut cursor(src), None)
                .err()
                .unwrap();
            assert_eq!(err, direct, "add_module must forward the typed error");

            // file loading wraps the identical typed error in Load
            let file = temp_fixture("bad.bml", src);
            let mut loader = StandardLoader::default();
            let err = loader.import(&file).err().unwrap();
            let direct_named = direct_parse(src, "bad").err().unwrap();
            match &err {
                error::Error::Load {
                    path,
                    module,
                    source,
                } => {
                    assert_eq!(path, &file);
                    assert_eq!(module, "bad");
                    // same category/fields; only the module identity differs
                    assert_eq!(source.as_ref(), &direct_named);
                }
                other => panic!("expected Load wrapper, got: {other}"),
            }
            // human-readable output still names path and inner error
            assert!(err.to_string().contains(&file.display().to_string()));
            assert!(err.to_string().contains(&direct_named.to_string()));
            std::fs::remove_dir_all(file.parent().unwrap()).ok();
        }
    }

    #[test]
    fn load_wrapper_exposes_source_chain() {
        let file = temp_fixture("chain.bml", "a {\n");
        let mut loader = StandardLoader::default();
        let err = loader.add_file(&file).err().unwrap();

        let load = match &err {
            error::Error::Load {
                path,
                module,
                source,
            } => {
                assert_eq!(path, &file);
                // merged into `main` but reports its own physical path
                assert_eq!(module, "main");
                source
            }
            other => panic!("expected Load wrapper, got: {other}"),
        };
        // std::error::Error::source() exposes the original typed error
        // (typed access is the `source` field; `source()` hands the same
        // error to `dyn Error` consumers)
        assert_eq!(err.source().unwrap().to_string(), load.to_string());
        // leaf syntax errors are not forced to fabricate a source
        assert!(load.source().is_none());

        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    #[test]
    fn same_basename_in_different_dirs_is_distinguishable() {
        let file_x = temp_fixture("conf/x/dup.bml", "a {\n");
        let file_y = temp_fixture("conf/y/dup.bml", "a {\n");

        let mut loader = StandardLoader::default();
        let err_x = loader.import(&file_x).err().unwrap();
        let err_y = loader.import(&file_y).err().unwrap();
        let (
            error::Error::Load {
                path: px,
                module: mx,
                ..
            },
            error::Error::Load {
                path: py,
                module: my,
                ..
            },
        ) = (&err_x, &err_y)
        else {
            panic!("expected Load wrappers, got: {err_x:?} / {err_y:?}")
        };
        assert_ne!(px, py);
        assert_eq!(px, &file_x);
        assert_eq!(py, &file_y);
        // same logical module identity (same basename) is allowed to match
        assert_eq!(mx, my);

        std::fs::remove_dir_all(file_x.parent().unwrap().parent().unwrap()).ok();
        std::fs::remove_dir_all(file_y.parent().unwrap().parent().unwrap()).ok();
    }

    #[test]
    fn multibyte_crlf_and_eof_locations_survive_loading() {
        // multibyte UTF-8 before an error: column must count characters
        let src = "name = \"日本語テキスト\"\n@ bad\n";
        let direct = direct_parse(src, "main").err().unwrap();
        let mut loader = StandardLoader::default();
        let err = loader
            .add_module("main", &mut cursor(src), None)
            .err()
            .unwrap();
        assert_eq!(err, direct);
        assert!(matches!(err, error::Error::Expected { .. }));

        // CRLF input
        let crlf = "a = 1\r\nb {\r\n";
        let direct = direct_parse(crlf, "main").err().unwrap();
        let mut loader = StandardLoader::default();
        let err = loader
            .add_module("main", &mut cursor(crlf), None)
            .err()
            .unwrap();
        assert_eq!(err, direct);

        // error at EOF after a trailing newline
        let eof_src = "x = 1\ny {\n\n";
        let direct = direct_parse(eof_src, "main").err().unwrap();
        let mut loader = StandardLoader::default();
        let err = loader
            .add_module("main", &mut cursor(eof_src), None)
            .err()
            .unwrap();
        assert_eq!(err, direct);
    }

    #[test]
    fn non_ascii_source_filename_is_carried() {
        let file = temp_fixture("設定.bml", "a {\n");
        let mut loader = StandardLoader::default();
        match loader.import(&file).err().unwrap() {
            error::Error::Load { path, .. } => assert_eq!(path, file),
            other => panic!("expected Load wrapper, got: {other}"),
        }
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }

    /// Deterministic injected `Read + Seek` failure (not permission-based).
    struct FailingRead;
    impl Read for FailingRead {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected disk failure"))
        }
    }
    impl Seek for FailingRead {
        fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
            Ok(0)
        }
    }

    #[test]
    fn injected_read_failure_stays_io() {
        let mut loader = StandardLoader::default();
        let err = loader
            .add_module("main", &mut FailingRead, None)
            .err()
            .unwrap();
        match &err {
            error::Error::Io { reason } => {
                assert!(reason.contains("injected disk failure"), "got: {reason}")
            }
            other => panic!("genuine read failure must stay Io, got: {other}"),
        }
    }

    #[test]
    fn invalid_utf8_is_a_read_failure_not_a_syntax_error() {
        let mut loader = StandardLoader::default();
        let err = loader
            .add_module(
                "main",
                &mut Cursor::new(vec![0xff, 0xfe, b' ', b'=', b' ']),
                None,
            )
            .err()
            .unwrap();
        match &err {
            error::Error::Io { reason } => {
                assert!(reason.to_lowercase().contains("utf-8"), "got: {reason}")
            }
            other => panic!("invalid UTF-8 must be a read failure, got: {other}"),
        }
    }

    #[test]
    fn validation_on_load_keeps_semantic_error() {
        let mut loader = StandardLoader::builder().validate_on_load(true).build();
        let err = loader
            .add_module("main", &mut cursor("x: i32 = \"str\"\n"), None)
            .err()
            .unwrap();
        assert!(
            matches!(err, error::Error::Assign { .. }),
            "semantic validation failure must stay semantic, got: {err}"
        );
    }

    #[test]
    fn failed_parse_publishes_nothing() {
        let file = temp_fixture("unpublished.bml", "a {\n");
        let mut loader = StandardLoader::default();
        assert!(loader.import(&file).is_err());
        assert_eq!(loader.cache_size(), 0);
        assert!(!loader.has_module("unpublished"));
        assert_eq!(loader.module_names().len(), 0);

        assert!(loader.add_file(&file).is_err());
        assert!(!loader.has_module("main"));
        std::fs::remove_dir_all(file.parent().unwrap()).ok();
    }
}
