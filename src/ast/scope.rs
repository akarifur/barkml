use super::types::{StatementType, ValueType};
use super::{Data, Statement, StatementData, TemplatePart, Value};
use crate::{Result, error};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use snafu::ensure;
use std::fmt;
use uuid::Uuid;

/// Maximum recursion depth for macro resolution to prevent infinite loops
const MAX_RECURSION_DEPTH: usize = 100;

/// A single component of a structured symbol-table path or reference.
///
/// Path components keep block-identity boundaries intact: a block named
/// `app` labeled `"a.b"` contributes `[Id("app"), Label("a.b")]`, which
/// is distinct from `app "a" "b"` (`[Id("app"), Label("a"), Label("b")]`).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Segment {
    /// A statement identifier or table-key component (`vars.editor`)
    Id(String),
    /// A block label component
    Label(String),
    /// A quoted selector component (`["key.with.dots"]`); matches identifiers
    /// and labels by exact text without splitting on punctuation
    Key(String),
    /// A zero-based array element selector (`items[0]`)
    Index(usize),
}

impl Segment {
    /// Returns true if this segment is addressable by the given string
    /// on the dotted diagnostic surface (text comparison only; used for
    /// near-miss suggestions, not for resolution).
    pub fn matches(&self, input: &str) -> bool {
        match self {
            Self::Id(value) | Self::Label(value) | Self::Key(value) => value == input,
            Self::Index(value) => value.to_string() == input,
        }
    }

    /// Returns true if a reference segment can match this symbol-table
    /// segment. `Key` selectors address both identifiers and labels by exact
    /// text; `Id` matches identifiers; `Index` matches array elements only.
    pub fn accepts(&self, selector: &Self) -> bool {
        match (self, selector) {
            (Self::Id(a), Self::Id(b) | Self::Key(b))
            | (Self::Label(a), Self::Label(b) | Self::Key(b)) => a == b,
            (Self::Index(a), Self::Index(b)) => a == b,
            _ => false,
        }
    }

    /// Renders a reference path in surface syntax: `vars.editor`,
    /// `app["a.b"].enabled`, `artifact["linux", "aarch64"]`, `items[0]`.
    /// Consecutive `Label`/`Key` selectors render as one bracket group.
    pub fn display_path(segments: &[Self]) -> String {
        let mut out = String::new();
        let mut bracket: Vec<String> = Vec::new();
        let flush = |out: &mut String, bracket: &mut Vec<String>| {
            if !bracket.is_empty() {
                out.push('[');
                out.push_str(
                    &bracket
                        .drain(..)
                        .map(|x| format!("\"{}\"", x))
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                out.push(']');
            }
        };

        for segment in segments {
            match segment {
                Self::Id(value) => {
                    flush(&mut out, &mut bracket);
                    if !out.is_empty() {
                        out.push('.');
                    }
                    out.push_str(value);
                }
                Self::Label(value) | Self::Key(value) => bracket.push(value.clone()),
                Self::Index(value) => {
                    flush(&mut out, &mut bracket);
                    out.push_str(&format!("[{}]", value));
                }
            }
        }
        flush(&mut out, &mut bracket);
        out
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Id(value) | Self::Label(value) | Self::Key(value) => write!(f, "{}", value),
            Self::Index(value) => write!(f, "{}", value),
        }
    }
}

/// Scope is used to resolve reference expressions.
///
/// The Scope struct builds a symbol table from a root statement (typically a
/// composed module) and resolves all reference expressions to their target
/// values. Resolution is root-relative: references never traverse relative
/// to their own location (`self`/`super` are ordinary identifiers), and
/// declaration order does not matter. References preserve the target's type
/// and value, including composite values.
///
/// # Compatibility changes (from macro replacements)
///
/// - `Data::Macro` and `ValueType::Macro` were removed in favor of
///   `Data::Reference(Vec<Segment>)` and `ValueType::Reference`.
/// - `Scope::lookup` now takes structured `&[Segment]` reference segments
///   (root-relative) instead of a dotted `&str`; `lookup_segments` still
///   takes a full symbol-table path including the root segment.
/// - `Scope::validate_macros` was renamed to `Scope::validate_references`.
/// - Implicit `self`/`super` relative lookup was removed; legacy `m!path`
///   and `m'...'` forms fail with a migration error at parse time.
/// - Array elements are addressed by zero-based index (`items[0]`), not by
///   legacy dotted numeric paths.
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

/// Whether a type contains a reference anywhere (including inside arrays
/// and tables); assignments of such values adopt the resolved target type.
fn contains_reference_impl(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Reference | ValueType::Template => true,
        ValueType::Array(children) => children.iter().any(contains_reference_impl),
        ValueType::Table(children) => children.values().any(contains_reference_impl),
        _ => false,
    }
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
                    array_path.push(Segment::Index(index));
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

    /// Finds symbol table entries whose path (excluding the root segment)
    /// is addressable by the given reference segments. Selector kinds are
    /// enforced: `Key` addresses identifiers and labels, `Index` addresses
    /// array elements only. Multiple matches indicate an ambiguous selector.
    fn reference_matches(&self, segments: &[Segment]) -> Vec<&Value> {
        self.symbol_table
            .iter()
            .filter(|(path, _)| {
                path.len() == segments.len() + 1
                    && path[1..]
                        .iter()
                        .zip(segments)
                        .all(|(segment, selector)| segment.accepts(selector))
            })
            .map(|(_, value)| value)
            .collect()
    }

    /// Applies reference resolution to the entire scope
    pub fn apply(&mut self) -> Result<Statement> {
        let mut visit_log = IndexSet::new();
        let root = self.root.clone();
        self.recursion_depth = 0;
        self.resolve_statement(&root, &mut visit_log)
    }

    /// Resolves a structured reference to its actual value, preserving the
    /// target's type and data. All lookups are root-relative; `self`/`super`
    /// carry no special meaning.
    fn resolve_reference(
        &mut self,
        at: &Value,
        segments: &[Segment],
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
        let mut chain_log = IndexSet::new();
        let result = self.resolve_reference_internal(at, segments, visit_log, &mut chain_log);
        self.recursion_depth -= 1;
        result
    }

    fn resolve_reference_internal(
        &mut self,
        at: &Value,
        segments: &[Segment],
        visit_log: &mut IndexSet<Uuid>,
        chain_log: &mut IndexSet<String>,
    ) -> Result<Value> {
        // Detect reference cycles across chained hops
        let display = Segment::display_path(segments);
        ensure!(
            chain_log.insert(display),
            error::LoopSnafu {
                location: at.meta.location.clone()
            }
        );

        let targets: Vec<&Value> = self.reference_matches(segments);
        if targets.len() > 1 {
            return error::WrongSelectorSnafu {
                location: at.meta.location.clone(),
                path: Segment::display_path(segments),
                reason: format!("selector matches {} distinct declarations", targets.len()),
            }
            .fail();
        }

        if let Some(target) = targets.first() {
            let resolved_value = Value {
                uid: at.uid,
                data: target.data.clone(),
                meta: at.meta.clone(),
            };

            // If the resolved value is itself a reference, resolve it recursively
            if let Data::Reference(chained) = &resolved_value.data {
                return self.resolve_reference_internal(at, chained, visit_log, chain_log);
            }

            // Composite targets may themselves contain references; resolve
            // them so no dangling references escape resolution. The copy is
            // resolved with a fresh visit log since cloned children retain
            // the target's UIDs; depth and chain guards still bound cycles.
            if resolved_value.data.is_collection() {
                let mut fresh_log = IndexSet::new();
                return self.resolve_value(&resolved_value, &mut fresh_log);
            }

            return Ok(resolved_value);
        }

        self.reference_failure(at, segments)
    }

    /// Produces a diagnostic for an unresolvable reference: distinguishes
    /// wrong selector kinds (quoted key vs array index), out-of-range and
    /// empty-array indices, and plain missing targets with near-miss paths.
    fn reference_failure(&self, at: &Value, segments: &[Segment]) -> Result<Value> {
        let display = Segment::display_path(segments);
        let location = at.meta.location.clone();

        // Longest kind-aware prefix already resolved
        let mut deepest: Option<(usize, &Value)> = None;
        for (path, value) in &self.symbol_table {
            if path.len() > segments.len() + 1 {
                continue;
            }
            let matched = path[1..]
                .iter()
                .zip(segments)
                .take_while(|(segment, selector)| segment.accepts(selector))
                .count();
            if deepest.is_none() || deepest.is_some_and(|(best, _)| matched > best) {
                deepest = Some((matched, value));
            }
        }

        if let Some((matched, value)) = deepest {
            if 0 < matched && matched < segments.len() {
                let next = &segments[matched];
                match (&value.data, next) {
                    (Data::Array(contents), Segment::Index(index)) if *index >= contents.len() => {
                        return error::NoElementSnafu {
                            location,
                            index: *index,
                        }
                        .fail();
                    }
                    (Data::Array(_), Segment::Key(key)) => {
                        return error::WrongSelectorSnafu {
                            location,
                            path: display,
                            reason: format!(
                                "quoted selector \"{}\" cannot address an array element; use a numeric index like items[0]",
                                key
                            ),
                        }
                        .fail();
                    }
                    (_, Segment::Index(index)) => {
                        return error::WrongSelectorSnafu {
                            location,
                            path: display,
                            reason: format!("numeric index [{}] used where no array exists", index),
                        }
                        .fail();
                    }
                    _ => {}
                }
            }
        }

        error::UnknownReferenceSnafu {
            location,
            path: display,
            available: self.near_misses(segments),
        }
        .fail()
    }

    /// Paths that share a component with the failing reference, capped for
    /// readable diagnostics.
    fn near_misses(&self, segments: &[Segment]) -> Vec<String> {
        let mut misses: Vec<String> = self
            .symbol_table
            .keys()
            .filter(|path| {
                path.len() > 1
                    && segments.iter().any(|selector| {
                        path[1..]
                            .iter()
                            .any(|segment| segment.matches(&selector.to_string()))
                    })
            })
            .map(|path| Segment::display_path(&path[1..]))
            .collect();
        misses.truncate(20);
        misses
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
                // Adopt the resolved target type only when the stored type was
                // derived from the parsed value (i.e. contains a reference);
                // declared types stay enforced against the resolved value
                let adopts_target = contains_reference_impl(expected);
                let new_value = self.resolve_value(at.get_value().unwrap(), visit_log)?;
                let expected = if adopts_target {
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

    /// Resolves an interpolated string: each placeholder is resolved with
    /// the same reference machinery (cycle, depth, and missing-target
    /// semantics) and rendered under the scalar-only interpolation policy.
    /// Targets that are themselves templates resolve recursively; the
    /// recursion depth guard bounds placeholder cycles.
    fn resolve_template(
        &mut self,
        at: &Value,
        parts: &[TemplatePart],
        visit_log: &mut IndexSet<Uuid>,
    ) -> Result<Value> {
        ensure!(
            self.recursion_depth < MAX_RECURSION_DEPTH,
            error::RecursionLimitSnafu {
                location: at.meta.location.clone(),
                limit: MAX_RECURSION_DEPTH,
            }
        );
        self.recursion_depth += 1;

        let mut out = String::new();
        for part in parts {
            match part {
                TemplatePart::Literal(text) => out.push_str(text),
                TemplatePart::Placeholder(segments, location) => {
                    let resolved = self.resolve_reference(at, segments, visit_log)?;
                    // Placeholder targets carry the template's own UID after
                    // resolution, so the generic resolve_value visit-log check
                    // would misfire; nested templates recurse here instead.
                    let rendered = match resolved.data {
                        Data::Template(nested) => self.resolve_template(at, &nested, visit_log)?,
                        _ => resolved,
                    };
                    out.push_str(&rendered.render_scalar(location)?);
                }
            }
        }
        self.recursion_depth -= 1;

        Ok(Value {
            uid: at.uid,
            data: Data::String(out),
            meta: at.meta.clone(),
        })
    }

    /// Resolves all references in a value
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
            Data::Reference(segments) => self.resolve_reference(at, segments, visit_log)?,
            Data::Template(parts) => self.resolve_template(at, parts, visit_log)?,
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

    /// Looks up a value by structured reference segments (root-relative),
    /// applying the same selector-kind rules as reference resolution.
    pub fn lookup(&self, segments: &[Segment]) -> Option<&Value> {
        self.reference_matches(segments).first().copied()
    }

    /// Looks up a value by its full structured symbol-table path (including
    /// the leading root segment).
    pub fn lookup_segments(&self, path: &[Segment]) -> Option<&Value> {
        self.symbol_table.get(path)
    }

    /// Returns all available root-relative paths as display strings.
    /// Note that dotted display is for diagnostics only; two
    /// distinct structured paths may render identically.
    pub fn available_paths(&self) -> Vec<String> {
        self.symbol_table
            .keys()
            .filter(|path| path.len() > 1)
            .map(|path| Segment::display_path(&path[1..]))
            .collect()
    }

    /// Validates that all references can be resolved
    pub fn validate_references(&self) -> Result<()> {
        let references: Vec<Value> = self
            .symbol_table
            .values()
            .filter(|value| matches!(value.data, Data::Reference(_) | Data::Template(_)))
            .cloned()
            .collect();

        let mut scope = Self {
            root: self.root.clone(),
            symbol_table: self.symbol_table.clone(),
            path_lookup: IndexMap::new(),
            recursion_depth: 0,
        };

        for value in references {
            match &value.data {
                Data::Reference(segments) => {
                    let mut visit_log = IndexSet::new();
                    scope.resolve_reference(&value, segments, &mut visit_log)?;
                }
                Data::Template(_) => {
                    let mut visit_log = IndexSet::new();
                    scope.resolve_value(&value, &mut visit_log)?;
                }
                _ => {}
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
        assert!(
            scope
                .lookup(&[Segment::Id("test_var".to_string())])
                .is_some()
        );
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

        // Create a reference that targets the target value
        let reference_value =
            Value::new_reference(vec![Segment::Id("target".to_string())], meta.clone());
        let reference_stmt =
            Statement::new_assign("reference", None, reference_value, meta.clone()).unwrap();
        children.insert("reference".to_string(), reference_stmt);

        let module = Statement::new_module("root", children, meta);
        let mut scope = Scope::new(&module);

        let resolved = scope.apply().unwrap();
        let resolved_reference = resolved.find_by_path("reference").unwrap();

        if let Some(resolved_value) = resolved_reference.get_value() {
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

        // Reference lookup matches label components in order via quoted selectors
        assert_eq!(
            scope
                .lookup(&[
                    Segment::Id("app".into()),
                    Segment::Key("a".into()),
                    Segment::Key("b".into()),
                    Segment::Id("enabled".into()),
                ])
                .unwrap()
                .as_bool(),
            Some(&false)
        );

        // A quoted dotted label stays a single component
        assert_eq!(
            scope
                .lookup(&[
                    Segment::Id("app".into()),
                    Segment::Key("a.b".into()),
                    Segment::Id("enabled".into()),
                ])
                .unwrap()
                .as_bool(),
            Some(&true)
        );
    }
}
