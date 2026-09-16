use super::types::{Metadata, StatementType, ValueType};
use super::value::{Data, Value};
use crate::{Result, error};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use uuid::Uuid;

fn deserialize_labels<'de, D>(deserializer: D) -> std::result::Result<Vec<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let labels = Vec::<Value>::deserialize(deserializer)?;
    for label in &labels {
        if !matches!(label.data, Data::String(_)) {
            return Err(serde::de::Error::custom(format!(
                "invalid block label: expected a string, found {}",
                label.type_of()
            )));
        }
    }
    Ok(labels)
}

/// Contains the actual set of data for a statement
///
/// This enum represents the different kinds of data that can be stored in a statement,
/// depending on the statement type.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum StatementData {
    /// Data for block statements, which have labels and child statements
    ///
    /// The first parameter is a vector of literal string label values.
    /// The second parameter is a map of child statements indexed by their IDs.
    Labeled(
        #[serde(deserialize_with = "deserialize_labels")] Vec<Value>,
        IndexMap<String, Statement>,
    ),

    /// Data for module statements, which contain child statements
    ///
    /// The parameter is a map of child statements indexed by their IDs.
    Group(IndexMap<String, Statement>),

    /// Data for control and assignment statements, which contain a single value
    ///
    /// The parameter is the value assigned to the statement.
    Single(Value),
}

impl StatementData {
    /// Returns true if this statement data contains child statements
    pub const fn has_children(&self) -> bool {
        matches!(self, Self::Labeled(_, _) | Self::Group(_))
    }

    /// Returns the number of direct children
    pub fn child_count(&self) -> usize {
        match self {
            Self::Labeled(_, children) | Self::Group(children) => children.len(),
            Self::Single(_) => 0,
        }
    }

    /// Returns an iterator over all child statements
    pub fn children(&self) -> Box<dyn Iterator<Item = &Statement> + '_> {
        match self {
            Self::Labeled(_, children) | Self::Group(children) => Box::new(children.values()),
            Self::Single(_) => Box::new(std::iter::empty()),
        }
    }

    /// Returns a mutable iterator over all child statements
    pub fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut Statement> + '_> {
        match self {
            Self::Labeled(_, children) | Self::Group(children) => Box::new(children.values_mut()),
            Self::Single(_) => Box::new(std::iter::empty()),
        }
    }
}

/// Represents top-level statements and groupings in the BarkML language
///
/// A Statement is a fundamental structural element in BarkML. It can represent
/// assignments, control statements, blocks, or modules. Each statement
/// has a unique identifier, a type, metadata, and associated data.
#[derive(Clone, Deserialize, Serialize)]
pub struct Statement {
    /// Unique identifier for the statement, used for reference tracking
    pub uid: Uuid,

    /// Identifier name of the statement (e.g., variable name, block name)
    pub id: String,

    /// Type information for the statement
    pub type_: StatementType,

    /// Metadata including source location, comments, and labels
    pub meta: Metadata,

    /// The actual data contained in this statement
    pub data: StatementData,
}

impl Statement {
    /// Creates a new Statement with the given properties
    pub fn new(id: &str, type_: StatementType, data: StatementData, meta: Metadata) -> Self {
        Self {
            uid: Uuid::now_v7(),
            id: id.to_string(),
            type_,
            meta,
            data,
        }
    }

    /// Returns true if this statement is a container (has children)
    pub const fn is_container(&self) -> bool {
        self.data.has_children()
    }

    /// Returns true if this statement is an assignment
    pub const fn is_assignment(&self) -> bool {
        matches!(self.data, StatementData::Single(_))
    }

    /// Returns the number of direct children
    pub fn child_count(&self) -> usize {
        self.data.child_count()
    }

    /// Returns an iterator over all child statements
    pub fn children(&self) -> Box<dyn Iterator<Item = &Statement> + '_> {
        self.data.children()
    }

    /// Returns a mutable iterator over all child statements
    pub fn children_mut(&mut self) -> Box<dyn Iterator<Item = &mut Statement> + '_> {
        self.data.children_mut()
    }

    /// Recursively counts all statements in the tree
    pub fn total_statement_count(&self) -> usize {
        1 + self
            .children()
            .map(|child| child.total_statement_count())
            .sum::<usize>()
    }

    /// Finds a child statement by ID
    pub fn find_child(&self, id: &str) -> Option<&Statement> {
        match &self.data {
            StatementData::Labeled(_, children) | StatementData::Group(children) => {
                children.get(id)
            }
            StatementData::Single(_) => None,
        }
    }

    /// Finds a child statement by ID (mutable)
    pub fn find_child_mut(&mut self, id: &str) -> Option<&mut Statement> {
        match &mut self.data {
            StatementData::Labeled(_, children) | StatementData::Group(children) => {
                children.get_mut(id)
            }
            StatementData::Single(_) => None,
        }
    }

    /// Recursively searches for a statement by path (dot-separated)
    pub fn find_by_path(&self, path: &str) -> Option<&Statement> {
        let parts: Vec<&str> = path.split('.').collect();
        self.find_by_path_parts(&parts)
    }

    fn find_by_path_parts(&self, parts: &[&str]) -> Option<&Statement> {
        if parts.is_empty() {
            return Some(self);
        }

        let child = self.find_child(parts[0])?;
        if parts.len() == 1 {
            Some(child)
        } else {
            child.find_by_path_parts(&parts[1..])
        }
    }
}

impl PartialEq for Statement {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.data == other.data
    }
}

impl Statement {
    /// Check and convert a value for an assignment with improved error handling
    fn convert_value(expected_type: &ValueType, value: &Value) -> Result<Value> {
        // If types match exactly, no conversion needed
        if expected_type == &value.type_of() {
            return Ok(value.clone());
        }

        // Check if conversion is allowed
        if !expected_type.can_assign(&value.type_of()) {
            return error::ImplicitConvertSnafu {
                left: expected_type.clone(),
                right: value.type_of(),
            }
            .fail();
        }

        // Perform the conversion
        let converted_data = match (expected_type, &value.data) {
            // Unsigned integer conversions
            (ValueType::Unsigned, Data::U8(v)) => Data::Unsigned(*v as u64),
            (ValueType::Unsigned, Data::U16(v)) => Data::Unsigned(*v as u64),
            (ValueType::Unsigned, Data::U32(v)) => Data::Unsigned(*v as u64),
            (ValueType::Unsigned, Data::U64(v)) => Data::Unsigned(*v),
            (ValueType::Unsigned, Data::Unsigned(v)) => Data::Unsigned(*v),

            (ValueType::U64, Data::Unsigned(v)) => Data::U64(*v),
            (ValueType::U64, Data::U8(v)) => Data::U64(*v as u64),
            (ValueType::U64, Data::U16(v)) => Data::U64(*v as u64),
            (ValueType::U64, Data::U32(v)) => Data::U64(*v as u64),
            (ValueType::U64, Data::U64(v)) => Data::U64(*v),

            // Signed integer conversions
            (ValueType::Signed, Data::I8(v)) => Data::Signed(*v as i64),
            (ValueType::Signed, Data::I16(v)) => Data::Signed(*v as i64),
            (ValueType::Signed, Data::I32(v)) => Data::Signed(*v as i64),
            (ValueType::Signed, Data::I64(v)) => Data::Signed(*v),
            (ValueType::Signed, Data::Signed(v)) => Data::Signed(*v),
            (ValueType::Signed, Data::U8(v)) => Data::Signed(*v as i64),
            (ValueType::Signed, Data::U16(v)) => Data::Signed(*v as i64),
            (ValueType::Signed, Data::U32(v)) => Data::Signed(*v as i64),

            (ValueType::I64, Data::Signed(v)) => Data::I64(*v),
            (ValueType::I64, Data::I8(v)) => Data::I64(*v as i64),
            (ValueType::I64, Data::I16(v)) => Data::I64(*v as i64),
            (ValueType::I64, Data::I32(v)) => Data::I64(*v as i64),
            (ValueType::I64, Data::I64(v)) => Data::I64(*v),
            (ValueType::I64, Data::U8(v)) => Data::I64(*v as i64),
            (ValueType::I64, Data::U16(v)) => Data::I64(*v as i64),
            (ValueType::I64, Data::U32(v)) => Data::I64(*v as i64),

            // Float conversions
            (ValueType::Float, Data::F32(v)) => Data::Float(*v as f64),
            (ValueType::Float, Data::F64(v)) => Data::Float(*v),
            (ValueType::Float, Data::Float(v)) => Data::Float(*v),

            (ValueType::F64, Data::Float(v)) => Data::F64(*v),
            (ValueType::F64, Data::F32(v)) => Data::F64(*v as f64),
            (ValueType::F64, Data::F64(v)) => Data::F64(*v),

            // If we get here, the conversion should have been caught earlier
            _ => {
                return error::ImplicitConvertSnafu {
                    left: expected_type.clone(),
                    right: value.type_of(),
                }
                .fail();
            }
        };

        Ok(Value {
            uid: value.uid,
            data: converted_data,
            meta: value.meta.clone(),
        })
    }

    /// Creates a new assignment statement
    pub fn new_assign(
        id: &str,
        type_hint: Option<ValueType>,
        value: Value,
        meta: Metadata,
    ) -> Result<Self> {
        let expected_type = type_hint.unwrap_or_else(|| value.type_of());
        let converted_value = Self::convert_value(&expected_type, &value)?;

        Ok(Self::new(
            id,
            StatementType::Assignment(expected_type),
            StatementData::Single(converted_value),
            meta,
        ))
    }

    /// Creates a new block statement
    pub fn new_block(
        id: &str,
        labels: Vec<Value>,
        children: IndexMap<String, Statement>,
        meta: Metadata,
    ) -> Self {
        let statement_type = StatementType::Block {
            labels: labels.iter().map(|x| x.type_of()).collect(),
            contents: children
                .iter()
                .map(|(k, v)| (k.clone(), v.type_.clone()))
                .collect(),
        };

        Self::new(
            id,
            statement_type,
            StatementData::Labeled(labels, children),
            meta,
        )
    }

    /// Creates a new module statement
    pub fn new_module(id: &str, children: IndexMap<String, Statement>, meta: Metadata) -> Self {
        let statement_type = StatementType::Module(
            children
                .iter()
                .map(|(k, v)| (k.clone(), v.type_.clone()))
                .collect(),
        );

        Self::new(id, statement_type, StatementData::Group(children), meta)
    }

    /// Gets the value for assignment statements
    pub fn get_value(&self) -> Option<&Value> {
        match &self.data {
            StatementData::Single(value) => Some(value),
            _ => None,
        }
    }

    /// Gets the labels and children for block statements
    pub fn get_labeled(&self) -> Option<(&Vec<Value>, &IndexMap<String, Statement>)> {
        match &self.data {
            StatementData::Labeled(labels, contents) => Some((labels, contents)),
            _ => None,
        }
    }

    /// Gets the children for container statements
    pub fn get_grouped(&self) -> Option<&IndexMap<String, Statement>> {
        match &self.data {
            StatementData::Group(contents) => Some(contents),
            StatementData::Labeled(_, contents) => Some(contents),
            _ => None,
        }
    }

    /// Returns the semantic identity of this statement: its id plus, for
    /// blocks, the ordered sequence of literal string labels.
    ///
    /// Component boundaries are preserved: `app "a.b"` and `app "a" "b"`
    /// are distinct identities.
    pub fn identity(&self) -> (&str, &[Value]) {
        let labels = match &self.data {
            StatementData::Labeled(labels, _) => labels.as_slice(),
            _ => &[],
        };
        (self.id.as_str(), labels)
    }

    /// Returns the storage key used to slot this statement into its parent's
    /// child map. This is a map key, not a semantic identity: zero-label
    /// statements use their id; labeled blocks use their unique uid so that
    /// same-kind siblings with different labels never overwrite each other.
    pub fn storage_key(&self) -> String {
        match &self.data {
            StatementData::Labeled(labels, _) if !labels.is_empty() => self.uid.to_string(),
            _ => self.id.clone(),
        }
    }

    /// Finds a child block by structured identity: the block id plus the
    /// ordered label sequence. An empty label slice matches zero-label blocks
    /// and non-block children with the given id.
    pub fn get_child(&self, id: &str, labels: &[&str]) -> Option<&Statement> {
        self.children().find(|child| {
            let (child_id, child_labels) = child.identity();
            child_id == id
                && child_labels.len() == labels.len()
                && child_labels.iter().zip(labels).all(|(value, expected)| {
                    value.as_string().map(|s| s == expected).unwrap_or(false)
                })
        })
    }

    /// Iterates over the child blocks of this statement, yielding each
    /// block's structured identity `(id, ordered labels, statement)`.
    ///
    /// This is the AST-aware traversal for consumers that need resource
    /// names; the Serde child-map projection does not expose labels.
    pub fn blocks(&self) -> impl Iterator<Item = (&str, &[Value], &Statement)> {
        self.children()
            .filter(|child| matches!(child.data, StatementData::Labeled(..)))
            .map(|child| {
                let labels = match &child.data {
                    StatementData::Labeled(labels, _) => labels.as_slice(),
                    _ => &[],
                };
                (child.id.as_str(), labels, child)
            })
    }

    /// Validates the statement structure recursively
    pub fn validate(&self) -> Result<()> {
        // Validate this statement
        if let StatementType::Assignment(expected) = &self.type_
            && let Some(value) = self.get_value()
            && !expected.can_assign(&value.type_of())
        {
            return error::ImplicitConvertSnafu {
                left: expected.clone(),
                right: value.type_of(),
            }
            .fail();
        }

        // Recursively validate children
        for child in self.children() {
            child.validate()?;
        }

        Ok(())
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write comment if present
        if let Some(comment) = self.meta.comment.as_ref() {
            writeln!(f, "/*\n{}\n*/", comment)?;
        }

        match &self.type_ {
            StatementType::Assignment(type_) => {
                write!(f, "{}: {} = {}", self.id, type_, self.get_value().unwrap())
            }
            StatementType::Block { .. } => {
                let (labels, body) = self.get_labeled().unwrap();
                let labels_str = if labels.is_empty() {
                    String::new()
                } else {
                    format!(
                        " {}",
                        labels
                            .iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )
                };

                writeln!(f, "{}{} {{", self.id, labels_str)?;
                for child in body.values() {
                    writeln!(f, "  {}", child)?;
                }
                write!(f, "}}")
            }
            StatementType::Module(_) => {
                let body = self.get_grouped().unwrap();
                for (i, child) in body.values().enumerate() {
                    if i > 0 {
                        writeln!(f)?;
                    }
                    write!(f, "{}", child)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Statement")
            .field("uid", &self.uid)
            .field("id", &self.id)
            .field("type", &self.type_)
            .field("meta", &self.meta)
            .field("child_count", &self.child_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::Location;

    #[test]
    fn test_statement_creation() {
        let meta = Metadata::new(Location::new(0, 0));
        let value = Value::new_string("test".to_string(), meta.clone());

        let stmt = Statement::new_assign("test_var", None, value, meta).unwrap();
        assert_eq!(stmt.id, "test_var");
        assert!(stmt.is_assignment());
        assert!(!stmt.is_container());
    }

    #[test]
    fn test_statement_children() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();

        let child_value = Value::new_int(42, meta.clone());
        let child_stmt = Statement::new_assign("child", None, child_value, meta.clone()).unwrap();
        children.insert("child".to_string(), child_stmt);

        let block = Statement::new_block("test_block", Vec::new(), children, meta);

        assert!(block.is_container());
        assert_eq!(block.child_count(), 1);
        assert!(block.find_child("child").is_some());
    }

    #[test]
    fn test_statement_path_finding() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();
        let mut grandchildren = IndexMap::new();

        let grandchild_value = Value::new_string("deep".to_string(), meta.clone());
        let grandchild =
            Statement::new_assign("grandchild", None, grandchild_value, meta.clone()).unwrap();
        grandchildren.insert("grandchild".to_string(), grandchild);

        let child = Statement::new_block("child", Vec::new(), grandchildren, meta.clone());
        children.insert("child".to_string(), child);

        let root = Statement::new_module("root", children, meta);

        assert!(root.find_by_path("child").is_some());
        assert!(root.find_by_path("child.grandchild").is_some());
        assert!(root.find_by_path("nonexistent").is_none());
    }

    #[test]
    fn test_type_conversion() {
        let meta = Metadata::new(Location::new(0, 0));
        let value = Value::new_u32(42, meta.clone());

        // Should convert u32 to u64
        let stmt = Statement::new_assign("test", Some(ValueType::U64), value, meta).unwrap();

        if let Some(converted_value) = stmt.get_value() {
            assert_eq!(converted_value.type_of(), ValueType::U64);
            assert_eq!(converted_value.as_u64(), Some(&42u64));
        }
    }
    #[test]
    fn test_labeled_statement_serde_round_trip() {
        use serde_json;

        let meta = Metadata::new(Location::new(1, 2));
        let label = Value::new_string("a.b".to_string(), meta.clone());
        let mut inner = IndexMap::new();
        inner.insert(
            "enabled".to_string(),
            Statement::new_assign(
                "enabled",
                None,
                Value::new_bool(true, meta.clone()),
                meta.clone(),
            )
            .unwrap(),
        );
        let block = Statement::new_block("app", vec![label], inner, meta);

        // Derived AST serde preserves Labeled data, including labels
        let json = serde_json::to_string(&block).unwrap();
        let round: Statement = serde_json::from_str(&json).unwrap();

        let (labels, children) = round.get_labeled().unwrap();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].as_string(), Some(&"a.b".to_string()));
        assert_eq!(
            children["enabled"].get_value().unwrap().as_bool(),
            Some(&true)
        );
    }
    #[test]
    fn test_non_string_labels_rejected_by_serde() {
        use serde_json;

        let label = Value::new_u32(3, Metadata::new(Location::new(0, 0)));
        let block = Statement::new_block(
            "app",
            vec![label],
            IndexMap::new(),
            Metadata::new(Location::new(0, 0)),
        );
        let json = serde_json::to_string(&block).unwrap();
        let err = serde_json::from_str::<Statement>(&json).unwrap_err();
        assert!(
            err.to_string().contains("invalid block label"),
            "got: {err}"
        );
    }
}
