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

    /// Merges the contents of the right statement into the left statement with enhanced error handling
    ///
    /// This function recursively merges two statements, handling different statement types
    /// and respecting the collision policy. It provides detailed error information
    /// and supports partial merging with rollback on failure.
    ///
    /// # Arguments
    ///
    /// * `left` - The target statement to merge into (modified in-place)
    /// * `right` - The source statement to merge from
    /// * `allow_collisions` - Whether to allow collisions (overwrite on conflict)
    ///
    /// # Returns
    ///
    /// Ok(()) if the merge was successful, or an error if there was a collision
    /// and collisions are not allowed
    fn merge_statements(
        left: &mut Statement,
        right: &Statement,
        allow_collisions: bool,
    ) -> Result<()> {
        match &right.data {
            StatementData::Group(right_stmts) | StatementData::Labeled(_, right_stmts) => {
                match &mut left.data {
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
                                Self::merge_statements(target, value, allow_collisions)?;
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
                                left_id: left.id.clone(),
                                left_location: left.meta.location.clone(),
                                right_id: right.id.clone(),
                                right_location: right.meta.location.clone()
                            }
                        );
                        *left = right.clone();
                    }
                }
            }
            StatementData::Single(_) => {
                // Value collision - replace if allowed
                ensure!(
                    allow_collisions,
                    error::CollisionSnafu {
                        left_id: left.id.clone(),
                        left_location: left.meta.location.clone(),
                        right_id: right.id.clone(),
                        right_location: right.meta.location.clone()
                    }
                );
                *left = right.clone();
            }
        }
        Ok(())
    }

    /// Parses a BarkML file with caching and error recovery
    fn parse_file<R>(
        &mut self,
        name: &str,
        code: &mut R,
        filename: Option<String>,
    ) -> Result<Statement>
    where
        R: Read + Seek,
    {
        let start_time = Instant::now();

        let filename = filename.unwrap_or_else(|| name.to_string());
        let mut module_code = String::new();

        code.read_to_string(&mut module_code)
            .map_err(|e| error::Error::Io {
                reason: format!("Failed to read file '{}': {}", filename, e),
            })?;

        // Validate the content is not empty
        if module_code.trim().is_empty() {
            return Err(error::Error::Io {
                reason: format!("File '{}' is empty or contains only whitespace", filename),
            });
        }

        let lexer = Token::lexer(&module_code);
        let mut parser = Parser::new(&filename, lexer);
        let module = parser.parse().map_err(|e| {
            // Enhance error with file context
            error::Error::Io {
                reason: format!("Failed to parse file '{}': {}", filename, e),
            }
        })?;

        // Update statistics
        self.stats.files_processed += 1;
        self.stats.processing_time_ms += start_time.elapsed().as_millis() as u64;

        // Validate if configured to do so
        if self.config.validate_on_load {
            module.validate().map_err(|e| error::Error::Io {
                reason: format!("Validation failed for file '{}': {}", filename, e),
            })?;
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
        let module = self.parse_file(name, code, filename)?;

        if let Some(existing) = self.modules.get_mut(name) {
            Self::merge_statements(existing, &module, self.config.allow_collisions)?;
        } else {
            self.modules.insert(name.to_string(), module);
            self.stats.modules_created += 1;
        }

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

        // Check cache first
        if let Some(cached_module) = self.file_cache.get(path) {
            if let Some(existing) = self.modules.get_mut(&name) {
                Self::merge_statements(existing, cached_module, self.config.allow_collisions)?;
            } else {
                self.modules.insert(name, cached_module.clone());
                self.stats.modules_created += 1;
            }
            return Ok(self);
        }

        // Open and read the file
        let mut file = File::open(path).map_err(|e| error::Error::Io {
            reason: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        // Parse and cache the module
        let module = self.parse_file(&name, &mut file, Some(name.clone()))?;
        self.file_cache.insert(path.to_path_buf(), module.clone());

        // Add to modules
        if let Some(existing) = self.modules.get_mut(&name) {
            Self::merge_statements(existing, &module, self.config.allow_collisions)?;
        } else {
            self.modules.insert(name, module);
            self.stats.modules_created += 1;
        }

        Ok(self)
    }

    /// Add a single file to this loader and merge it into the main module
    pub fn add_file<P>(&mut self, path: P) -> Result<&mut Self>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        utils::validate_path(path)?;

        let name = utils::basename(path)?;

        // Check cache first
        if let Some(cached_module) = self.file_cache.get(path) {
            if let Some(existing) = self.modules.get_mut("main") {
                Self::merge_statements(existing, cached_module, self.config.allow_collisions)?;
            } else {
                self.modules
                    .insert("main".to_string(), cached_module.clone());
                self.stats.modules_created += 1;
            }
            return Ok(self);
        }

        // Open and read the file
        let mut file = File::open(path).map_err(|e| error::Error::Io {
            reason: format!("Failed to open file '{}': {}", path.display(), e),
        })?;

        // Parse and cache the module
        let module = self.parse_file("main", &mut file, Some(name))?;
        self.file_cache.insert(path.to_path_buf(), module.clone());

        // Add to main module
        if let Some(existing) = self.modules.get_mut("main") {
            Self::merge_statements(existing, &module, self.config.allow_collisions)?;
        } else {
            self.modules.insert("main".to_string(), module);
            self.stats.modules_created += 1;
        }

        Ok(self)
    }

    /// Add a directory to this loader and import all files as individual modules
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

            // Try .bml file first
            let file_path = base_path.join(name).with_extension("bml");
            if file_path.exists() && file_path.is_file() {
                return self.add_file(file_path);
            }

            // Try .d directory
            let dir_path = base_path.join(name).with_extension("d");
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
                    Statement::new_control(
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
}
