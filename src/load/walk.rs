use super::{Result, error};
use crate::ast::{Statement, Value, ValueType};
use indexmap::IndexSet;
use snafu::OptionExt;
use std::fmt;

/// Enhanced walker for navigating and extracting data from BarkML statements and values
///
/// This enum provides a comprehensive and ergonomic way to traverse the AST and extract values
/// from BarkML documents. It handles both Statement and Value nodes with enhanced error handling,
/// type safety, and performance optimizations.
///
/// # Features
///
/// - Type-safe value extraction with automatic conversion
/// - Path-based navigation with dot notation support
/// - Comprehensive error handling with location information
/// - Performance optimizations for common operations
/// - Support for complex queries and filtering
/// - Debugging and introspection capabilities
///
#[derive(Clone)]
pub enum Walk<'source> {
    /// A reference to a Statement node in the AST
    Statement(&'source Statement),

    /// A reference to a Value node in the AST
    Value(&'source Value),
}

impl<'source> Walk<'source> {
    /// Create a new walker over the given module statement
    pub fn new(module: &'source Statement) -> Self {
        Self::Statement(module)
    }

    /// Create a new walker over a value
    pub fn from_value(value: &'source Value) -> Self {
        Self::Value(value)
    }

    /// Get the current location information for error reporting
    pub fn location(&self) -> &crate::ast::Location {
        match self {
            Self::Statement(stmt) => &stmt.meta.location,
            Self::Value(value) => &value.meta.location,
        }
    }

    /// Get the type of the current node
    pub fn node_type(&self) -> NodeType {
        match self {
            Self::Statement(stmt) => NodeType::Statement(stmt.type_.clone()),
            Self::Value(value) => NodeType::Value(value.type_of()),
        }
    }

    /// Check if the current node is a statement
    pub const fn is_statement(&self) -> bool {
        matches!(self, Self::Statement(_))
    }

    /// Check if the current node is a value
    pub const fn is_value(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Fetch a child statement by field name with enhanced error handling
    pub fn get_child(&self, field: &str) -> Result<&Statement> {
        match self {
            Self::Statement(stmt) => {
                let children = stmt.get_grouped().context(error::NotScopeSnafu {
                    location: stmt.meta.location.clone(),
                })?;

                children.get(field).context(error::NoFieldSnafu {
                    field: field.to_string(),
                    location: stmt.meta.location.clone(),
                })
            }
            Self::Value(value) => error::NotScopeSnafu {
                location: value.meta.location.clone(),
            }
            .fail(),
        }
    }

    /// Fetch and convert a value from a field with path support
    pub fn get<T>(&self, field: &str) -> Result<T>
    where
        T: TryFrom<&'source Value, Error = error::Error>,
    {
        // Support dot notation for nested paths
        if field.contains('.') {
            return self.get_by_path(field);
        }

        match self {
            Self::Statement(stmt) => {
                let children = stmt
                    .get_grouped()
                    .or_else(|| stmt.get_labeled().map(|x| x.1))
                    .context(error::NotScopeSnafu {
                        location: stmt.meta.location.clone(),
                    })?;

                let target_stmt = children.get(field).context(error::NoFieldSnafu {
                    location: stmt.meta.location.clone(),
                    field: field.to_string(),
                })?;

                let value = target_stmt.get_value().context(error::NoValueSnafu {
                    location: stmt.meta.location.clone(),
                    field: field.to_string(),
                })?;

                value.try_into()
            }
            Self::Value(value) => {
                let table = value.as_table().context(error::NotScopeSnafu {
                    location: value.meta.location.clone(),
                })?;

                let target_value = table.get(field).context(error::NoFieldSnafu {
                    location: value.meta.location.clone(),
                    field: field.to_string(),
                })?;

                target_value.try_into()
            }
        }
    }

    /// Get a value by dot-separated path (e.g., "config.database.host")
    pub fn get_by_path<T>(&self, path: &str) -> Result<T>
    where
        T: TryFrom<&'source Value, Error = error::Error>,
    {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self.clone();

        // Navigate to the target
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // Last part - extract the value
                return current.get(part);
            } else {
                // Intermediate part - navigate deeper
                current = current.walk(part)?;
            }
        }

        unreachable!("Path navigation should have returned or errored")
    }

    /// Get the identifier of the current statement
    pub fn get_id(&self) -> Option<&str> {
        match self {
            Self::Statement(stmt) => Some(&stmt.id),
            Self::Value(_) => None,
        }
    }

    /// Fetch and convert a block label to the requested type
    pub fn get_label<T>(&self, index: usize) -> Result<T>
    where
        T: TryFrom<&'source Value, Error = error::Error>,
    {
        match self {
            Self::Statement(stmt) => {
                let (labels, _) = stmt.get_labeled().context(error::NotScopeSnafu {
                    location: stmt.meta.location.clone(),
                })?;

                let label = labels.get(index).context(error::NoElementSnafu {
                    location: stmt.meta.location.clone(),
                    index,
                })?;

                label.try_into()
            }
            Self::Value(value) => error::NotScopeSnafu {
                location: value.meta.location.clone(),
            }
            .fail(),
        }
    }

    /// Get all block names in the current scope
    pub fn get_all_blocks(&self) -> Result<IndexSet<String>> {
        match self {
            Self::Statement(stmt) => {
                let children = stmt.get_grouped().context(error::NotScopeSnafu {
                    location: stmt.meta.location.clone(),
                })?;

                Ok(children
                    .iter()
                    .filter_map(|(k, s)| {
                        if matches!(s.type_, crate::StatementType::Block { .. }) {
                            Some(k.clone())
                        } else {
                            None
                        }
                    })
                    .collect())
            }
            Self::Value(value) => error::NotScopeSnafu {
                location: value.meta.location.clone(),
            }
            .fail(),
        }
    }

    /// Get all blocks with a specific ID, returning each block's ordered
    /// label sequence. Label order is preserved and component boundaries are
    /// kept intact: `app "a.b"` yields `["a.b"]` and `app "a" "b"` yields
    /// `["a", "b"]`.
    pub fn get_blocks(&self, field: &str) -> Result<Vec<Vec<String>>> {
        match self {
            Self::Statement(stmt) => {
                let children = stmt
                    .get_grouped()
                    .or_else(|| stmt.get_labeled().map(|x| x.1))
                    .context(error::NotScopeSnafu {
                        location: stmt.meta.location.clone(),
                    })?;

                Ok(children
                    .iter()
                    .filter_map(|(_, s)| {
                        let (labels, _) = s.get_labeled()?;
                        if s.id != field {
                            return None;
                        }
                        Some(
                            labels
                                .iter()
                                .map(|label| label.as_string().cloned().unwrap_or_default())
                                .collect(),
                        )
                    })
                    .collect())
            }
            Self::Value(value) => error::NotScopeSnafu {
                location: value.meta.location.clone(),
            }
            .fail(),
        }
    }

    /// Navigate to a block by structured identity: its id plus the ordered
    /// label sequence. This is the unambiguous way to address labeled
    /// blocks; `walk(field)` only addresses unlabeled children.
    pub fn walk_block(&self, id: &str, labels: &[&str]) -> Result<Self> {
        match self {
            Self::Statement(stmt) => {
                let target = stmt.get_child(id, labels).context(error::NoFieldSnafu {
                    location: stmt.meta.location.clone(),
                    field: format!(
                        "{}{}",
                        id,
                        labels
                            .iter()
                            .map(|l| format!(" {:?}", l))
                            .collect::<String>()
                    ),
                })?;
                Ok(Self::Statement(target))
            }
            Self::Value(value) => error::NotScopeSnafu {
                location: value.meta.location.clone(),
            }
            .fail(),
        }
    }

    /// Create a new walker for a nested field
    pub fn walk(&self, field: &str) -> Result<Self> {
        match self {
            Self::Statement(stmt) => {
                if let Some(children) = stmt
                    .get_grouped()
                    .or_else(|| stmt.get_labeled().map(|x| x.1))
                {
                    let target = children.get(field).context(error::NoFieldSnafu {
                        location: stmt.meta.location.clone(),
                        field: field.to_string(),
                    })?;
                    Ok(Self::Statement(target))
                } else if let Some(value) = stmt.get_value() {
                    Ok(Self::Value(value))
                } else {
                    error::NotScopeSnafu {
                        location: stmt.meta.location.clone(),
                    }
                    .fail()
                }
            }
            Self::Value(value) => {
                let table = value.as_table().context(error::NotScopeSnafu {
                    location: value.meta.location.clone(),
                })?;

                let target = table.get(field).context(error::NoFieldSnafu {
                    location: value.meta.location.clone(),
                    field: field.to_string(),
                })?;

                Ok(Self::Value(target))
            }
        }
    }

    /// Get all field names in the current scope
    pub fn field_names(&self) -> Result<Vec<String>> {
        match self {
            Self::Statement(stmt) => {
                let children = stmt
                    .get_grouped()
                    .or_else(|| stmt.get_labeled().map(|x| x.1))
                    .context(error::NotScopeSnafu {
                        location: stmt.meta.location.clone(),
                    })?;

                Ok(children.keys().cloned().collect())
            }
            Self::Value(value) => {
                let table = value.as_table().context(error::NotScopeSnafu {
                    location: value.meta.location.clone(),
                })?;

                Ok(table.keys().cloned().collect())
            }
        }
    }

    /// Check if a field exists
    pub fn has_field(&self, field: &str) -> bool {
        self.get_child(field).is_ok() || self.walk(field).is_ok()
    }

    /// Get the number of children/fields
    pub fn len(&self) -> usize {
        match self {
            Self::Statement(stmt) => stmt
                .get_grouped()
                .or_else(|| stmt.get_labeled().map(|x| x.1))
                .map(|children| children.len())
                .unwrap_or(0),
            Self::Value(value) => value.as_table().map(|table| table.len()).unwrap_or(0),
        }
    }

    /// Check if the current node is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Represents the type of a node in the AST
#[derive(Debug, Clone)]
pub enum NodeType {
    Statement(crate::StatementType),
    Value(ValueType),
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeType::Statement(stmt_type) => write!(
                f,
                "Statement({})",
                match stmt_type {
                    crate::StatementType::Control(_) => "Control",
                    crate::StatementType::Assignment(_) => "Assignment",
                    crate::StatementType::Block { .. } => "Block",
                    crate::StatementType::Module(_) => "Module",
                }
            ),
            NodeType::Value(value_type) => write!(f, "Value({})", value_type),
        }
    }
}

impl<'source> fmt::Debug for Walk<'source> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Statement(stmt) => f
                .debug_struct("Walk::Statement")
                .field("id", &stmt.id)
                .field("type", &stmt.type_)
                .field("location", &stmt.meta.location)
                .finish(),
            Self::Value(value) => f
                .debug_struct("Walk::Value")
                .field("type", &value.type_of())
                .field("location", &value.meta.location)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Metadata, Statement, Value};
    use indexmap::IndexMap;

    fn create_test_statement() -> Statement {
        let mut children = IndexMap::new();

        let value = Value::new_string("test_value".to_string(), Metadata::default());
        let stmt = Statement::new_assign("test_field", None, value, Metadata::default()).unwrap();
        children.insert("test_field".to_string(), stmt);

        Statement::new_module("test", children, Metadata::default())
    }

    #[test]
    fn test_walker_creation() {
        let stmt = create_test_statement();
        let walker = Walk::new(&stmt);

        assert!(walker.is_statement());
        assert!(!walker.is_value());
        assert_eq!(walker.get_id(), Some("test"));
    }

    #[test]
    fn test_field_access() {
        let stmt = create_test_statement();
        let walker = Walk::new(&stmt);

        assert!(walker.has_field("test_field"));
        assert!(!walker.has_field("nonexistent"));

        let value: String = walker.get("test_field").unwrap();
        assert_eq!(value, "test_value");
    }

    #[test]
    fn test_navigation() {
        let stmt = create_test_statement();
        let walker = Walk::new(&stmt);

        let child_walker = walker.walk("test_field").unwrap();
        assert!(child_walker.is_statement());
    }

    #[test]
    fn test_field_enumeration() {
        let stmt = create_test_statement();
        let walker = Walk::new(&stmt);

        let fields = walker.field_names().unwrap();
        assert_eq!(fields.len(), 1);
        assert!(fields.contains(&"test_field".to_string()));

        assert_eq!(walker.len(), 1);
        assert!(!walker.is_empty());
    }

    #[test]
    fn test_node_type() {
        let stmt = create_test_statement();
        let walker = Walk::new(&stmt);

        let node_type = walker.node_type();
        assert!(matches!(node_type, NodeType::Statement(_)));
    }
}
