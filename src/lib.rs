//! BarkML - A declarative configuration language
//!
//! BarkML is a declarative configuration format inspired by TOML, HCL, and other configuration languages.
//! It was created initially to be used with operational tools and generative tooling. The language
//! defaults to UTF-8 parsing and supports self-referential macro replacements.
//!
//! # Features
//!
//! - Declarative configuration syntax
//! - Self-referential macro replacements
//! - UTF-8 support by default
//! - Type-safe value handling
//! - Comprehensive error reporting
//!
//! # Examples
//!
//! ```rust
//! use barkml::from_str;
//!
//! let config = r#"
//! versioning = "1.0.0"
//! database {
//! host = "localhost"
//! port = 5432
//! }
//! "#;
//!
//! let statement = from_str(config).expect("Failed to parse BarkML");
//! ```

#![allow(clippy::approx_constant)]
#![allow(clippy::from_str_radix_10)]
#![allow(clippy::result_large_err)]

// Standard library imports
use std::io::Cursor;

// Local crate modules
mod ast;
mod error;
mod load;
mod syn;

// Serde deserialization support
pub mod de;

// Serde serialization support
pub mod ser;

// Re-exports
pub use ast::*;
pub use error::Error;
pub use load::*;
pub use syn::*;

/// Result type alias for BarkML operations
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// Parses a BarkML string and returns the root statement.
///
/// This is the primary entry point for parsing BarkML content from a string.
/// The function creates a temporary cursor over the input bytes and uses the
/// standard loader to parse the content.
///
/// # Arguments
///
/// * `input` - A string slice containing the BarkML content to parse
///
/// # Returns
///
/// Returns a `Result<Statement>` containing the parsed root statement on success,
/// or an error if parsing fails.
///
/// # Examples
///
/// ```rust
/// use barkml::from_str;
///
/// let config = r#"
/// versioning = "1.0.0"
/// database {
/// host = "localhost"
/// port = 5432
/// }
/// "#;
///
/// let statement = from_str(config).expect("Failed to parse BarkML");
/// ```
///
/// # Errors
///
/// This function will return an error if:
/// - The input contains invalid BarkML syntax
/// - There are type mismatches in the configuration
/// - Macro resolution fails
/// - The parser encounters unexpected tokens
pub fn from_str(input: &str) -> Result<Statement> {
    let mut cursor = Cursor::new(input.as_bytes());
    StandardLoader::default()
        .add_module("main", &mut cursor, None)?
        .load()
}
