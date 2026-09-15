use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Defines the type of a given value
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ValueType {
    /// String values
    String,

    /// Non-precise signed integer (defaults to 64-bit)
    Signed,
    /// 8-bit signed integer
    I8,
    /// 16-bit signed integer
    I16,
    /// 32-bit signed integer
    I32,
    /// 64-bit signed integer
    I64,
    /// 128-bit signed integer
    I128,

    /// Non-precise unsigned integer (defaults to 64-bit)
    Unsigned,
    /// 8-bit unsigned integer
    U8,
    /// 16-bit unsigned integer
    U16,
    /// 32-bit unsigned integer
    U32,
    /// 64-bit unsigned integer
    U64,
    /// 128-bit unsigned integer
    U128,

    /// Non-precise floating point (defaults to 64-bit)
    Float,
    /// 32-bit floating point
    F32,
    /// 64-bit floating point
    F64,

    /// Byte Array (Vec<u8>)
    Bytes,

    /// Boolean
    Bool,

    /// Semantic Version
    Version,
    /// Semantic Version Requirement
    Require,

    /// Reference expression
    Reference,

    /// Label identifier
    Label,

    /// :Symbol identifier
    Symbol,

    /// Null
    Null,

    /// Array
    Array(Vec<Self>),

    /// Table
    Table(IndexMap<String, Self>),
}

impl ValueType {
    /// Checks if the right type can be assigned to the left when a type requirement
    /// is not specified.
    ///
    /// This function implements the type compatibility rules for BarkML, determining
    /// whether a value of one type can be assigned to a variable of another type.
    ///
    /// # Arguments
    ///
    /// * `right` - The type of the value being assigned
    ///
    /// # Returns
    ///
    /// `true` if the assignment is valid, `false` otherwise
    pub fn can_assign(&self, right: &Self) -> bool {
        use ValueType::*;

        match (self, right) {
            // Exact matches are always valid
            (left, right) if left == right => true,

            // String types are only compatible with other strings
            (String, String) => true,

            // Signed integer compatibility
            (Signed, I8 | I16 | I32 | I64 | I128 | Signed) => true,
            (Signed, U8 | U16 | U32) => true, // Small unsigned can fit in signed
            (I64, Signed | I8 | I16 | I32 | I64) => true,
            (I64, U8 | U16 | U32) => true, // Small unsigned can fit in i64

            // Unsigned integer compatibility
            (Unsigned, U8 | U16 | U32 | U64 | U128 | Unsigned) => true,
            (U64, Unsigned | U8 | U16 | U32 | U64) => true,

            // Float compatibility
            (Float, F32 | F64 | Float) => true,
            (F64, F64 | Float) => true,

            // No other implicit conversions allowed
            _ => false,
        }
    }

    /// Determines if this type is a numeric type
    pub const fn is_numeric(&self) -> bool {
        matches!(
            self,
            Self::Signed
                | Self::Unsigned
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::I128
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::U128
                | Self::Float
                | Self::F32
                | Self::F64
        )
    }

    /// Determines if this type is an integer type
    pub const fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::Signed
                | Self::Unsigned
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::I128
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::U128
        )
    }

    /// Determines if this type is a floating point type
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float | Self::F32 | Self::F64)
    }

    /// Determines if this type is a compound type (array or table)
    pub const fn is_compound(&self) -> bool {
        matches!(self, Self::Array(_) | Self::Table(_))
    }

    /// Determines if this type is a primitive type (not compound)
    pub const fn is_primitive(&self) -> bool {
        !self.is_compound()
    }

    /// Returns the size in bytes for fixed-size types, None for variable-size types
    pub const fn size_hint(&self) -> Option<usize> {
        match self {
            Self::I8 | Self::U8 => Some(1),
            Self::I16 | Self::U16 => Some(2),
            Self::I32 | Self::U32 | Self::F32 => Some(4),
            Self::I64 | Self::U64 | Self::F64 | Self::Signed | Self::Unsigned | Self::Float => {
                Some(8)
            }
            Self::I128 | Self::U128 => Some(16),
            Self::Bool => Some(1),
            _ => None, // Variable size types
        }
    }

    /// Returns the category of this type for grouping purposes
    pub const fn category(&self) -> TypeCategory {
        match self {
            Self::String => TypeCategory::Text,
            Self::Signed | Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 => {
                TypeCategory::SignedInteger
            }
            Self::Unsigned | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::U128 => {
                TypeCategory::UnsignedInteger
            }
            Self::Float | Self::F32 | Self::F64 => TypeCategory::Float,
            Self::Bytes => TypeCategory::Binary,
            Self::Bool => TypeCategory::Boolean,
            Self::Version | Self::Require => TypeCategory::Version,
            Self::Reference => TypeCategory::Reference,
            Self::Label | Self::Symbol => TypeCategory::Identifier,
            Self::Null => TypeCategory::Null,
            Self::Array(_) => TypeCategory::Collection,
            Self::Table(_) => TypeCategory::Collection,
        }
    }
}

/// Categories for grouping related types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategory {
    Text,
    SignedInteger,
    UnsignedInteger,
    Float,
    Binary,
    Boolean,
    Version,
    Reference,
    Identifier,
    Null,
    Collection,
}

impl fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String => f.write_str("string"),
            Self::Signed => f.write_str("int"),
            Self::I8 => f.write_str("i8"),
            Self::I16 => f.write_str("i16"),
            Self::I32 => f.write_str("i32"),
            Self::I64 => f.write_str("i64"),
            Self::I128 => f.write_str("i128"),
            Self::Unsigned => f.write_str("uint"),
            Self::U8 => f.write_str("u8"),
            Self::U16 => f.write_str("u16"),
            Self::U32 => f.write_str("u32"),
            Self::U64 => f.write_str("u64"),
            Self::U128 => f.write_str("u128"),
            Self::Float => f.write_str("float"),
            Self::F32 => f.write_str("f32"),
            Self::F64 => f.write_str("f64"),
            Self::Bytes => f.write_str("bytes"),
            Self::Bool => f.write_str("bool"),
            Self::Version => f.write_str("version"),
            Self::Require => f.write_str("require"),
            Self::Reference => f.write_str("reference"),
            Self::Label => f.write_str("label"),
            Self::Symbol => f.write_str("symbol"),
            Self::Null => f.write_str("null"),
            Self::Array(children) => {
                if children.is_empty() {
                    f.write_str("[]")
                } else {
                    write!(
                        f,
                        "[{}]",
                        children
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Self::Table(children) => {
                if children.is_empty() {
                    f.write_str("{}")
                } else {
                    write!(
                        f,
                        "{{ {} }}",
                        children
                            .iter()
                            .map(|(k, v)| format!("{k}: {v}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        }
    }
}

/// Represents the type of a statement in the BarkML language
#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub enum StatementType {
    /// Control statement ($identifier = value)
    /// Stores the expected type of the value
    Control(ValueType),

    /// Assignment statement (identifier = value)
    /// Stores the expected type of the value
    Assignment(ValueType),

    /// Block statement (identifier labels { statements })
    /// Stores the types of labels and contents
    Block {
        /// Types of the label values
        labels: Vec<ValueType>,
        /// Types of the contained statements
        contents: IndexMap<String, Self>,
    },

    /// Module statement (top-level container)
    /// Stores the types of contained statements
    Module(IndexMap<String, Self>),
}

impl StatementType {
    /// Returns true if this statement type can contain child statements
    pub const fn is_container(&self) -> bool {
        matches!(self, Self::Block { .. } | Self::Module(_))
    }

    /// Returns true if this statement type represents a value assignment
    pub const fn is_assignment(&self) -> bool {
        matches!(self, Self::Control(_) | Self::Assignment(_))
    }

    /// Gets the value type for assignment statements, None for containers
    pub fn value_type(&self) -> Option<&ValueType> {
        match self {
            Self::Control(vt) | Self::Assignment(vt) => Some(vt),
            _ => None,
        }
    }

    /// Gets the child statement types for container statements
    pub fn child_types(&self) -> Option<&IndexMap<String, Self>> {
        match self {
            Self::Block { contents, .. } | Self::Module(contents) => Some(contents),
            _ => None,
        }
    }
}

/// Stores the metadata associated with a value or statement
///
/// Metadata provides additional information about AST nodes, including source location,
/// comments, and labels. This information is useful for error reporting, documentation
/// generation, and preserving the original structure of the code.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct Metadata {
    /// Source location information
    pub location: Location,

    /// Optional comment associated with the node (from # or /* */ comments)
    pub comment: Option<String>,

    /// Optional label associated with the node (from !label syntax)
    pub label: Option<String>,
}

impl Metadata {
    /// Creates a new Metadata instance with the given location
    pub const fn new(location: Location) -> Self {
        Self {
            location,
            comment: None,
            label: None,
        }
    }

    /// Creates a new Metadata instance with location, comment, and label
    pub const fn with_details(
        location: Location,
        comment: Option<String>,
        label: Option<String>,
    ) -> Self {
        Self {
            location,
            comment,
            label,
        }
    }

    /// Returns true if this metadata has a comment
    pub const fn has_comment(&self) -> bool {
        self.comment.is_some()
    }

    /// Returns true if this metadata has a label
    pub const fn has_label(&self) -> bool {
        self.label.is_some()
    }

    /// Returns true if this metadata has any additional information beyond location
    pub const fn has_annotations(&self) -> bool {
        self.has_comment() || self.has_label()
    }
}

/// Represents a location in the source code
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Location {
    /// Module name where the token was found
    pub module: Option<String>,
    /// Line number (0-based)
    pub line: usize,
    /// Column number (0-based)
    pub column: usize,
    /// Original source text for the token
    pub source_text: Option<String>,
    /// Length of the token in characters
    pub length: usize,
    /// File path where the token was found
    pub file_path: Option<String>,
}

impl Location {
    /// Create a new location with basic information
    pub const fn new(line: usize, column: usize) -> Self {
        Self {
            module: None,
            line,
            column,
            source_text: None,
            length: 0,
            file_path: None,
        }
    }

    /// Create a new location with full information
    pub const fn with_details(
        module: Option<String>,
        line: usize,
        column: usize,
        source_text: Option<String>,
        length: usize,
        file_path: Option<String>,
    ) -> Self {
        Self {
            module,
            line,
            column,
            source_text,
            length,
            file_path,
        }
    }

    /// Set the module name
    pub fn set_module(&mut self, module: &str) {
        self.module = Some(module.to_string());
    }

    /// Set the source text
    pub fn set_source_text(&mut self, text: &str) {
        self.source_text = Some(text.to_string());
        self.length = text.len();
    }

    /// Set the file path
    pub fn set_file_path(&mut self, path: &str) {
        self.file_path = Some(path.to_string());
    }

    /// Get the human-readable position (1-based for display)
    pub const fn position(&self) -> (usize, usize) {
        (self.line + 1, self.column + 1)
    }

    /// Get the source context for error reporting
    ///
    /// Returns the source text if available, or an empty string if not.
    /// This is useful for providing context in error messages.
    pub fn context(&self) -> String {
        self.source_text.clone().unwrap_or_default()
    }

    /// Get a formatted source context with line numbers for error reporting
    ///
    /// Returns a formatted string with line numbers and the source text,
    /// highlighting the current line if possible.
    pub fn formatted_context(&self) -> String {
        let Some(source) = &self.source_text else {
            return String::new();
        };

        let lines: Vec<&str> = source.lines().collect();
        let line_num = self.line;

        // Determine the range of lines to show (up to 2 lines before and after)
        let start_line = line_num.saturating_sub(2);
        let end_line = std::cmp::min(line_num + 3, lines.len());

        let mut result = String::new();
        for (offset, line_content) in lines
            .iter()
            .enumerate()
            .skip(start_line)
            .take(end_line - start_line)
        {
            let i = offset;
            let line_display = i + 1; // 1-based line numbers for display
            if i == line_num {
                // Highlight the current line
                result.push_str(&format!("-> {}: {}\n", line_display, line_content));

                // Add caret pointing to the column
                let spaces = 3 + line_display.to_string().len() + 2 + self.column;
                result.push_str(&format!("{}^\n", " ".repeat(spaces)));
            } else {
                result.push_str(&format!("   {}: {}\n", line_display, line_content));
            }
        }

        result
    }

    /// Returns true if this location has complete information
    pub fn is_complete(&self) -> bool {
        self.module.is_some() && self.file_path.is_some() && self.source_text.is_some()
    }

    /// Create a new location that spans from this location to another
    pub fn span_to(&self, other: &Location) -> Location {
        let start_line = std::cmp::min(self.line, other.line);
        let _end_line = std::cmp::max(self.line, other.line);
        let start_col = if self.line == other.line {
            std::cmp::min(self.column, other.column)
        } else if self.line < other.line {
            self.column
        } else {
            other.column
        };
        let length = if self.line == other.line {
            (std::cmp::max(self.column + self.length, other.column + other.length))
                .saturating_sub(start_col)
        } else {
            // Multi-line span - approximate
            self.length + other.length
        };

        Location {
            module: self.module.clone().or_else(|| other.module.clone()),
            line: start_line,
            column: start_col,
            source_text: self
                .source_text
                .clone()
                .or_else(|| other.source_text.clone()),
            length,
            file_path: self.file_path.clone().or_else(|| other.file_path.clone()),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (line, column) = self.position();

        match (&self.module, &self.file_path) {
            (Some(module), Some(file_path)) => {
                write!(f, "[{}@{}:{}:{}]", module, file_path, line, column)
            }
            (Some(module), None) => {
                write!(f, "[{}@{}:{}]", module, line, column)
            }
            (None, Some(file_path)) => {
                write!(f, "[{}:{}:{}]", file_path, line, column)
            }
            (None, None) => {
                write!(f, "[{}:{}]", line, column)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_can_assign() {
        // Test exact matches
        assert!(ValueType::String.can_assign(&ValueType::String));
        assert!(ValueType::I32.can_assign(&ValueType::I32));

        // Test signed integer compatibility
        assert!(ValueType::Signed.can_assign(&ValueType::I8));
        assert!(ValueType::Signed.can_assign(&ValueType::U8));
        assert!(ValueType::I64.can_assign(&ValueType::I32));

        // Test unsigned integer compatibility
        assert!(ValueType::Unsigned.can_assign(&ValueType::U8));
        assert!(ValueType::U64.can_assign(&ValueType::U32));

        // Test float compatibility
        assert!(ValueType::Float.can_assign(&ValueType::F32));
        assert!(ValueType::F64.can_assign(&ValueType::Float));

        // Test incompatible types
        assert!(!ValueType::String.can_assign(&ValueType::I32));
        assert!(!ValueType::I32.can_assign(&ValueType::String));
        assert!(!ValueType::U64.can_assign(&ValueType::I64));
    }

    #[test]
    fn test_value_type_categories() {
        assert!(ValueType::I32.is_numeric());
        assert!(ValueType::F64.is_numeric());
        assert!(!ValueType::String.is_numeric());

        assert!(ValueType::I32.is_integer());
        assert!(!ValueType::F64.is_integer());

        assert!(ValueType::F64.is_float());
        assert!(!ValueType::I32.is_float());

        assert!(ValueType::Array(vec![]).is_compound());
        assert!(!ValueType::String.is_compound());
    }

    #[test]
    fn test_location_span() {
        let loc1 = Location::new(0, 5);
        let loc2 = Location::new(0, 10);
        let span = loc1.span_to(&loc2);

        assert_eq!(span.line, 0);
        assert_eq!(span.column, 5);
    }
}
