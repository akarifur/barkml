//! Abstract Syntax Tree module for BarkML
//!
//! This module contains the core data structures that represent the parsed BarkML language.
//! The AST is composed of statements, values, types, and scopes that together form
//! a complete representation of a BarkML document.
//!
//! # Key Components
//!
//! - **Types**: Core type system including `ValueType`, `StatementType`, `Location`, and `Metadata`
//! - **Values**: Data representation with the `Value` and `Data` enums
//! - **Statements**: Structural elements with the `Statement` and `StatementData` enums
//! - **Scope**: Macro resolution and symbol table management
//!

mod scope;
mod statement;
mod types;
mod value;

// Re-export all public items from submodules
pub use scope::*;
pub use statement::*;
pub use types::*;
pub use value::*;

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    /// Recursively counts all values in a statement tree
    pub fn count_values(stmt: &Statement) -> usize {
        match &stmt.data {
            StatementData::Single(_) => 1,
            StatementData::Group(children) | StatementData::Labeled(_, children) => {
                children.values().map(count_values).sum()
            }
        }
    }

    /// Recursively counts all statements in a statement tree
    pub fn count_statements(stmt: &Statement) -> usize {
        1 + stmt.children().map(count_statements).sum::<usize>()
    }

    /// Finds all statements of a specific type
    pub fn find_statements_by_type(
        stmt: &Statement,
        target_type: &StatementType,
    ) -> Vec<Statement> {
        let mut results = Vec::new();

        if std::mem::discriminant(&stmt.type_) == std::mem::discriminant(target_type) {
            results.push(stmt.clone());
        }

        for child in stmt.children() {
            results.extend(find_statements_by_type(child, target_type));
        }

        results
    }

    /// Finds all values of a specific type
    pub fn find_values_by_type(stmt: &Statement, target_type: &ValueType) -> Vec<Value> {
        let mut results = Vec::new();

        if let Some(value) = stmt.get_value() {
            if std::mem::discriminant(&value.type_of()) == std::mem::discriminant(target_type) {
                results.push(value.clone());
            }
        }

        for child in stmt.children() {
            results.extend(find_values_by_type(child, target_type));
        }

        results
    }

    /// Validates an entire AST tree
    pub fn validate_tree(stmt: &Statement) -> crate::Result<()> {
        stmt.validate()?;
        for child in stmt.children() {
            validate_tree(child)?;
        }
        Ok(())
    }

    /// Calculates the total memory usage of an AST tree
    pub fn calculate_memory_usage(stmt: &Statement) -> usize {
        let mut total = std::mem::size_of::<Statement>();

        if let Some(value) = stmt.get_value() {
            total += value.memory_size();
        }

        for child in stmt.children() {
            total += calculate_memory_usage(child);
        }

        total
    }

    /// Collects all unique identifiers in the AST
    pub fn collect_identifiers(stmt: &Statement) -> std::collections::HashSet<String> {
        let mut identifiers = std::collections::HashSet::new();
        identifiers.insert(stmt.id.clone());

        for child in stmt.children() {
            identifiers.extend(collect_identifiers(child));
        }

        identifiers
    }

    /// Pretty prints an AST tree with indentation
    pub fn pretty_print(stmt: &Statement, indent: usize) -> String {
        let mut result = String::new();
        let indent_str = "  ".repeat(indent);

        result.push_str(&format!(
            "{}{}[{}] ({})\n",
            indent_str,
            stmt.id,
            stmt.uid,
            match &stmt.type_ {
                StatementType::Assignment(_) => "Assignment",
                StatementType::Block { .. } => "Block",
                StatementType::Module(_) => "Module",
            }
        ));

        if let Some(value) = stmt.get_value() {
            result.push_str(&format!(
                "{}  Value: {} ({})\n",
                indent_str,
                value.type_of(),
                value.to_macro_string()
            ));
        }

        for child in stmt.children() {
            result.push_str(&pretty_print(child, indent + 1));
        }

        result
    }

    #[test]
    fn test_ast_creation_and_traversal() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();

        // Create some child statements
        let value1 = Value::new_string("test1".to_string(), meta.clone());
        let stmt1 = Statement::new_assign("var1", None, value1, meta.clone()).unwrap();
        children.insert("var1".to_string(), stmt1);

        let value2 = Value::new_int(42, meta.clone());
        let stmt2 = Statement::new_assign("var2", None, value2, meta.clone()).unwrap();
        children.insert("var2".to_string(), stmt2);

        let module = Statement::new_module("root", children, meta);

        // Test utility functions
        assert_eq!(count_statements(&module), 3); // root + 2 children
        assert_eq!(count_values(&module), 2); // 2 values

        let identifiers = collect_identifiers(&module);
        assert!(identifiers.contains("root"));
        assert!(identifiers.contains("var1"));
        assert!(identifiers.contains("var2"));

        // Test memory calculation
        let memory_usage = calculate_memory_usage(&module);
        assert!(memory_usage > 0);

        // Test validation
        assert!(validate_tree(&module).is_ok());
    }

    #[test]
    fn test_find_by_type() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();

        let value = Value::new_string("test".to_string(), meta.clone());
        let stmt = Statement::new_assign("var", None, value, meta.clone()).unwrap();
        children.insert("var".to_string(), stmt);

        let module = Statement::new_module("root", children, meta);

        let assignments =
            find_statements_by_type(&module, &StatementType::Assignment(ValueType::String));
        assert_eq!(assignments.len(), 1);

        let strings = find_values_by_type(&module, &ValueType::String);
        assert_eq!(strings.len(), 1);
    }

    #[test]
    fn test_pretty_print() {
        let meta = Metadata::new(Location::new(0, 0));
        let value = Value::new_string("test".to_string(), meta.clone());
        let stmt = Statement::new_assign("var", None, value, meta).unwrap();

        let output = pretty_print(&stmt, 0);
        assert!(output.contains("var"));
        assert!(output.contains("Assignment"));
        assert!(output.contains("string"));
    }
}
