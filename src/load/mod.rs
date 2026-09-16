//! Module for loading and processing BarkML files
//!
//! This module provides comprehensive functionality for loading BarkML files from various sources,
//! resolving macros, and walking through the resulting AST. It supports multiple loading strategies
//! including single files, directories, and complex module hierarchies.
//!
//! # Key Components
//!
//! - **Loader Trait**: Defines the interface for all loaders
//! - **StandardLoader**: The primary implementation for loading BarkML files
//! - **Walk**: Ergonomic API for traversing and extracting data from loaded documents
//!
use crate::ast::{Scope, Statement};
use crate::{Result, error};
use std::path::Path;

mod standard;
mod walk;

pub use standard::*;
pub use walk::*;

/// LoaderInterface defines the shared interface for structs that
/// can read and load BarkML files.
///
/// Implementations of this trait provide different strategies for loading
/// BarkML content from various sources and with different configurations.
/// The trait is designed to be flexible and extensible, allowing for
/// custom loading strategies while maintaining a consistent API.
pub trait Loader {
    /// Checks if macro resolution is enabled for this loader
    ///
    /// When enabled, macros in the loaded content will be resolved and replaced
    /// with their actual values during the loading process.
    ///
    /// # Returns
    ///
    /// `true` if macro resolution is enabled, `false` otherwise
    fn is_resolution_enabled(&self) -> bool;

    /// Returns the maximum expansion depth for reference resolution.
    ///
    /// This guard bounds long acyclic dependency chains; reference cycles
    /// are detected structurally and are not affected by the limit.
    ///
    /// # Returns
    ///
    /// The configured limit, defaulting to 100
    fn max_recursion_depth(&self) -> usize {
        100
    }

    /// Disables macro resolution for this loader
    ///
    /// When disabled, macros in the loaded content will remain as-is,
    /// allowing for manual resolution later or inspection of the raw macro references.
    ///
    /// # Returns
    ///
    /// A mutable reference to self for method chaining, or an error if
    /// the operation cannot be completed
    fn skip_macro_resolution(&mut self) -> Result<&mut Self>;

    /// Reads all BarkML configuration files according to the loader's configuration
    /// and returns the resulting module statement
    ///
    /// This method performs the actual file reading and parsing but does not
    /// resolve macros. The returned statement tree may contain unresolved macro
    /// references that need to be processed separately.
    ///
    /// # Returns
    ///
    /// The parsed module statement, or an error if reading or parsing fails
    fn read(&self) -> Result<Statement>;

    /// Loads everything into a module statement and resolves macros if enabled
    ///
    /// This is the main entry point for loading BarkML content. It reads the content,
    /// then optionally resolves macros based on the loader's configuration.
    /// This method provides the complete loading pipeline in a single call.
    ///
    /// # Returns
    ///
    /// The fully processed module statement with macros resolved (if enabled),
    /// or an error if any step of the loading process fails
    fn load(&self) -> Result<Statement> {
        let mut module = self.read()?;
        if self.is_resolution_enabled() {
            let mut scope = Scope::with_limit(&module, self.max_recursion_depth());
            module = scope.apply()?;
        }
        Ok(module)
    }

    /// Validates the loaded content without fully processing it
    ///
    /// This method performs validation checks on the loaded content to ensure
    /// it's well-formed and doesn't contain obvious errors. It's useful for
    /// quick validation without the overhead of full macro resolution.
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation passes, or an error describing the validation failure
    fn validate(&self) -> Result<()> {
        let module = self.read()?;
        module.validate()?;

        // If macro resolution is enabled, validate that all macros can be resolved
        if self.is_resolution_enabled() {
            let scope = Scope::with_limit(&module, self.max_recursion_depth());
            scope.validate_references()?;
        }

        Ok(())
    }
}

/// Configuration options for loaders
///
/// This struct provides common configuration options that can be used
/// across different loader implementations to ensure consistent behavior.
#[derive(Debug, Clone)]
pub struct LoaderConfig {
    /// Whether to resolve macros during loading
    pub resolve_macros: bool,

    /// Whether to allow collisions between modules (overwrite on conflict)
    pub allow_collisions: bool,

    /// Maximum recursion depth for macro resolution
    pub max_recursion_depth: usize,

    /// Whether to validate content during loading
    pub validate_on_load: bool,

    /// Search paths for auto-discovery of modules
    pub search_paths: Vec<std::path::PathBuf>,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            resolve_macros: true,
            allow_collisions: false,
            max_recursion_depth: 100,
            validate_on_load: false,
            search_paths: vec![std::env::current_dir().unwrap_or_else(|_| ".".into())],
        }
    }
}

/// Statistics about the loading process
///
/// This struct provides information about what was loaded and processed,
/// useful for debugging and monitoring.
#[derive(Debug, Clone, Default)]
pub struct LoadStats {
    /// Number of files processed
    pub files_processed: usize,

    /// Number of modules created
    pub modules_created: usize,

    /// Number of macros resolved
    pub macros_resolved: usize,

    /// Total processing time in milliseconds
    pub processing_time_ms: u64,

    /// Memory usage in bytes (approximate)
    pub memory_usage_bytes: usize,
}

impl LoadStats {
    /// Creates a new empty statistics object
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds statistics from another LoadStats object
    pub fn merge(&mut self, other: &LoadStats) {
        self.files_processed += other.files_processed;
        self.modules_created += other.modules_created;
        self.macros_resolved += other.macros_resolved;
        self.processing_time_ms += other.processing_time_ms;
        self.memory_usage_bytes += other.memory_usage_bytes;
    }
}

/// Utility functions for working with BarkML files and paths
pub mod utils {
    use super::*;
    use std::ffi::OsStr;

    /// Checks if a path represents a BarkML file
    pub fn is_barkml_file<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref()
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.eq_ignore_ascii_case("bml"))
            .unwrap_or(false)
    }

    /// Checks if a path represents a BarkML directory (ends with .d)
    pub fn is_barkml_dir<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref()
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.eq_ignore_ascii_case("d"))
            .unwrap_or(false)
    }

    /// Extracts the basename (filename without extension) from a path
    pub fn basename<P: AsRef<Path>>(path: P) -> Result<String> {
        let path = path.as_ref();
        let file_name = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| error::Error::Basename)?;

        // Handle the case where there might not be an extension
        if let Some(extension) = path.extension().and_then(OsStr::to_str) {
            let suffix = format!(".{}", extension);
            Ok(file_name
                .strip_suffix(&suffix)
                .unwrap_or(file_name)
                .to_string())
        } else {
            Ok(file_name.to_string())
        }
    }

    /// Discovers BarkML files in a directory
    pub fn discover_files<P: AsRef<Path>>(dir: P) -> Result<Vec<std::path::PathBuf>> {
        let dir = dir.as_ref();
        let mut files = Vec::new();

        if !dir.exists() {
            return Ok(files);
        }

        let entries = std::fs::read_dir(dir).map_err(|e| error::Error::Io {
            reason: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| error::Error::Io {
                reason: e.to_string(),
            })?;
            let path = entry.path();

            if path.is_file() && is_barkml_file(&path) {
                files.push(path);
            }
        }

        files.sort();
        Ok(files)
    }

    /// Validates that a path is safe to read from
    pub fn validate_path<P: AsRef<Path>>(path: P) -> Result<()> {
        let path = path.as_ref();

        // Check if path exists
        if !path.exists() {
            return Err(error::Error::NotFound {
                path: path.to_path_buf(),
            });
        }

        // Check for path traversal attempts
        if path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(error::Error::Io {
                reason: "Path traversal not allowed".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::utils::*;
    use super::*;

    #[test]
    fn test_is_barkml_file() {
        assert!(is_barkml_file("test.bml"));
        assert!(is_barkml_file("test.BML"));
        assert!(!is_barkml_file("test.txt"));
        assert!(!is_barkml_file("test"));
    }

    #[test]
    fn test_is_barkml_dir() {
        assert!(is_barkml_dir("test.d"));
        assert!(is_barkml_dir("test.D"));
        assert!(!is_barkml_dir("test.txt"));
        assert!(!is_barkml_dir("test"));
    }

    #[test]
    fn test_basename() {
        assert_eq!(basename("test.bml").unwrap(), "test");
        assert_eq!(basename("path/to/test.bml").unwrap(), "test");
        assert_eq!(basename("test").unwrap(), "test");
    }

    #[test]
    fn test_loader_config_default() {
        let config = LoaderConfig::default();
        assert!(config.resolve_macros);
        assert!(!config.allow_collisions);
        assert_eq!(config.max_recursion_depth, 100);
        assert!(!config.validate_on_load);
    }

    #[test]
    fn test_load_stats() {
        let mut stats1 = LoadStats::new();
        stats1.files_processed = 5;
        stats1.modules_created = 3;

        let mut stats2 = LoadStats::new();
        stats2.files_processed = 2;
        stats2.modules_created = 1;

        stats1.merge(&stats2);

        assert_eq!(stats1.files_processed, 7);
        assert_eq!(stats1.modules_created, 4);
    }
}
