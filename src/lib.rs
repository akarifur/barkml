//! BarkML - A declarative configuration language
//!
//! BarkML is a declarative configuration format inspired by TOML, HCL, and other configuration languages.
//! It was created initially to be used with operational tools and generative tooling. The language
//! defaults to UTF-8 parsing and supports self-referential value references.
//!
//! # Features
//!
//! - Declarative configuration syntax
//! - Root-relative reference expressions (`vars.editor`, `app["a.b"].enabled`)
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
//!
//! # Embedding BarkML in an application
//!
//! BarkML is a reusable configuration language, not an application framework.
//! The grammar only knows generic blocks, labeled identities, assignments,
//! tables, arrays, and primitive values. Names like `app`, `file`, `profile`,
//! `source`, `override`, or `stead` are ordinary identifiers: parsing a document
//! that uses them selects no profile, merges no override, and runs no provider.
//! All application semantics — what those names mean, how profiles and
//! overrides compose, schema validation — belong to the embedding consumer.
//!
//! ## Parse unresolved, compose yourself, then resolve
//!
//! A consumer can parse input without resolving references, inspect the tree
//! (labels and source locations are preserved on every node), perform its own
//! selection and composition, supply explicit context such as host facts, and
//! only then request generic reference resolution:
//!
//! ```rust
//! use std::io::Cursor;
//! use barkml::{Loader, Scope, StandardLoader};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let user_config = r#"
//! app "firefox" {
//!     enabled = vars.enable_firefox
//!     channel = >=120.0.0
//! }
//! "#;
//!
//! let host_facts = r#"
//! vars {
//!     enable_firefox = true
//! }
//! "#;
//!
//! // 1. Parse unresolved: the tree still contains reference expressions.
//! let mut loader = StandardLoader::default();
//! loader.skip_macro_resolution()?;
//!
//! // 2. Explicit context: the consumer supplies host facts as ordinary
//! //    assignments. Nothing is discovered from the machine implicitly.
//! loader.add_module("main", &mut Cursor::new(host_facts.as_bytes()), None)?;
//! loader.add_module("main", &mut Cursor::new(user_config.as_bytes()), None)?;
//!
//! // 3. Inspect before resolving: labels and locations are available for
//! //    downstream lowering and diagnostics.
//! let module = loader.read()?;
//! for (id, labels, block) in module.blocks() {
//!     let _ = (id, labels, &block.meta.location);
//! }
//!
//! // 4. Generic reference resolution, rooted at the merged document.
//! let mut scope = Scope::new(&module);
//! let resolved = scope.apply()?;
//!
//! let app = resolved
//!     .get_child("app", &["firefox"])
//!     .expect("labeled block is addressable by identity");
//! let enabled = app
//!     .find_child("enabled")
//!     .expect("assignment inside app block")
//!     .get_value()
//!     .expect("assignment carries a value");
//! assert_eq!(enabled.as_bool(), Some(&true));
//! # Ok(())
//! # }
//! ```
//!
//! ## Scope of the embedding contract
//!
//! - Parsing and generic resolution never discover the OS, fetch secrets,
//!   invoke commands, or write application configuration. The only I/O
//!   BarkML performs is the explicit filesystem loading you request through
//!   [`StandardLoader`].
//! - There is no network import keyword, no built-in provider vocabulary, and
//!   no language-level profile/override engine. Module merges happen only when
//!   you explicitly call loading APIs such as `add_module`; they are a generic
//!   mechanism, not downstream profile policy.
//! - Numeric suffixes, native SemVer literals, and requirement literals are
//!   generic language features available to any consumer.
//! - Error categories stay distinct: syntax errors (lexing/parsing), type
//!   errors (assignment compatibility), and reference errors (unresolvable or
//!   cyclic references) come from BarkML; rejecting an unknown block name or
//!   an invalid resource shape is downstream schema validation and belongs to
//!   the consumer.

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
