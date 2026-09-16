//! Serde serialization support for BarkML AST nodes.
//!
//! This module provides comprehensive serde serialization support for converting Rust data
//! structures into BarkML `Statement` and `Value` types. This enables programmatic generation
//! of BarkML AST nodes from Rust structs and other data types.
//!
//! The module consists of:
//! - `error`: Custom error types for serialization failures
//! - `statement`: Serializer implementation that produces `Statement` types
//! - `value`: Serializer implementation that produces `Value` types

// External crates
use serde::Serialize;

// Local crate
use crate::{Location, Metadata, Result, Statement, Value};

pub(crate) mod error;
mod statement;
mod value;

pub use statement::*;
pub use value::*;

/// Serialize a type `T` to a BarkML `Statement`.
///
/// This is the main entry point for serializing Rust data structures to BarkML statements.
/// It supports serializing structs, maps, and other complex data types to statement groups.
///
/// # Errors
///
/// Returns an error if the value cannot be serialized to a BarkML statement or if
/// any nested values fail to serialize.
pub fn to_statement<T>(value: &T) -> Result<Statement>
where
    T: Serialize,
{
    let meta = Metadata::new(Location::new(0, 0));
    let mut serializer = StatementSerializer::new(meta);
    let result = value.serialize(&mut serializer)?;
    Ok(result)
}

/// Serialize a type `T` to a BarkML `Value`.
///
/// This function allows serializing Rust data structures to individual BarkML values.
/// It's useful when you want to convert a specific value to a BarkML representation.
///
/// # Errors
///
/// Returns an error if the value type cannot be represented as a BarkML value or if
/// the serialization process fails.
pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: Serialize,
{
    let meta = Metadata::new(Location::new(0, 0));
    let mut serializer = ValueSerializer::new(meta);
    let result = value.serialize(&mut serializer)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    // External crates
    use indexmap::IndexMap;
    use serde::{Deserialize, Serialize};

    // Parent module
    use super::*;

    // Local crate
    use crate::de::from_value;

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    #[allow(dead_code)] // fields exercised via (de)serialization, not by name
    struct TestConfig {
        version: String,
        debug: bool,
        port: u16,
        timeout: f64,
    }

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct DatabaseConfig {
        host: String,
        port: u16,
        ssl: bool,
    }

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct AppConfig {
        app_name: String,
        database: DatabaseConfig,
        features: Vec<String>,
    }

    #[test]
    fn serialize_simple_values_works_correctly() {
        // Arrange & Act - Test string
        let value = to_value(&"test".to_string()).expect("should serialize string");

        // Assert
        if let crate::Data::String(s) = &value.data {
            assert_eq!(s, "test");
        } else {
            panic!("Expected string value");
        }

        // Arrange & Act - Test integer
        let value = to_value(&42i32).expect("should serialize integer");

        // Assert
        if let crate::Data::I32(n) = &value.data {
            assert_eq!(*n, 42);
        } else {
            panic!("Expected i32 value");
        }

        // Arrange & Act - Test boolean
        let value = to_value(&true).expect("should serialize boolean");

        // Assert
        if let crate::Data::Bool(b) = &value.data {
            assert!(*b);
        } else {
            panic!("Expected boolean value");
        }

        // Arrange & Act - Test float
        let value = to_value(&3.14f64).expect("should serialize float");

        // Assert
        if let crate::Data::F64(f) = &value.data {
            assert_eq!(*f, 3.14);
        } else {
            panic!("Expected f64 value");
        }
    }

    #[test]
    fn serialize_array_works_correctly() {
        // Arrange
        let array = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];

        // Act
        let value = to_value(&array).expect("should serialize array");

        // Assert - Check that we can deserialize it back
        let deserialized: Vec<String> = from_value(&value).expect("should deserialize back");
        assert_eq!(deserialized, array);
    }

    #[test]
    fn serialize_map_works_correctly() {
        // Arrange
        let mut map = IndexMap::new();
        map.insert("name".to_string(), "Alice".to_string());
        map.insert("age".to_string(), "30".to_string());

        // Act
        let value = to_value(&map).expect("should serialize map");

        // Assert - Check that we can deserialize it back
        let deserialized: IndexMap<String, String> =
            from_value(&value).expect("should deserialize back");
        assert_eq!(deserialized, map);
    }

    #[test]
    fn roundtrip_serialization_works_correctly() {
        // Arrange
        let original = AppConfig {
            app_name: "TestApp".to_string(),
            database: DatabaseConfig {
                host: "db.example.com".to_string(),
                port: 3306,
                ssl: false,
            },
            features: vec!["feature1".to_string(), "feature2".to_string()],
        };

        // Act - Serialize to value, then deserialize back
        let value = to_value(&original).expect("should serialize");
        let roundtrip: AppConfig = from_value(&value).expect("should deserialize");

        // Assert
        assert_eq!(roundtrip, original);
    }
}
