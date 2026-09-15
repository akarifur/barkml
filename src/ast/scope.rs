use super::types::StatementType;
use super::{Data, Statement, StatementData, Value};
use crate::{Result, error};
use indexmap::{IndexMap, IndexSet};
use snafu::{OptionExt, ensure};
use std::fmt;
use uuid::Uuid;

/// Maximum recursion depth for macro resolution to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 100;

/// A single component of a structured symbol-table path.
///
/// Path components keep block-identity boundaries intact: a block named
/// `app` labeled `"a.b"` contributes `[Id("app"), Label("a.b")]`, which
/// is distinct from `app "a" "b"` (`[Id("app"), Label("a"), Label("b")]`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Segment {
    /// A statement identifier component
    Id(String),
    /// A block label component
    Label(String),
}

impl Segment {
    /// Returns true if this segment is addressable by the given string
    /// on the dotted macro-reference surface syntax.
    pub fn matches(&self, input: &str) -> bool {
        match self {
            Self::Id(value) | Self::Label(value) => value == input,
        }
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Id(value) | Self::Label(value) => write!(f, "{}", value),
        }
    }
}

/// Scope is used to resolve macros and manage symbol references
///
/// The Scope struct provides functionality for resolving macro references within a BarkML
/// document. It builds a symbol table from a root statement (typically a module) and
/// provides methods to resolve all macro references to their actual values.
///
/// This is a key part of the BarkML processing pipeline, as it handles the substitution
/// of macro references with their actual values, allowing for powerful templating and
/// reuse capabilities in the language.
pub struct Scope {
    /// The root statement (typically a module) that defines the scope
    root: Statement,

    /// Maps structured paths to their corresponding values
    symbol_table: IndexMap<Vec<Segment>, Value>,

    /// Maps value UIDs to their structured paths for efficient lookup
    path_lookup: IndexMap<Uuid, Vec<Segment>>,

    /// Current recursion depth for macro resolution
    recursion_depth: usize,
}

impl Scope {
    /// Creates a new Scope from a root statement
    pub fn new(node: &Statement) -> Self {
        let mut scope = Self {
            root: node.clone(),
            symbol_table: IndexMap::new(),
            path_lookup: IndexMap::new(),
            recursion_depth: 0,
        };
        Self::build_symbol_table(&mut scope, node, Vec::new());
        scope
    }

    /// Builds the symbol table by walking the AST
    fn build_symbol_table(scope: &mut Scope, node: &Statement, path: Vec<Segment>) {
        let mut new_path = path;
        new_path.push(Segment::Id(node.id.clone()));

        // A block's labels are ordered path components in their own right,
        // preserving component boundaries instead of dot-joining.
        if let StatementData::Labeled(labels, _) = &node.data {
            for label in labels {
                if let Some(text) = label.as_string() {
                    new_path.push(Segment::Label(text.clone()));
                }
            }
        }

        match &node.data {
            StatementData::Group(children) | StatementData::Labeled(_, children) => {
                for child in children.values() {
                    Self::build_symbol_table(scope, child, new_path.clone());
                }
            }
            StatementData::Single(value) => {
                Self::walk_value(scope, value, new_path);
            }
        }
    }

    /// Recursively walks through a value to build symbol table entries
    fn walk_value(scope: &mut Scope, node: &Value, path: Vec<Segment>) {
        Self::add_symbol(scope, path.clone(), node);

        match &node.data {
            Data::Table(contents) => {
                for (key, value) in contents {
                    let mut new_path = path.clone();
                    new_path.push(Segment::Id(key.clone()));
                    Self::walk_value(scope, value, new_path);
                }
            }
            Data::Array(contents) => {
                for (index, child) in contents.iter().enumerate() {
                    let mut array_path = path.clone();
                    array_path.push(Segment::Id(index.to_string()));
                    Self::walk_value(scope, child, array_path);
                }
            }
            _ => {}
        }
    }

    /// Adds a symbol to the symbol table
    fn add_symbol(scope: &mut Scope, path: Vec<Segment>, node: &Value) {
        scope.symbol_table.insert(path.clone(), node.clone());
        scope.path_lookup.insert(node.uid, path);
    }

    /// Finds a symbol table entry addressable by the given dotted path
    /// components. Both `Id` and `Label` segments match by their string
    /// value, in path order, so ordered label sequences are addressed
    /// unambiguously.
    fn get_by_parts(&self, parts: &[String]) -> Option<&Value> {
        self.symbol_table
            .iter()
            .find(|(path, _)| {
                path.len() == parts.len()
                    && path
                        .iter()
                        .zip(parts)
                        .all(|(segment, part)| segment.matches(part))
            })
            .map(|(_, value)| value)
    }

    /// Applies macro resolution to the entire scope
    pub fn apply(&mut self) -> Result<Statement> {
        let mut visit_log = IndexSet::new();
        let root = self.root.clone();
        self.recursion_depth = 0;
        self.resolve_statement(&root, &mut visit_log)
    }

    /// Resolves a path reference, handling relative paths like 'self' and 'super'
    fn resolve_path(&self, current: &Value, input: String) -> Result<Vec<String>> {
        let operating_path: Vec<String> = if input.starts_with("self") || input.starts_with("super")
        {
            let current_path = self
                .path_lookup
                .get(&current.uid)
                .context(error::NoMacroSnafu {
                    location: current.meta.location.clone(),
                    path: "unknown".to_string(),
                })?;
            let current_path: Vec<String> = current_path
                .iter()
                .map(|segment| segment.to_string())
                .collect();

            if input.starts_with("super") {
                let mut current_segments = current_path;
                let new_segments: Vec<&str> = input.split('.').collect();

                // Remove the current segment for 'super'
                current_segments.pop();

                // Add the remaining path segments
                for segment in new_segments.iter().skip(1) {
                    if *segment == "super" {
                        current_segments.pop();
                    } else {
                        current_segments.push(segment.to_string());
                    }
                }

                current_segments
            } else {
                // Replace 'self' with current path
                let replaced = input.replace("self", &current_path.join("."));
                replaced.split('.').map(|x| x.to_string()).collect()
            }
        } else {
            input.split('.').map(|x| x.to_string()).collect()
        };

        // Normalize the path by handling 'this', 'self', and 'super' references
        let mut final_path: Vec<String> = Vec::new();
        for entry in operating_path {
            match entry.as_str() {
                "this" | "self" => continue, // Skip these as they're already resolved
                "super" => {
                    final_path.pop(); // Go up one level
                }
                _ => {
                    final_path.push(entry);
                }
            }
        }

        Ok(final_path)
    }
    /// Resolves a macro reference to its actual value
    fn resolve_macro(
        &mut self,
        at: &Value,
        input: String,
        visit_log: &mut IndexSet<Uuid>,
    ) -> Result<Value> {
        // Check recursion depth
        ensure!(
            self.recursion_depth < MAX_RECURSION_DEPTH,
            error::RecursionLimitSnafu {
                location: at.meta.location.clone(),
                limit: MAX_RECURSION_DEPTH,
            }
        );

        self.recursion_depth += 1;
        let result = self.resolve_macro_internal(at, input, visit_log);
        self.recursion_depth -= 1;
        result
    }

    fn resolve_macro_internal(
        &mut self,
        at: &Value,
        input: String,
        visit_log: &mut IndexSet<Uuid>,
    ) -> Result<Value> {
        // First check if the whole string is a singular reference to a macro value
        let path = self.resolve_path(at, input.clone())?;

        if let Some(data) = self.get_by_parts(&path) {
            let mut resolved_value = Value {
                uid: at.uid,
                data: data.data.clone(),
                meta: at.meta.clone(),
            };

            // If the resolved value is still a macro, resolve it recursively
            if matches!(resolved_value.data, Data::Macro(_)) {
                resolved_value = self.resolve_value(&resolved_value, visit_log)?;
            }

            Ok(resolved_value)
        } else {
            // Handle macro string interpolation
            self.resolve_macro_string(at, input, visit_log)
        }
    }

    /// Resolves macro string interpolation (e.g., "Hello {name}")
    fn resolve_macro_string(
        &mut self,
        at: &Value,
        input: String,
        visit_log: &mut IndexSet<Uuid>,
    ) -> Result<Value> {
        // Check if there are any interpolation markers
        if !input.contains('{') {
            return error::NoMacroSnafu {
                location: at.meta.location.clone(),
                path: input,
            }
            .fail();
        }

        let mut result = String::new();
        let chars = input.chars().peekable();
        let mut brace_depth = 0;
        let mut current_macro = String::new();

        for ch in chars {
            match ch {
                '{' if brace_depth == 0 => {
                    brace_depth = 1;
                    current_macro.clear();
                }
                '{' if brace_depth > 0 => {
                    brace_depth += 1;
                    current_macro.push(ch);
                }
                '}' if brace_depth == 1 => {
                    // Resolve the macro reference
                    let path = self.resolve_path(at, current_macro.clone())?;
                    let resolved_value = self.get_by_parts(&path).context(error::NoMacroSnafu {
                        location: at.meta.location.clone(),
                        path: current_macro.clone(),
                    })?;

                    let mut final_value = resolved_value.clone();
                    if matches!(final_value.data, Data::Macro(_)) {
                        final_value = self.resolve_value(&final_value, visit_log)?;
                    }

                    result.push_str(&final_value.to_macro_string());
                    brace_depth = 0;
                    current_macro.clear();
                }
                '}' if brace_depth > 1 => {
                    brace_depth -= 1;
                    current_macro.push(ch);
                }
                _ if brace_depth > 0 => {
                    current_macro.push(ch);
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        // Check for unclosed braces
        if brace_depth > 0 {
            return error::NoMacroSnafu {
                location: at.meta.location.clone(),
                path: format!("Unclosed macro reference: {{{}", current_macro),
            }
            .fail();
        }

        Ok(Value {
            uid: at.uid,
            data: Data::String(result),
            meta: at.meta.clone(),
        })
    }

    /// Resolves all macros in a statement
    fn resolve_statement(
        &mut self,
        at: &Statement,
        visit_log: &mut IndexSet<Uuid>,
    ) -> Result<Statement> {
        let uid = at.uid;

        // Check for circular references
        ensure!(
            !visit_log.contains(&uid),
            error::LoopSnafu {
                location: at.meta.location.clone()
            }
        );

        let result = match &at.type_ {
            StatementType::Module(_) => {
                let mut new_children = IndexMap::new();
                for (key, value) in at.get_grouped().unwrap() {
                    new_children.insert(key.clone(), self.resolve_statement(value, visit_log)?);
                }

                Statement::new_module(&at.id, new_children, at.meta.clone())
            }
            StatementType::Block { .. } => {
                let mut new_children = IndexMap::new();
                let mut new_labels = Vec::new();
                let (labels, children) = at.get_labeled().unwrap();

                for label in labels {
                    new_labels.push(self.resolve_value(label, visit_log)?);
                }

                for (key, value) in children.iter() {
                    new_children.insert(key.clone(), self.resolve_statement(value, visit_log)?);
                }

                Statement::new_block(&at.id, new_labels, new_children, at.meta.clone())
            }
            StatementType::Control(expected) => {
                let new_value = self.resolve_value(at.get_value().unwrap(), visit_log)?;

                // Validate type compatibility
                ensure!(
                    expected.can_assign(&new_value.type_of()),
                    error::ImplicitConvertSnafu {
                        left: expected.clone(),
                        right: new_value.type_of()
                    }
                );

                Statement::new_control(&at.id, Some(expected.clone()), new_value, at.meta.clone())?
            }
            StatementType::Assignment(expected) => {
                let is_macro = at.get_value().unwrap().as_macro().is_some();
                let new_value = self.resolve_value(at.get_value().unwrap(), visit_log)?;
                let expected = if is_macro {
                    new_value.type_of()
                } else {
                    expected.clone()
                };
                // Validate type compatibility
                ensure!(
                    expected.can_assign(&new_value.type_of()),
                    error::ImplicitConvertSnafu {
                        left: expected.clone(),
                        right: new_value.type_of()
                    }
                );

                Statement::new_assign(&at.id, Some(expected.clone()), new_value, at.meta.clone())?
            }
        };

        visit_log.insert(uid);
        Ok(result)
    }

    /// Resolves all macros in a value
    fn resolve_value(&mut self, at: &Value, visit_log: &mut IndexSet<Uuid>) -> Result<Value> {
        let uid = at.uid;

        // Check for circular references
        ensure!(
            !visit_log.contains(&uid),
            error::LoopSnafu {
                location: at.meta.location.clone()
            }
        );

        let result = match &at.data {
            Data::Macro(value) => self.resolve_macro(at, value.clone(), visit_log)?,
            Data::Table(children) => {
                let mut new_children = IndexMap::new();
                for (key, value) in children.iter() {
                    new_children.insert(key.clone(), self.resolve_value(value, visit_log)?);
                }
                Value {
                    uid,
                    data: Data::Table(new_children),
                    meta: at.meta.clone(),
                }
            }
            Data::Array(children) => {
                let mut new_children = Vec::new();
                for value in children.iter() {
                    new_children.push(self.resolve_value(value, visit_log)?);
                }
                Value {
                    uid,
                    data: Data::Array(new_children),
                    meta: at.meta.clone(),
                }
            }
            _ => at.clone(),
        };

        visit_log.insert(uid);
        Ok(result)
    }

    /// Returns a reference to the symbol table
    pub fn symbol_table(&self) -> &IndexMap<Vec<Segment>, Value> {
        &self.symbol_table
    }

    /// Returns a reference to the path lookup table
    pub fn path_lookup(&self) -> &IndexMap<Uuid, Vec<Segment>> {
        &self.path_lookup
    }

    /// Looks up a value by its dotted path. Both statement identifiers and
    /// block labels match by string, in order.
    pub fn lookup(&self, path: &str) -> Option<&Value> {
        let parts: Vec<String> = path.split('.').map(|x| x.to_string()).collect();
        self.get_by_parts(&parts)
    }

    /// Looks up a value by structured path segments
    pub fn lookup_segments(&self, path: &[Segment]) -> Option<&Value> {
        self.symbol_table.get(path)
    }

    /// Returns all available paths in the symbol table as dotted display
    /// strings. Note that dotted display is for diagnostics only; two
    /// distinct structured paths may render identically.
    pub fn available_paths(&self) -> Vec<String> {
        self.symbol_table
            .keys()
            .map(|path| {
                path.iter()
                    .map(|segment| segment.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            })
            .collect()
    }

    /// Validates that all macro references can be resolved
    pub fn validate_macros(&self) -> Result<()> {
        for (_path, value) in &self.symbol_table {
            if let Data::Macro(macro_ref) = &value.data {
                let resolved_path = self.resolve_path(value, macro_ref.clone())?;
                if self.get_by_parts(&resolved_path).is_none() {
                    return error::NoMacroSnafu {
                        location: value.meta.location.clone(),
                        path: macro_ref.clone(),
                    }
                    .fail();
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::{Location, Metadata};

    #[test]
    fn test_scope_creation() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();

        let value = Value::new_string("test".to_string(), meta.clone());
        let stmt = Statement::new_assign("test_var", None, value, meta.clone()).unwrap();
        children.insert("test_var".to_string(), stmt);

        let module = Statement::new_module("root", children, meta);
        let scope = Scope::new(&module);

        assert!(!scope.symbol_table.is_empty());
        assert!(scope.lookup("root.test_var").is_some());
    }

    #[test]
    fn test_macro_resolution() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();

        // Create a value to reference
        let target_value = Value::new_string("hello".to_string(), meta.clone());
        let target_stmt =
            Statement::new_assign("target", None, target_value, meta.clone()).unwrap();
        children.insert("target".to_string(), target_stmt);

        // Create a macro that references the target - specify String as expected type
        let macro_value = Value::new_macro("root.target".to_string(), meta.clone());
        let macro_stmt =
            Statement::new_assign("macro_ref", None, macro_value, meta.clone()).unwrap();
        children.insert("macro_ref".to_string(), macro_stmt);

        let module = Statement::new_module("root", children, meta);
        let mut scope = Scope::new(&module);

        let resolved = scope.apply().unwrap();
        let resolved_macro = resolved.find_by_path("macro_ref").unwrap();

        if let Some(resolved_value) = resolved_macro.get_value() {
            assert_eq!(resolved_value.as_string(), Some(&"hello".to_string()));
        }
    }
    #[test]
    fn test_labeled_block_identity_distinct() {
        let meta = Metadata::new(Location::new(0, 0));
        let mut children = IndexMap::new();

        let label = |text: &str| Value::new_string(text.to_string(), meta.clone());
        let make_block = |id: &str, labels: Vec<Value>, enabled: bool| {
            let mut inner = IndexMap::new();
            let value = Value::new_bool(enabled, meta.clone());
            inner.insert(
                "enabled".to_string(),
                Statement::new_assign("enabled", None, value, meta.clone()).unwrap(),
            );
            Statement::new_block(id, labels, inner, meta.clone())
        };

        // app "a.b" and app "a" "b" are distinct identities
        children.insert(
            "one".to_string(),
            make_block("app", vec![label("a.b")], true),
        );
        children.insert(
            "two".to_string(),
            make_block("app", vec![label("a"), label("b")], false),
        );

        let module = Statement::new_module("root", children, meta);
        let scope = Scope::new(&module);

        // Structured paths are distinct keys
        let dotted = scope
            .symbol_table()
            .get(&vec![
                Segment::Id("root".into()),
                Segment::Id("app".into()),
                Segment::Label("a.b".into()),
                Segment::Id("enabled".into()),
            ])
            .unwrap();
        assert_eq!(dotted.as_bool(), Some(&true));

        let split = scope
            .symbol_table()
            .get(&vec![
                Segment::Id("root".into()),
                Segment::Id("app".into()),
                Segment::Label("a".into()),
                Segment::Label("b".into()),
                Segment::Id("enabled".into()),
            ])
            .unwrap();
        assert_eq!(split.as_bool(), Some(&false));

        // Dotted surface lookup matches label components in order
        assert_eq!(
            scope.lookup("root.app.a.b.enabled").unwrap().as_bool(),
            Some(&false)
        );
    }
}
