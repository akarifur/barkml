//! Error types and handling for BarkML parsing and processing.
//!
//! This module defines the comprehensive error types used throughout the BarkML library.
//! All errors implement the `snafu::Snafu` trait for rich error context and formatting.

// Standard library imports
use std::{
    num::{ParseFloatError, ParseIntError},
    path::PathBuf,
};

// External crate imports
use snafu::Snafu;

// Local crate imports
use crate::{
    Token,
    ast::{Location, ValueType},
};

/// Comprehensive error type for all BarkML operations.
///
/// This enum covers all possible error conditions that can occur during
/// BarkML parsing, loading, and processing. Each variant includes contextual
/// information to help with debugging and error reporting.
#[derive(Debug, Snafu, Clone, Default, PartialEq)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display(
        "{location} - type error: cannot assign a value of type '{right}' to a field with type '{left}'"
    ))]
    Assign {
        location: Location,
        left: ValueType,
        right: ValueType,
    },
    #[snafu(display("failed to resolve basename of path"))]
    Basename,
    #[snafu(display("{location} - invalid base64 encoding: {source}"))]
    Base64 {
        location: Location,
        source: base64::DecodeError,
    },
    #[snafu(display(
        "{duplicate} - duplicate declaration of '{}'{label_suffix}: already declared at {original}",
        id,
        label_suffix = if labels.is_empty() { String::new() } else { format!(" {}", labels.join(" ")) }
    ))]
    DuplicateDeclaration {
        id: String,
        /// Ordered decoded string labels; empty for assignments and table keys
        labels: Vec<String>,
        original: Location,
        duplicate: Location,
    },
    #[snafu(display(
        "name collision between {left_id} ({left_location}) and {right_id} ({right_location})"
    ))]
    Collision {
        left_id: String,
        left_location: Location,
        right_id: String,
        right_location: Location,
    },
    #[snafu(transparent)]
    Deserialize { source: crate::de::error::Error },
    #[snafu(display("{location} - unexpected end of file"))]
    Eof { location: Location },
    #[snafu(display(
        "{location} - interpolation type error: values of type '{kind}' cannot be interpolated into a string"
    ))]
    InterpolationType { location: Location, kind: ValueType },
    #[snafu(display("{location} - invalid escape sequence in string literal: '{escape}'"))]
    InvalidEscape { location: Location, escape: String },
    #[snafu(display("{location} - malformed interpolation placeholder: {reason}"))]
    Placeholder { location: Location, reason: String },
    #[snafu(display(
        "{eof} - syntax error: missing '{expected}' to close {context} opened at {open}"
    ))]
    Unterminated {
        open: Location,
        expected: String,
        eof: Location,
        context: String,
    },
    #[snafu(display("{location} - syntax error: expected {expected}, found {got}\n{context}"))]
    Expected {
        location: Location,
        expected: String,
        got: Token,
        context: String,
    },
    #[snafu(display("{location} - invalid floating point number: {source}"))]
    Float {
        location: Location,
        source: ParseFloatError,
    },
    #[snafu(display("type error: implicit conversion from '{left}' to '{right}' is not allowed"))]
    ImplicitConvert { left: ValueType, right: ValueType },
    #[snafu(display("{location} - invalid integer: {source}"))]
    Integer {
        location: Location,
        source: ParseIntError,
    },
    #[snafu(display("i/o error occurred during loading: {reason}"))]
    Io { reason: String },
    #[snafu(display(
        "{location} - legacy macro syntax '{form}' was removed; use a root-relative reference instead, e.g. 'vars.editor'"
    ))]
    LegacyMacro { location: Location, form: String },
    #[snafu(display("{location} - array index out of bounds: no element at index {index}"))]
    NoElement { location: Location, index: usize },
    #[snafu(display("{location} - field not found: '{field}'"))]
    NoField { location: Location, field: String },
    #[snafu(display("{location} - field '{field}' is not a value"))]
    NoValue { location: Location, field: String },
    #[snafu(display(
        "{location} - reference resolution failed: could not locate value at path '{path}'{}",
        if available.is_empty() { String::new() } else { format!("; available paths:\n{}", available.join("\n")) }
    ))]
    UnknownReference {
        location: Location,
        path: String,
        available: Vec<String>,
    },
    #[snafu(display("{location} - invalid selector in reference '{path}': {reason}"))]
    WrongSelector {
        location: Location,
        path: String,
        reason: String,
    },
    #[snafu(display(
        "missing main module: the standard loader requires at least one main module to load"
    ))]
    NoMain,
    #[snafu(display("file not found: '{}'", path.display()))]
    NotFound { path: PathBuf },
    #[snafu(display("{location} - not a scope with fields"))]
    NotScope { location: Location },
    #[snafu(display("{location} - recursion limit exceeded: maximum depth of {limit} reached"))]
    RecursionLimit { location: Location, limit: usize },
    #[snafu(display("{location} - invalid semantic version requirement: {reason}"))]
    Require { location: Location, reason: String },
    #[snafu(display("{location} - infinite loop detected during reference resolution"))]
    Loop { location: Location },
    #[snafu(display("module not found: could not find file named {name}.bml or directory named {name}.d in any of these paths:\n{}", search_paths.iter().map(|x| x.to_string_lossy().to_string()).collect::<Vec<_>>().join("\n")))]
    Search {
        name: String,
        search_paths: Vec<PathBuf>,
    },
    #[snafu(transparent)]
    Serialize { source: crate::ser::error::Error },
    #[snafu(display("{location} - unterminated string literal"))]
    UnterminatedString { location: Location },
    #[snafu(display("unknown error occurred"))]
    #[default]
    Unknown,
    #[snafu(display("{location} - invalid semantic version: {reason}"))]
    Version { location: Location, reason: String },
}
