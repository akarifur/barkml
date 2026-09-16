//! Statement serializer implementation.

// External crates
use indexmap::IndexMap;
use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

// Parent module
use super::{
    ValueSerializer,
    error::{Error, Result},
};

// Local crate
use crate::{Metadata, Statement, Value};

/// Serializer for BarkML statements.
pub struct StatementSerializer {
    metadata: Metadata,
}

impl StatementSerializer {
    /// Create a new statement serializer.
    pub fn new(metadata: Metadata) -> Self {
        Self { metadata }
    }
}

impl Serializer for &mut StatementSerializer {
    type Ok = Statement;
    type Error = Error;

    type SerializeSeq = SerializeStatementSeq;
    type SerializeTuple = SerializeStatementSeq;
    type SerializeTupleStruct = SerializeStatementSeq;
    type SerializeTupleVariant = SerializeStatementTupleVariant;
    type SerializeMap = SerializeStatementMap;
    type SerializeStruct = SerializeStatementMap;
    type SerializeStructVariant = SerializeStatementStructVariant;

    fn serialize_bool(self, v: bool) -> Result<Statement> {
        let value = Value::new_bool(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_i8(self, v: i8) -> Result<Statement> {
        let value = Value::new_i8(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_i16(self, v: i16) -> Result<Statement> {
        let value = Value::new_i16(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_i32(self, v: i32) -> Result<Statement> {
        let value = Value::new_i32(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_i64(self, v: i64) -> Result<Statement> {
        let value = Value::new_i64(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_u8(self, v: u8) -> Result<Statement> {
        let value = Value::new_u8(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_u16(self, v: u16) -> Result<Statement> {
        let value = Value::new_u16(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_u32(self, v: u32) -> Result<Statement> {
        let value = Value::new_u32(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_u64(self, v: u64) -> Result<Statement> {
        let value = Value::new_u64(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_f32(self, v: f32) -> Result<Statement> {
        let value = Value::new_f32(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_f64(self, v: f64) -> Result<Statement> {
        let value = Value::new_f64(v, self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_char(self, v: char) -> Result<Statement> {
        let value = Value::new_string(v.to_string(), self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_str(self, v: &str) -> Result<Statement> {
        let value = Value::new_string(v.to_string(), self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Statement> {
        let value = Value::new_bytes(v.to_vec(), self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_none(self) -> Result<Statement> {
        let value = Value::new_null(self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_some<T>(self, value: &T) -> Result<Statement>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Statement> {
        let value = Value::new_null(self.metadata.clone().clone());
        Statement::new_assign("value", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Statement> {
        let value = Value::new_null(self.metadata.clone().clone());
        Statement::new_assign(name, None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Statement> {
        let value = Value::new_string(variant.to_string(), self.metadata.clone().clone());
        Statement::new_assign("variant", None, value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Statement>
    where
        T: ?Sized + Serialize,
    {
        let mut value_serializer = ValueSerializer::new(self.metadata.clone().clone());
        let serialized_value = value.serialize(&mut value_serializer)?;
        Statement::new_assign(name, None, serialized_value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Statement>
    where
        T: ?Sized + Serialize,
    {
        let mut value_serializer = ValueSerializer::new(self.metadata.clone().clone());
        let serialized_value = value.serialize(&mut value_serializer)?;
        Statement::new_assign(variant, None, serialized_value, self.metadata.clone()).map_err(|e| {
            Error::Message {
                message: e.to_string(),
            }
        })
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SerializeStatementSeq {
            statements: IndexMap::new(),
            index: 0,
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
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SerializeStatementTupleVariant {
            name: variant.to_string(),
            statements: IndexMap::new(),
            index: 0,
            metadata: self.metadata.clone(),
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(SerializeStatementMap {
            statements: IndexMap::new(),
            next_key: None,
            metadata: self.metadata.clone(),
        })
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(SerializeStatementMap {
            statements: IndexMap::new(),
            next_key: None,
            metadata: self.metadata.clone(),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(SerializeStatementStructVariant {
            name: variant.to_string(),
            statements: IndexMap::new(),
            metadata: self.metadata.clone(),
        })
    }
}

/// Serializer for statement sequences.
pub struct SerializeStatementSeq {
    statements: IndexMap<String, Statement>,
    index: usize,
    metadata: Metadata,
}

impl SerializeSeq for SerializeStatementSeq {
    type Ok = Statement;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = StatementSerializer::new(self.metadata.clone().clone());
        let statement = value.serialize(&mut serializer)?;
        self.statements
            .insert(format!("item_{}", self.index), statement);
        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result<Statement> {
        Ok(Statement::new_module(
            "sequence",
            self.statements,
            self.metadata.clone(),
        ))
    }
}

impl SerializeTuple for SerializeStatementSeq {
    type Ok = Statement;
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Statement> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for SerializeStatementSeq {
    type Ok = Statement;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Statement> {
        SerializeSeq::end(self)
    }
}

/// Serializer for statement tuple variants.
pub struct SerializeStatementTupleVariant {
    name: String,
    statements: IndexMap<String, Statement>,
    index: usize,
    metadata: Metadata,
}

impl SerializeTupleVariant for SerializeStatementTupleVariant {
    type Ok = Statement;
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = StatementSerializer::new(self.metadata.clone().clone());
        let statement = value.serialize(&mut serializer)?;
        self.statements
            .insert(format!("item_{}", self.index), statement);
        self.index += 1;
        Ok(())
    }

    fn end(self) -> Result<Statement> {
        Ok(Statement::new_module(
            &self.name,
            self.statements,
            self.metadata.clone(),
        ))
    }
}

/// Serializer for statement maps and structs.
pub struct SerializeStatementMap {
    statements: IndexMap<String, Statement>,
    next_key: Option<String>,
    metadata: Metadata,
}

impl SerializeMap for SerializeStatementMap {
    type Ok = Statement;
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut key_serializer = super::value::KeySerializer;
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

        let mut value_serializer = ValueSerializer::new(self.metadata.clone().clone());
        let serialized_value = value.serialize(&mut value_serializer)?;
        let statement =
            Statement::new_assign(&key, None, serialized_value, self.metadata.clone().clone())
                .map_err(|e| Error::Message {
                    message: e.to_string(),
                })?;
        self.statements.insert(key, statement);
        Ok(())
    }

    fn end(self) -> Result<Statement> {
        Ok(Statement::new_module(
            "root",
            self.statements,
            self.metadata.clone(),
        ))
    }
}

impl SerializeStruct for SerializeStatementMap {
    type Ok = Statement;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut value_serializer = ValueSerializer::new(self.metadata.clone().clone());
        let serialized_value = value.serialize(&mut value_serializer)?;
        let statement =
            Statement::new_assign(key, None, serialized_value, self.metadata.clone().clone())
                .map_err(|e| Error::Message {
                    message: e.to_string(),
                })?;
        self.statements.insert(key.to_string(), statement);
        Ok(())
    }

    fn end(self) -> Result<Statement> {
        SerializeMap::end(self)
    }
}

/// Serializer for statement struct variants.
pub struct SerializeStatementStructVariant {
    name: String,
    statements: IndexMap<String, Statement>,
    metadata: Metadata,
}

impl SerializeStructVariant for SerializeStatementStructVariant {
    type Ok = Statement;
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        let mut value_serializer = ValueSerializer::new(self.metadata.clone().clone());
        let serialized_value = value.serialize(&mut value_serializer)?;
        let statement =
            Statement::new_assign(key, None, serialized_value, self.metadata.clone().clone())
                .map_err(|e| Error::Message {
                    message: e.to_string(),
                })?;
        self.statements.insert(key.to_string(), statement);
        Ok(())
    }

    fn end(self) -> Result<Statement> {
        Ok(Statement::new_module(
            &self.name,
            self.statements,
            self.metadata.clone(),
        ))
    }
}
