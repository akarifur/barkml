use super::types::{Metadata, ValueType};
use crate::error;
use base64::Engine;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use snafu::OptionExt;
use std::fmt;
use uuid::Uuid;

/// Stores the actual in-memory data for a value in BarkML
///
/// This enum represents all possible data types that can be stored in a BarkML value.
/// Each variant corresponds to a specific data type in the language.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Data {
    /// String value ('text')
    String(String),

    /// Generic signed integer
    Signed(i64),
    /// 8-bit signed integer
    I8(i8),
    /// 16-bit signed integer
    I16(i16),
    /// 32-bit signed integer
    I32(i32),
    /// 64-bit signed integer
    I64(i64),
    /// 128-bit signed integer
    I128(i128),

    /// Generic unsigned integer
    Unsigned(u64),
    /// 8-bit unsigned integer
    U8(u8),
    /// 16-bit unsigned integer
    U16(u16),
    /// 32-bit unsigned integer
    U32(u32),
    /// 64-bit unsigned integer
    U64(u64),
    /// 128-bit unsigned integer
    U128(u128),

    /// Generic floating point number
    Float(f64),
    /// 32-bit floating point number
    F32(f32),
    /// 64-bit floating point number
    F64(f64),

    /// Binary data (b'base64encoded')
    Bytes(Vec<u8>),
    /// Boolean value (true/false)
    Bool(bool),
    /// Semantic version (1.2.3)
    Version(semver::Version),
    /// Version requirement (^1.2.3, ~2.0)
    Require(semver::VersionReq),
    /// Reference expression (`vars.editor`, `app["a.b"].enabled`)
    Reference(Vec<super::scope::Segment>),
    /// Symbol identifier (:symbol)
    Symbol(String),
    /// Null value
    Null,
    /// Array of values
    Array(Vec<Value>),
    /// Table (key-value mapping)
    Table(IndexMap<String, Value>),
}

impl Data {
    /// Returns the ValueType corresponding to this Data variant
    pub fn type_of(&self) -> ValueType {
        match self {
            Data::String(_) => ValueType::String,
            Data::Signed(_) => ValueType::Signed,
            Data::I8(_) => ValueType::I8,
            Data::I16(_) => ValueType::I16,
            Data::I32(_) => ValueType::I32,
            Data::I64(_) => ValueType::I64,
            Data::I128(_) => ValueType::I128,
            Data::Unsigned(_) => ValueType::Unsigned,
            Data::U8(_) => ValueType::U8,
            Data::U16(_) => ValueType::U16,
            Data::U32(_) => ValueType::U32,
            Data::U64(_) => ValueType::U64,
            Data::U128(_) => ValueType::U128,
            Data::Float(_) => ValueType::Float,
            Data::F32(_) => ValueType::F32,
            Data::F64(_) => ValueType::F64,
            Data::Bytes(_) => ValueType::Bytes,
            Data::Bool(_) => ValueType::Bool,
            Data::Version(_) => ValueType::Version,
            Data::Require(_) => ValueType::Require,
            Data::Reference(_) => ValueType::Reference,
            Data::Symbol(_) => ValueType::Symbol,
            Data::Null => ValueType::Null,
            Data::Array(values) => ValueType::Array(values.iter().map(|x| x.type_of()).collect()),
            Data::Table(values) => ValueType::Table(
                values
                    .iter()
                    .map(|(k, v)| (k.clone(), v.type_of()))
                    .collect(),
            ),
        }
    }

    /// Returns true if this data represents a numeric value
    pub const fn is_numeric(&self) -> bool {
        matches!(
            self,
            Data::Signed(_)
                | Data::I8(_)
                | Data::I16(_)
                | Data::I32(_)
                | Data::I64(_)
                | Data::I128(_)
                | Data::Unsigned(_)
                | Data::U8(_)
                | Data::U16(_)
                | Data::U32(_)
                | Data::U64(_)
                | Data::U128(_)
                | Data::Float(_)
                | Data::F32(_)
                | Data::F64(_)
        )
    }

    /// Returns true if this data represents a collection (array or table)
    pub const fn is_collection(&self) -> bool {
        matches!(self, Data::Array(_) | Data::Table(_))
    }

    /// Returns the approximate memory size of this data in bytes
    pub fn memory_size(&self) -> usize {
        match self {
            Data::String(s) => s.capacity() + std::mem::size_of::<String>(),
            Data::Bytes(b) => b.capacity() + std::mem::size_of::<Vec<u8>>(),
            Data::Array(arr) => {
                arr.iter().map(|v| v.memory_size()).sum::<usize>()
                    + arr.capacity() * std::mem::size_of::<Value>()
            }
            Data::Table(table) => {
                table
                    .iter()
                    .map(|(k, v)| k.capacity() + v.memory_size())
                    .sum::<usize>()
                    + table.capacity()
                        * (std::mem::size_of::<String>() + std::mem::size_of::<Value>())
            }
            Data::Symbol(s) => s.capacity() + std::mem::size_of::<String>(),
            Data::Reference(segments) => {
                segments.iter().map(std::mem::size_of_val).sum::<usize>()
                    + std::mem::size_of::<Vec<super::scope::Segment>>()
            }
            _ => std::mem::size_of_val(self),
        }
    }
}

/// Represents an individual value in the BarkML language
///
/// A Value is the fundamental unit of data in BarkML. It contains the actual data,
/// a unique identifier, and metadata about the value's source location and annotations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Value {
    /// Unique identifier for the value, used for reference tracking and macro resolution
    pub uid: Uuid,

    /// The actual data contained in this value
    pub data: Data,

    /// Metadata including source location, comments, and labels
    pub meta: Metadata,
}

impl Value {
    /// Creates a new Value with the given data and metadata
    pub fn new(data: Data, meta: Metadata) -> Self {
        Self {
            uid: Uuid::now_v7(),
            data,
            meta,
        }
    }

    /// Creates a deep clone of this value with a new UUID
    pub fn deep_clone(&self) -> Self {
        Self {
            uid: Uuid::now_v7(),
            data: self.data.clone(),
            meta: self.meta.clone(),
        }
    }

    /// Returns the type of this value
    pub fn type_of(&self) -> ValueType {
        self.data.type_of()
    }

    /// Returns true if this value is null
    pub const fn is_null(&self) -> bool {
        matches!(self.data, Data::Null)
    }

    /// Returns true if this value is numeric
    pub const fn is_numeric(&self) -> bool {
        self.data.is_numeric()
    }

    /// Returns true if this value is a collection
    pub const fn is_collection(&self) -> bool {
        self.data.is_collection()
    }

    /// Returns the approximate memory size of this value
    pub fn memory_size(&self) -> usize {
        self.data.memory_size() + std::mem::size_of::<Uuid>() + std::mem::size_of::<Metadata>()
    }

    /// Converts this value to an interpolation-friendly string representation
    pub fn to_macro_string(&self) -> String {
        match &self.data {
            Data::Reference(segments) => super::scope::Segment::display_path(segments),
            Data::Symbol(value) | Data::String(value) => value.clone(),
            Data::Array(array) => array
                .iter()
                .map(|x| x.to_macro_string())
                .collect::<Vec<_>>()
                .join(","),
            Data::Table(children) => children
                .iter()
                .map(|(k, v)| format!("{}:{}", k, v.to_macro_string()))
                .collect::<Vec<_>>()
                .join(","),
            Data::Unsigned(value) => value.to_string(),
            Data::U8(value) => value.to_string(),
            Data::U16(value) => value.to_string(),
            Data::U32(value) => value.to_string(),
            Data::U64(value) => value.to_string(),
            Data::U128(value) => value.to_string(),
            Data::Signed(value) => value.to_string(),
            Data::I8(value) => value.to_string(),
            Data::I16(value) => value.to_string(),
            Data::I32(value) => value.to_string(),
            Data::I64(value) => value.to_string(),
            Data::I128(value) => value.to_string(),
            Data::Float(value) => value.to_string(),
            Data::F32(value) => value.to_string(),
            Data::F64(value) => value.to_string(),
            Data::Bool(value) => if *value { "true" } else { "false" }.to_string(),
            Data::Bytes(value) => {
                base64::engine::general_purpose::STANDARD.encode(value.as_slice())
            }
            Data::Null => "null".to_string(),
            Data::Version(value) => value.to_string(),
            Data::Require(value) => value.to_string(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

// Macro to generate constructor and accessor methods for each data type
macro_rules! value_methods {
    ($new_fn:ident, $as_fn:ident, $as_mut_fn:ident, $data_type:ty, $variant:ident) => {
        impl Value {
            pub fn $new_fn(data: $data_type, meta: Metadata) -> Self {
                Self::new(Data::$variant(data.into()), meta)
            }

            pub fn $as_fn(&self) -> Option<&$data_type> {
                match &self.data {
                    Data::$variant(value) => Some(value),
                    _ => None,
                }
            }

            pub fn $as_mut_fn(&mut self) -> Option<&mut $data_type> {
                match &mut self.data {
                    Data::$variant(value) => Some(value),
                    _ => None,
                }
            }
        }
    };
}

// Generate methods for all data types
value_methods!(new_string, as_string, as_string_mut, String, String);
value_methods!(new_int, as_int, as_int_mut, i64, Signed);
value_methods!(new_i8, as_i8, as_i8_mut, i8, I8);
value_methods!(new_i16, as_i16, as_i16_mut, i16, I16);
value_methods!(new_i32, as_i32, as_i32_mut, i32, I32);
value_methods!(new_i64, as_i64, as_i64_mut, i64, I64);
value_methods!(new_i128, as_i128, as_i128_mut, i128, I128);
value_methods!(new_uint, as_uint, as_uint_mut, u64, Unsigned);
value_methods!(new_u8, as_u8, as_u8_mut, u8, U8);
value_methods!(new_u16, as_u16, as_u16_mut, u16, U16);
value_methods!(new_u32, as_u32, as_u32_mut, u32, U32);
value_methods!(new_u64, as_u64, as_u64_mut, u64, U64);
value_methods!(new_u128, as_u128, as_u128_mut, u128, U128);
value_methods!(new_float, as_float, as_float_mut, f64, Float);
value_methods!(new_f32, as_f32, as_f32_mut, f32, F32);
value_methods!(new_f64, as_f64, as_f64_mut, f64, F64);
value_methods!(new_bytes, as_bytes, as_bytes_mut, Vec<u8>, Bytes);
value_methods!(new_bool, as_bool, as_bool_mut, bool, Bool);
value_methods!(
    new_version,
    as_version,
    as_version_mut,
    semver::Version,
    Version
);
value_methods!(
    new_require,
    as_require,
    as_require_mut,
    semver::VersionReq,
    Require
);
value_methods!(
    new_reference,
    as_reference,
    as_reference_mut,
    Vec<super::scope::Segment>,
    Reference
);
value_methods!(new_symbol, as_symbol, as_symbol_mut, String, Symbol);
value_methods!(new_array, as_array, as_array_mut, Vec<Value>, Array);
value_methods!(new_table, as_table, as_table_mut, IndexMap<String, Value>, Table);

impl Value {
    pub fn new_null(meta: Metadata) -> Self {
        Self::new(Data::Null, meta)
    }
}

// TryFrom implementations for type conversion
macro_rules! try_from_value {
    ($target_type:ty, $accessor:ident, $value_type:expr) => {
        impl<'a> TryFrom<&'a Value> for $target_type {
            type Error = error::Error;

            fn try_from(value: &'a Value) -> Result<$target_type, Self::Error> {
                value
                    .$accessor()
                    .context(error::ImplicitConvertSnafu {
                        left: $value_type,
                        right: value.type_of(),
                    })
                    .cloned()
            }
        }
    };
}

// Generate TryFrom implementations
try_from_value!(i8, as_i8, ValueType::I8);
try_from_value!(i16, as_i16, ValueType::I16);
try_from_value!(i32, as_i32, ValueType::I32);
try_from_value!(i128, as_i128, ValueType::I128);
try_from_value!(u8, as_u8, ValueType::U8);
try_from_value!(u16, as_u16, ValueType::U16);
try_from_value!(u32, as_u32, ValueType::U32);
try_from_value!(u128, as_u128, ValueType::U128);
try_from_value!(f32, as_f32, ValueType::F32);
try_from_value!(Vec<u8>, as_bytes, ValueType::Bytes);
try_from_value!(bool, as_bool, ValueType::Bool);
try_from_value!(semver::Version, as_version, ValueType::Version);
try_from_value!(semver::VersionReq, as_require, ValueType::Require);

// Special TryFrom implementations with fallback logic
impl<'a> TryFrom<&'a Value> for String {
    type Error = error::Error;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value.type_of() {
            ValueType::String => Ok(value.as_string().unwrap().clone()),
            ValueType::Symbol => Ok(value.as_symbol().unwrap().clone()),
            vtype => error::ImplicitConvertSnafu {
                left: ValueType::String,
                right: vtype,
            }
            .fail(),
        }
    }
}

impl<'a> TryFrom<&'a Value> for i64 {
    type Error = error::Error;

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        value
            .as_i64()
            .or(value.as_int())
            .context(error::ImplicitConvertSnafu {
                left: ValueType::I64,
                right: value.type_of(),
            })
            .cloned()
    }
}

impl<'a> TryFrom<&'a Value> for u64 {
    type Error = error::Error;

    fn try_from(value: &'a Value) -> Result<u64, Self::Error> {
        value
            .as_u64()
            .or(value.as_uint())
            .context(error::ImplicitConvertSnafu {
                left: ValueType::U64,
                right: value.type_of(),
            })
            .cloned()
    }
}

impl<'a> TryFrom<&'a Value> for f64 {
    type Error = error::Error;

    fn try_from(value: &'a Value) -> Result<f64, Self::Error> {
        value
            .as_f64()
            .or(value.as_float())
            .context(error::ImplicitConvertSnafu {
                left: ValueType::F64,
                right: value.type_of(),
            })
            .cloned()
    }
}

impl<'a> TryFrom<&'a Value> for Vec<Value> {
    type Error = error::Error;

    fn try_from(value: &'a Value) -> Result<Vec<Value>, Self::Error> {
        let mut result = Vec::new();
        for value in value.as_array().context(error::ImplicitConvertSnafu {
            left: ValueType::Array(Vec::new()),
            right: value.type_of(),
        })? {
            result.push(value.clone());
        }
        Ok(result)
    }
}

impl<'a> TryFrom<&'a Value> for IndexMap<String, Value> {
    type Error = error::Error;

    fn try_from(value: &'a Value) -> Result<IndexMap<String, Value>, Self::Error> {
        let mut result = IndexMap::new();
        for (key, value) in value.as_table().context(error::ImplicitConvertSnafu {
            left: ValueType::Table(IndexMap::new()),
            right: value.type_of(),
        })? {
            result.insert(key.clone(), value.clone());
        }
        Ok(result)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write comment if present
        if let Some(comment) = self.meta.comment.as_ref() {
            write!(f, "/* {} */ ", comment)?;
        }

        // Write label if present
        if let Some(label) = self.meta.label.as_ref() {
            write!(f, "!{} ", label)?;
        }

        // Write the actual value
        match &self.data {
            Data::String(value) => write!(f, "'{}'", value),
            Data::Signed(value) => write!(f, "{}", value),
            Data::I8(value) => write!(f, "{}i8", value),
            Data::I16(value) => write!(f, "{}i16", value),
            Data::I32(value) => write!(f, "{}i32", value),
            Data::I64(value) => write!(f, "{}i64", value),
            Data::I128(value) => write!(f, "{}i128", value),
            Data::Unsigned(value) => write!(f, "{}", value),
            Data::U8(value) => write!(f, "{}u8", value),
            Data::U16(value) => write!(f, "{}u16", value),
            Data::U32(value) => write!(f, "{}u32", value),
            Data::U64(value) => write!(f, "{}u64", value),
            Data::U128(value) => write!(f, "{}u128", value),
            Data::Float(value) => write!(f, "{}", value),
            Data::F32(value) => write!(f, "{}f32", value),
            Data::F64(value) => write!(f, "{}f64", value),
            Data::Bool(value) => write!(f, "{}", if *value { "true" } else { "false" }),
            Data::Bytes(value) => write!(
                f,
                "b'{}'",
                base64::engine::general_purpose::STANDARD.encode(value.as_slice())
            ),
            Data::Reference(segments) => {
                write!(f, "{}", super::scope::Segment::display_path(segments))
            }
            Data::Symbol(value) => write!(f, ":{}", value),
            Data::Null => write!(f, "null"),
            Data::Version(value) => write!(f, "{}", value),
            Data::Require(value) => write!(f, "{}", value),
            Data::Array(values) => {
                write!(
                    f,
                    "[{}]",
                    values
                        .iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Data::Table(values) => {
                write!(
                    f,
                    "{{\n{}\n}}",
                    values
                        .iter()
                        .map(|(k, v)| format!("  '{}' = {}", k, v))
                        .collect::<Vec<_>>()
                        .join(",\n")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::Location;

    #[test]
    fn test_value_creation() {
        let meta = Metadata::new(Location::new(0, 0));
        let value = Value::new_string("test".to_string(), meta);

        assert_eq!(value.as_string(), Some(&"test".to_string()));
        assert_eq!(value.type_of(), ValueType::String);
        assert!(!value.is_null());
    }

    #[test]
    fn test_value_deep_clone() {
        let meta = Metadata::new(Location::new(0, 0));
        let original = Value::new_int(42, meta);
        let cloned = original.deep_clone();

        assert_eq!(original.data, cloned.data);
        assert_ne!(original.uid, cloned.uid);
    }

    #[test]
    fn test_try_from_conversions() {
        let meta = Metadata::new(Location::new(0, 0));
        let value = Value::new_string("test".to_string(), meta);

        let converted: Result<String, _> = (&value).try_into();
        assert!(converted.is_ok());
        assert_eq!(converted.unwrap(), "test");
    }
}
