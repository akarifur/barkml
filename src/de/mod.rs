//! Serde deserialization support for BarkML AST nodes.
//!
//! This module provides comprehensive serde deserialization support for BarkML `Statement`
//! and `Value` types, enabling direct deserialization of Rust data structures from parsed
//! BarkML AST nodes.
//!
//! The module consists of:
//! - `error`: Custom error types for deserialization failures
//! - `statement`: Deserializer implementation for `Statement` types
//! - `value`: Deserializer implementation for `Value` types
//!
//! # Block labels and the child-map projection
//!
//! `from_statement` projects blocks as maps of their children; block labels are **not**
//! exposed as fields in the deserialized value, and this projection does not reconstruct
//! resource identity. Zero-label blocks appear under their id; labeled blocks appear under
//! their storage slot key (a uid string), so same-kind siblings with different labels do
//! not overwrite each other but cannot be located by label through this projection alone.
//! Consumers that need a block's `(id, ordered labels)` identity should traverse the AST
//! directly via [`crate::Statement::blocks`] / [`crate::Statement::identity`], or use the
//! structured lookups [`crate::Statement::get_child`] / [`crate::load::Walk::walk_block`].
//! Derived serde on `Statement`/`StatementData` (the AST round-trip path) retains the
//! `Labeled` data, including labels.

// External crates
use serde::de::Deserialize;

// Local crate
use crate::{Result, Statement, Value};

pub(crate) mod error;
mod statement;
mod value;

pub use statement::*;
pub use value::*;

/// Deserialize a type `T` from a BarkML `Statement`.
///
/// This is the main entry point for deserializing Rust data structures from BarkML statements.
/// It supports deserializing from module, block, and assignment statements.
///
/// # Errors
///
/// Returns an error if the statement structure doesn't match the expected type `T` or if
/// any values cannot be converted to the target types.
pub fn from_statement<T>(statement: &Statement) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let deserializer = StatementDeserializer::new(statement);
    let result = T::deserialize(deserializer)?;
    Ok(result)
}

/// Deserialize a type `T` from a BarkML `Value`.
///
/// This function allows deserializing Rust data structures from individual BarkML values.
/// It's useful when you have a specific value and want to convert it to a Rust type.
///
/// # Errors
///
/// Returns an error if the value type doesn't match the expected type `T` or if the
/// value cannot be converted to the target type.
pub fn from_value<T>(value: &Value) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let deserializer = ValueDeserializer::new(value);
    let result = T::deserialize(deserializer)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    // External crates
    use indexmap::IndexMap;
    use serde::Deserialize;

    // Parent module
    use super::*;

    // Local crate
    use crate::{Location, Metadata, from_str};

    #[derive(Debug, PartialEq, Deserialize)]
    struct TestConfig {
        #[serde(rename = "versioning")]
        version: String,
        debug: bool,
        port: u16,
        timeout: f64,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct DatabaseConfig {
        host: String,
        port: u16,
        ssl: bool,
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct AppConfig {
        app_name: String,
        database: DatabaseConfig,
        features: Vec<String>,
    }

    #[test]
    fn deserialize_simple_values_works_correctly() {
        // Arrange
        let meta = Metadata::new(Location::new(0, 0));

        // Act & Assert - Test string
        let value = Value::new_string("test".to_string(), meta.clone());
        let result: String = from_value(&value).expect("should deserialize string");
        assert_eq!(result, "test");

        // Act & Assert - Test integer
        let value = Value::new_int(42, meta.clone());
        let result: i32 = from_value(&value).expect("should deserialize integer");
        assert_eq!(result, 42);

        // Act & Assert - Test boolean
        let value = Value::new_bool(true, meta.clone());
        let result: bool = from_value(&value).expect("should deserialize boolean");
        assert!(result);

        // Act & Assert - Test float
        let value = Value::new_float(3.14, meta);
        let result: f64 = from_value(&value).expect("should deserialize float");
        assert_eq!(result, 3.14);
    }

    #[test]
    fn deserialize_from_statement_works_correctly() {
        // Arrange
        let barkml = r#"
        versioning = "1.0.0"
        debug = true
        port = 8080u16
        timeout = 30.5
        "#;

        // Act
        let statement = from_str(barkml).expect("should parse BarkML");
        let config: TestConfig = from_statement(&statement).expect("should deserialize config");

        // Assert
        assert_eq!(config.version, "1.0.0");
        assert!(config.debug);
        assert_eq!(config.port, 8080);
        assert_eq!(config.timeout, 30.5);
    }

    #[test]
    fn deserialize_nested_structure_works_correctly() {
        // Arrange
        let barkml = r#"
        app_name = "MyApp"
        features = ["auth", "logging", "metrics"]

        database {
        host = "localhost"
        port = 5432u16
        ssl = true
        }
        "#;

        // Act
        let statement = from_str(barkml).expect("should parse BarkML");
        let config: AppConfig = from_statement(&statement).expect("should deserialize config");

        // Assert
        assert_eq!(config.app_name, "MyApp");
        assert_eq!(config.features, vec!["auth", "logging", "metrics"]);
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
        assert!(config.database.ssl);
    }

    #[test]
    fn deserialize_array_works_correctly() {
        // Arrange
        let meta = Metadata::new(Location::new(0, 0));
        let array_data = vec![
            Value::new_string("first".to_string(), meta.clone()),
            Value::new_string("second".to_string(), meta.clone()),
            Value::new_string("third".to_string(), meta),
        ];
        let value = Value::new_array(array_data, Metadata::new(Location::new(0, 0)));

        // Act
        let result: Vec<String> = from_value(&value).expect("should deserialize array");

        // Assert
        assert_eq!(result, vec!["first", "second", "third"]);
    }

    #[test]
    fn deserialize_table_works_correctly() {
        // Arrange
        let meta = Metadata::new(Location::new(0, 0));
        let mut table_data = IndexMap::new();
        table_data.insert(
            "name".to_string(),
            Value::new_string("Alice".to_string(), meta.clone()),
        );
        table_data.insert("age".to_string(), Value::new_int(30, meta.clone()));

        let value = Value::new_table(table_data, meta);

        #[derive(Debug, PartialEq, Deserialize)]
        struct Person {
            name: String,
            age: i32,
        }

        // Act
        let result: Person = from_value(&value).expect("should deserialize table");

        // Assert
        assert_eq!(result.name, "Alice");
        assert_eq!(result.age, 30);
    }

    #[test]
    fn error_handling_works_correctly() {
        // Arrange
        let meta = Metadata::new(Location::new(0, 0));
        let value = Value::new_string("not_a_number".to_string(), meta);

        // Act
        let result: std::result::Result<i32, _> = from_value(&value);

        // Assert
        assert!(result.is_err());
    }
}
