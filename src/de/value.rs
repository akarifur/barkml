//! Value deserializer implementation.

// External crates
use serde::Deserializer;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};

// Parent module
use super::error::{self, Result};

// Local crate
use crate::{Data, Value};

/// Deserializer for BarkML values.
pub struct ValueDeserializer<'a> {
    value: &'a Value,
}

impl<'a> ValueDeserializer<'a> {
    /// Create a new value deserializer.
    pub fn new(value: &'a Value) -> Self {
        Self { value }
    }
}

impl<'de, 'a> Deserializer<'de> for ValueDeserializer<'a> {
    type Error = error::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::String(s) => visitor.visit_str(s),
            Data::Signed(n) => visitor.visit_i64(*n),
            Data::I8(n) => visitor.visit_i8(*n),
            Data::I16(n) => visitor.visit_i16(*n),
            Data::I32(n) => visitor.visit_i32(*n),
            Data::I64(n) => visitor.visit_i64(*n),
            Data::I128(n) => visitor.visit_i128(*n),
            Data::Unsigned(n) => visitor.visit_u64(*n),
            Data::U8(n) => visitor.visit_u8(*n),
            Data::U16(n) => visitor.visit_u16(*n),
            Data::U32(n) => visitor.visit_u32(*n),
            Data::U64(n) => visitor.visit_u64(*n),
            Data::U128(n) => visitor.visit_u128(*n),
            Data::Float(n) => visitor.visit_f64(*n),
            Data::F32(n) => visitor.visit_f32(*n),
            Data::F64(n) => visitor.visit_f64(*n),
            Data::Bool(b) => visitor.visit_bool(*b),
            Data::Null => visitor.visit_unit(),
            Data::Array(arr) => {
                let seq = ArrayAccess::new(arr);
                visitor.visit_seq(seq)
            }
            Data::Table(table) => {
                let map = TableAccess::new(table);
                visitor.visit_map(map)
            }
            Data::Bytes(bytes) => visitor.visit_bytes(bytes),
            Data::Version(version) => visitor.visit_str(&version.to_string()),
            Data::Require(req) => visitor.visit_str(&req.to_string()),
            Data::Reference(_) | Data::Template(_) => error::UnresolvedReferenceSnafu.fail(),
            Data::Symbol(symbol) => visitor.visit_str(symbol),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::Bool(b) => visitor.visit_bool(*b),
            _ => error::TypeMismatchSnafu {
                expected: "bool",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::I8(n) => visitor.visit_i8(*n),
            Data::Signed(n) => {
                if *n >= i64::from(i8::MIN) && *n <= i64::from(i8::MAX) {
                    visitor.visit_i8(*n as i8)
                } else {
                    error::InvalidValueSnafu {
                        value: n.to_string(),
                        expected_type: "i8",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "i8",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::I16(n) => visitor.visit_i16(*n),
            Data::I8(n) => visitor.visit_i16(i16::from(*n)),
            Data::Signed(n) => {
                if *n >= i64::from(i16::MIN) && *n <= i64::from(i16::MAX) {
                    visitor.visit_i16(*n as i16)
                } else {
                    error::InvalidValueSnafu {
                        value: &n.to_string().to_string(),
                        expected_type: "i16",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "i16",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::I32(n) => visitor.visit_i32(*n),
            Data::I8(n) => visitor.visit_i32(i32::from(*n)),
            Data::I16(n) => visitor.visit_i32(i32::from(*n)),
            Data::Signed(n) => {
                if *n >= i64::from(i32::MIN) && *n <= i64::from(i32::MAX) {
                    visitor.visit_i32(*n as i32)
                } else {
                    error::InvalidValueSnafu {
                        value: &n.to_string().to_string(),
                        expected_type: "i32",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "i32",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::I64(n) => visitor.visit_i64(*n),
            Data::I8(n) => visitor.visit_i64(i64::from(*n)),
            Data::I16(n) => visitor.visit_i64(i64::from(*n)),
            Data::I32(n) => visitor.visit_i64(i64::from(*n)),
            Data::Signed(n) => visitor.visit_i64(*n),
            _ => error::TypeMismatchSnafu {
                expected: "i64",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::I128(n) => visitor.visit_i128(*n),
            Data::I8(n) => visitor.visit_i128(i128::from(*n)),
            Data::I16(n) => visitor.visit_i128(i128::from(*n)),
            Data::I32(n) => visitor.visit_i128(i128::from(*n)),
            Data::I64(n) => visitor.visit_i128(i128::from(*n)),
            Data::Signed(n) => visitor.visit_i128(i128::from(*n)),
            _ => error::TypeMismatchSnafu {
                expected: "i128",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::U8(n) => visitor.visit_u8(*n),
            Data::Unsigned(n) => {
                if *n <= u64::from(u8::MAX) {
                    visitor.visit_u8(*n as u8)
                } else {
                    error::InvalidValueSnafu {
                        value: &n.to_string().to_string(),
                        expected_type: "u8",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "u8",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::U16(n) => visitor.visit_u16(*n),
            Data::U8(n) => visitor.visit_u16(u16::from(*n)),
            Data::Unsigned(n) => {
                if *n <= u64::from(u16::MAX) {
                    visitor.visit_u16(*n as u16)
                } else {
                    error::InvalidValueSnafu {
                        value: &n.to_string().to_string(),
                        expected_type: "u16",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "u16",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::U32(n) => visitor.visit_u32(*n),
            Data::U8(n) => visitor.visit_u32(u32::from(*n)),
            Data::U16(n) => visitor.visit_u32(u32::from(*n)),
            Data::Unsigned(n) => {
                if *n <= u64::from(u32::MAX) {
                    visitor.visit_u32(*n as u32)
                } else {
                    error::InvalidValueSnafu {
                        value: &n.to_string().to_string(),
                        expected_type: "u32",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "u32",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::U64(n) => visitor.visit_u64(*n),
            Data::U8(n) => visitor.visit_u64(u64::from(*n)),
            Data::U16(n) => visitor.visit_u64(u64::from(*n)),
            Data::U32(n) => visitor.visit_u64(u64::from(*n)),
            Data::Unsigned(n) => visitor.visit_u64(*n),
            _ => error::TypeMismatchSnafu {
                expected: "u64",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::U128(n) => visitor.visit_u128(*n),
            Data::U8(n) => visitor.visit_u128(u128::from(*n)),
            Data::U16(n) => visitor.visit_u128(u128::from(*n)),
            Data::U32(n) => visitor.visit_u128(u128::from(*n)),
            Data::U64(n) => visitor.visit_u128(u128::from(*n)),
            Data::Unsigned(n) => visitor.visit_u128(u128::from(*n)),
            _ => error::TypeMismatchSnafu {
                expected: "u128",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::F32(n) => visitor.visit_f32(*n),
            Data::Float(n) => visitor.visit_f32(*n as f32),
            _ => error::TypeMismatchSnafu {
                expected: "f32",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::F64(n) => visitor.visit_f64(*n),
            Data::F32(n) => visitor.visit_f64(f64::from(*n)),
            Data::Float(n) => visitor.visit_f64(*n),
            _ => error::TypeMismatchSnafu {
                expected: "f64",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::String(s) => {
                if s.len() == 1 {
                    visitor.visit_char(s.chars().next().expect("string has one character"))
                } else {
                    error::InvalidValueSnafu {
                        value: s.to_string(),
                        expected_type: "char",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "char",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::String(s) => visitor.visit_str(s),
            Data::Version(v) => visitor.visit_str(&v.to_string()),
            Data::Require(v) => visitor.visit_str(&v.to_string()),
            Data::Symbol(s) => visitor.visit_str(s),
            _ => error::TypeMismatchSnafu {
                expected: "string",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::Bytes(bytes) => visitor.visit_bytes(bytes),
            _ => error::TypeMismatchSnafu {
                expected: "bytes",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::Null => visitor.visit_unit(),
            _ => error::TypeMismatchSnafu {
                expected: "unit",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::Array(arr) => {
                let seq = ArrayAccess::new(arr);
                visitor.visit_seq(seq)
            }
            _ => error::TypeMismatchSnafu {
                expected: "sequence",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::Table(table) => {
                let map = TableAccess::new(table);
                visitor.visit_map(map)
            }
            _ => error::TypeMismatchSnafu {
                expected: "table",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match &self.value.data {
            Data::String(s) | Data::Symbol(s) => visitor.visit_enum(s.as_str().into_deserializer()),
            Data::Table(table) => {
                if table.len() == 1 {
                    let (key, value) = table.iter().next().expect("table has one entry");
                    visitor.visit_enum(EnumAccess::new(key, value))
                } else {
                    error::InvalidValueSnafu {
                        value: "table with multiple keys".to_string(),
                        expected_type: "enum",
                    }
                    .fail()
                }
            }
            _ => error::TypeMismatchSnafu {
                expected: "string, symbol, or macro",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// Sequence access for arrays.
struct ArrayAccess<'a> {
    iter: std::slice::Iter<'a, Value>,
}

impl<'a> ArrayAccess<'a> {
    fn new(array: &'a [Value]) -> Self {
        Self { iter: array.iter() }
    }
}

impl<'de, 'a> SeqAccess<'de> for ArrayAccess<'a> {
    type Error = error::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let deserializer = ValueDeserializer::new(value);
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None),
        }
    }
}

/// Map access for tables.
struct TableAccess<'a> {
    iter: indexmap::map::Iter<'a, String, Value>,
    value: Option<&'a Value>,
}

impl<'a> TableAccess<'a> {
    fn new(table: &'a indexmap::IndexMap<String, Value>) -> Self {
        Self {
            iter: table.iter(),
            value: None,
        }
    }
}

impl<'de, 'a> MapAccess<'de> for TableAccess<'a> {
    type Error = error::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => {
                let deserializer = ValueDeserializer::new(value);
                seed.deserialize(deserializer)
            }
            None => error::MessageSnafu {
                message: "value is missing".to_string(),
            }
            .fail(),
        }
    }
}

/// Enum access for enum deserialization.
struct EnumAccess<'a> {
    variant: &'a str,
    value: &'a Value,
}

impl<'a> EnumAccess<'a> {
    fn new(variant: &'a str, value: &'a Value) -> Self {
        Self { variant, value }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumAccess<'a> {
    type Error = error::Error;
    type Variant = ValueDeserializer<'a>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        let deserializer = ValueDeserializer::new(self.value);
        Ok((variant, deserializer))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for ValueDeserializer<'a> {
    type Error = error::Error;

    fn unit_variant(self) -> Result<()> {
        match &self.value.data {
            Data::Null => Ok(()),
            _ => error::TypeMismatchSnafu {
                expected: "unit variant",
                found: self.value.type_of().to_string(),
            }
            .fail(),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }
}
