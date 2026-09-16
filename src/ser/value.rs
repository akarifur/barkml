//! Value serializer implementation.

// External crates
use indexmap::IndexMap;
use serde::ser::{
    self, Serialize, SerializeMap as SerdeSerializeMap, SerializeSeq, SerializeStruct,
    SerializeStructVariant as SerdeSerializeStructVariant, SerializeTuple, SerializeTupleStruct,
    SerializeTupleVariant as SerdeSerializeTupleVariant, Serializer,
};

// Parent module
use super::error::{Error, Result};

// Local crate
use crate::{Metadata, Value};

/// Serializer for BarkML values.
pub struct ValueSerializer {
    metadata: Metadata,
}

impl ValueSerializer {
    /// Create a new value serializer.
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }
}

impl Serializer for &mut ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SerializeVec;
    type SerializeTuple = SerializeVec;
    type SerializeTupleStruct = SerializeVec;
    type SerializeTupleVariant = ValueSerializeTupleVariant;
    type SerializeMap = ValueSerializeMap;
    type SerializeStruct = ValueSerializeMap;
    type SerializeStructVariant = ValueSerializeStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Value> {
        Ok(Value::new_bool(v, self.metadata.clone()))
    }

    fn serialize_i8(self, v: i8) -> Result<Value> {
        Ok(Value::new_i8(v, self.metadata.clone()))
    }

    fn serialize_i16(self, v: i16) -> Result<Value> {
        Ok(Value::new_i16(v, self.metadata.clone()))
    }

    fn serialize_i32(self, v: i32) -> Result<Value> {
        Ok(Value::new_i32(v, self.metadata.clone()))
    }

    fn serialize_i64(self, v: i64) -> Result<Value> {
        Ok(Value::new_i64(v, self.metadata.clone()))
    }

    fn serialize_u8(self, v: u8) -> Result<Value> {
        Ok(Value::new_u8(v, self.metadata.clone()))
    }

    fn serialize_u16(self, v: u16) -> Result<Value> {
        Ok(Value::new_u16(v, self.metadata.clone()))
    }

    fn serialize_u32(self, v: u32) -> Result<Value> {
        Ok(Value::new_u32(v, self.metadata.clone()))
    }

    fn serialize_u64(self, v: u64) -> Result<Value> {
        Ok(Value::new_u64(v, self.metadata.clone()))
    }

    fn serialize_f32(self, v: f32) -> Result<Value> {
        Ok(Value::new_f32(v, self.metadata.clone()))
    }

    fn serialize_f64(self, v: f64) -> Result<Value> {
        Ok(Value::new_f64(v, self.metadata.clone()))
    }

    fn serialize_char(self, v: char) -> Result<Value> {
        Ok(Value::new_string(v.to_string(), self.metadata.clone()))
    }

    fn serialize_str(self, v: &str) -> Result<Value> {
        Ok(Value::new_string(v.to_string(), self.metadata.clone()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Value> {
        Ok(Value::new_bytes(v.to_vec(), self.metadata.clone()))
    }

    fn serialize_none(self) -> Result<Value> {
        Ok(Value::new_null(self.metadata.clone()))
    }

    fn serialize_some<T>(self, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value> {
        Ok(Value::new_null(self.metadata.clone()))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Value> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Value> {
        Ok(Value::new_string(
            variant.to_string(),
            self.metadata.clone(),
        ))
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value>
    where
        T: ?Sized + Serialize,
    {
        let mut table = IndexMap::new();
        let serialized_value = {
            let mut serializer = ValueSerializer::new(self.metadata.clone());
            value.serialize(&mut serializer)?
        };
        table.insert(variant.to_string(), serialized_value);
        Ok(Value::new_table(table, self.metadata.clone()))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeVec {
            vec: Vec::with_capacity(len.unwrap_or(0)),
            metadata: self.metadata.clone(),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(ValueSerializeTupleVariant {
            name: variant.to_string(),
            vec: Vec::with_capacity(len),
            metadata: self.metadata.clone(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(ValueSerializeMap {
            map: IndexMap::new(),
            next_key: None,
            metadata: self.metadata.clone(),
        })
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(ValueSerializeStructVariant {
            name: variant.to_string(),
            map: IndexMap::new(),
            metadata: self.metadata.clone(),
        })
    }
}

/// Serializer for sequences (arrays, tuples).
pub struct SerializeVec {
    vec: Vec<Value>,
    metadata: Metadata,
}

impl SerializeSeq for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = ValueSerializer::new(self.metadata.clone());
        let serialized = value.serialize(&mut serializer)?;
        self.vec.push(serialized);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::new_array(self.vec, self.metadata))
    }
}

impl SerializeTuple for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for SerializeVec {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value> {
        SerializeSeq::end(self)
    }
}

/// Serializer for tuple variants.
pub struct ValueSerializeTupleVariant {
    name: String,
    vec: Vec<Value>,
    metadata: Metadata,
}

impl SerdeSerializeTupleVariant for ValueSerializeTupleVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = ValueSerializer::new(self.metadata.clone());
        let serialized = value.serialize(&mut serializer)?;
        self.vec.push(serialized);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut table = IndexMap::new();
        let array_value = Value::new_array(self.vec, self.metadata.clone());
        table.insert(self.name, array_value);
        Ok(Value::new_table(table, self.metadata))
    }
}

/// Serializer for maps and structs.
pub struct ValueSerializeMap {
    map: IndexMap<String, Value>,
    next_key: Option<String>,
    metadata: Metadata,
}

impl SerdeSerializeMap for ValueSerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut key_serializer = KeySerializer;
        let key_str = key.serialize(&mut key_serializer)?;
        self.next_key = Some(key_str);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let key = self.next_key.take().ok_or_else(|| Error::Message {
            message: "serialize_value called without serialize_key".to_string(),
        })?;

        let mut serializer = ValueSerializer::new(self.metadata.clone());
        let serialized = value.serialize(&mut serializer)?;
        self.map.insert(key, serialized);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        Ok(Value::new_table(self.map, self.metadata))
    }
}

impl SerializeStruct for ValueSerializeMap {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = ValueSerializer::new(self.metadata.clone());
        let serialized = value.serialize(&mut serializer)?;
        self.map.insert(key.to_string(), serialized);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        SerdeSerializeMap::end(self)
    }
}

/// Serializer for struct variants.
pub struct ValueSerializeStructVariant {
    name: String,
    map: IndexMap<String, Value>,
    metadata: Metadata,
}

impl SerdeSerializeStructVariant for ValueSerializeStructVariant {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = ValueSerializer::new(self.metadata.clone());
        let serialized = value.serialize(&mut serializer)?;
        self.map.insert(key.to_string(), serialized);
        Ok(())
    }

    fn end(self) -> Result<Value> {
        let mut outer_table = IndexMap::new();
        let inner_table = Value::new_table(self.map, self.metadata.clone());
        outer_table.insert(self.name, inner_table);
        Ok(Value::new_table(outer_table, self.metadata))
    }
}

/// Key serializer that only accepts string keys.
pub struct KeySerializer;

impl Serializer for &mut KeySerializer {
    type Ok = String;
    type Error = Error;

    type SerializeSeq = ser::Impossible<String, Error>;
    type SerializeTuple = ser::Impossible<String, Error>;
    type SerializeTupleStruct = ser::Impossible<String, Error>;
    type SerializeTupleVariant = ser::Impossible<String, Error>;
    type SerializeMap = ser::Impossible<String, Error>;
    type SerializeStruct = ser::Impossible<String, Error>;
    type SerializeStructVariant = ser::Impossible<String, Error>;

    fn serialize_bool(self, _v: bool) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_i8(self, _v: i8) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_i16(self, _v: i16) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_i32(self, _v: i32) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_i64(self, _v: i64) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_u8(self, _v: u8) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_u16(self, _v: u16) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_u32(self, _v: u32) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_u64(self, _v: u64) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_f32(self, _v: f32) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_f64(self, _v: f64) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_char(self, v: char) -> Result<String> {
        Ok(v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<String> {
        Ok(v.to_string())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_none(self) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_some<T>(self, _value: &T) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_unit(self) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<String> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<String> {
        Ok(variant.to_string())
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::MapKeyMustBeString)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::MapKeyMustBeString)
    }
}
