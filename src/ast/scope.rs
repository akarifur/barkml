use super::types::{Location, StatementType, ValueType};
use super::value::escape_string;
use super::{Data, Statement, StatementData, TemplatePart, Value};
use crate::{Result, error};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use snafu::ensure;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Default maximum expansion depth for reference resolution when no
/// explicit limit is configured; guards long acyclic dependency chains.
///
/// Single source of truth for the default limit: the `Loader` trait
/// default and `LoaderConfig::default` derive from this constant.
pub const DEFAULT_RECURSION_LIMIT: usize = 100;

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
                        .map(|x| format!("\"{}\"", escape_string(&x)))
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

    /// Maximum expansion depth for reference resolution; guards long
    /// acyclic dependency chains separately from cycle detection
    recursion_limit: usize,
}

/// Traversal state for cycle-aware resolution.
///
/// Every dependency node is unseen, active on the current traversal stack,
/// or successfully resolved. Dependency identity is the target's canonical
/// symbol-table path: an edge back to an active node is a cycle, while an
/// edge to a resolved node is valid reuse. Only successful results are
/// cached, and active state is unwound on every error path.
struct Traversal {
    /// Active dependency nodes in visit order; the position of each node's
    /// first occurrence in a cycle is recovered from `index`
    stack: Vec<Frame>,
    /// Path -> position on the active stack
    index: HashMap<Vec<Segment>, usize>,
    /// Successfully resolved nodes keyed by canonical path, with the
    /// dependency height reached during their expansion (the number of
    /// reference edges in their longest dependency chain). Heights are
    /// charged on reuse so a cached target cannot let an over-limit chain
    /// bypass the configured bound, keeping outcomes independent of
    /// declaration order and prior resolution.
    resolved: IndexMap<Vec<Segment>, (Value, usize)>,
}

/// One active dependency node and the reference edge that led to it.
struct Frame {
    path: Vec<Segment>,
    edge: Location,
}

impl Traversal {
    fn new() -> Self {
        Self {
            stack: Vec::new(),
            index: HashMap::new(),
            resolved: IndexMap::new(),
        }
    }

    /// Position of `path` on the active stack, if it is currently active.
    fn active_position(&self, path: &[Segment]) -> Option<usize> {
        self.index.get(path).copied()
    }

    /// Marks `path` active before descending into its dependencies.
    /// Returns the stack mark to unwind/settle to.
    fn push(&mut self, path: &[Segment], edge: Location) -> usize {
        let mark = self.stack.len();
        self.index.insert(path.to_vec(), mark);
        self.stack.push(Frame {
            path: path.to_vec(),
            edge,
        });
        mark
    }

    /// Unwinds to `mark` after a failed expansion. Nothing is cached, so
    /// a retry starts from clean state.
    fn unwind(&mut self, mark: usize) {
        while self.stack.len() > mark {
            if let Some(frame) = self.stack.pop() {
                self.index.remove(&frame.path);
            }
        }
    }

    /// Pops the frame pushed at `mark` after a successful expansion and
    /// caches the resolved value and its dependency height for valid reuse
    /// by later references.
    fn settle(&mut self, mark: usize, value: Value, height: usize) {
        let frame = self.stack.pop().expect("settle mark must be on the stack");
        debug_assert_eq!(self.stack.len(), mark);
        self.index.remove(&frame.path);
        self.resolved.insert(frame.path, (value, height));
    }
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
    /// Creates a new Scope from a root statement with the default
    /// expansion-depth limit.
    pub fn new(node: &Statement) -> Self {
        Self::with_limit(node, DEFAULT_RECURSION_LIMIT)
    }

    /// Creates a new Scope with an explicit expansion-depth limit. The
    /// limit guards long acyclic dependency chains; cycles are detected by
    /// active back-edges regardless of the limit.
    pub fn with_limit(node: &Statement, limit: usize) -> Self {
        let mut scope = Self {
            root: node.clone(),
            symbol_table: IndexMap::new(),
            path_lookup: IndexMap::new(),
            recursion_limit: limit,
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

    /// Finds the symbol-table entries whose path (excluding the root
    /// segment) is addressable by the given reference segments, with both
    /// path and value. Selector kinds are enforced: `Key` addresses
    /// identifiers and labels, `Index` addresses array elements only.
    /// Multiple matches indicate an ambiguous selector.
    ///
    /// This is a linear scan filtered by exact path length; if resolution
    /// of very large symbol tables ever becomes hot, index paths by length
    /// (e.g. `HashMap<usize, Vec<&Path>>`) before matching.
    fn reference_matches(&self, segments: &[Segment]) -> Vec<(&Vec<Segment>, &Value)> {
        self.symbol_table
            .iter()
            .filter(|(path, _)| {
                path.len() == segments.len() + 1
                    && path[1..]
                        .iter()
                        .zip(segments)
                        .all(|(segment, selector)| segment.accepts(selector))
            })
            .collect()
    }

    /// Applies reference resolution to the entire scope. Each `apply` call
    /// starts from fresh traversal state, so repeated and retried calls
    /// never observe stale active-state cycles.
    pub fn apply(&mut self) -> Result<Statement> {
        let root = self.root.clone();
        let mut traversal = Traversal::new();
        self.resolve_statement(&root, Vec::new(), &mut traversal)
    }

    /// Resolves a structured reference to its actual value, preserving the
    /// target's type and data. All lookups are root-relative; `self`/`super`
    /// carry no special meaning.
    /// Resolves a structured reference to its actual value, preserving the
    /// target's type and data, and reports the target entry's dependency
    /// height (reference edges in its longest dependency chain). All
    /// lookups are root-relative; `self`/`super` carry no special meaning.
    fn resolve_reference(
        &mut self,
        at: &Value,
        segments: &[Segment],
        traversal: &mut Traversal,
    ) -> Result<(Value, usize)> {
        let matches = self.reference_matches(segments);
        if matches.len() > 1 {
            return error::WrongSelectorSnafu {
                location: at.meta.location.clone(),
                path: Segment::display_path(segments),
                reason: format!("selector matches {} distinct declarations", matches.len()),
            }
            .fail();
        }

        let found = matches
            .into_iter()
            .next()
            .map(|(path, value)| (path.clone(), value.clone()));
        match found {
            Some((path, target)) => self.resolve_entry(at, &path, &target, traversal, true),
            None => self.reference_failure(at, segments).map(|value| (value, 0)),
        }
    }

    /// Resolves the symbol-table entry at `path`, with `at` as the
    /// referring site (its UID and metadata are preserved on the result).
    ///
    /// Cycle, reuse, and depth are decided here, in that order: a back-edge
    /// to an active node is a cycle even at the depth limit; an edge to a
    /// resolved node reuses the cached result (charging its retained
    /// dependency height) rebuilt with the caller's identity; only
    /// otherwise is another expansion charged against the limit.
    ///
    /// Depth counts reference dependency edges: the root entry of a
    /// top-level statement (`via_reference == false`) is free, and each
    /// entry reached through a reference edge consumes one unit. A limit
    /// of `N` therefore permits an acyclic chain of exactly `N` edges and
    /// rejects one requiring `N + 1`, at the edge that would exceed it —
    /// regardless of declaration order or cache reuse.
    fn resolve_entry(
        &mut self,
        at: &Value,
        path: &[Segment],
        target: &Value,
        traversal: &mut Traversal,
        via_reference: bool,
    ) -> Result<(Value, usize)> {
        if let Some(start) = traversal.active_position(path) {
            return self.cycle_error(at, traversal, start, path).map(|v| (v, 0));
        }
        if let Some((cached, height)) = traversal.resolved.get(path) {
            // Deterministic reuse: a cached entry sits at depth
            // `stack.len()` (the root is at depth 0), so its subtree
            // reaches `depth + height`; permit while that stays in budget.
            ensure!(
                !via_reference || traversal.stack.len() + height <= self.recursion_limit,
                error::RecursionLimitSnafu {
                    location: at.meta.location.clone(),
                    limit: self.recursion_limit,
                    kind: "reference depth",
                }
            );
            return Ok((
                Value {
                    uid: at.uid,
                    data: cached.data.clone(),
                    meta: at.meta.clone(),
                },
                *height,
            ));
        }
        // Every pushed node after the root was entered through a reference
        // edge, so the edge count equals the stack length; taking another
        // edge is permitted while that count does not exceed the limit.
        ensure!(
            !via_reference || traversal.stack.len() <= self.recursion_limit,
            error::RecursionLimitSnafu {
                location: at.meta.location.clone(),
                limit: self.recursion_limit,
                kind: "reference depth",
            }
        );

        let mark = traversal.push(path, at.meta.location.clone());
        let result = self.resolve_value(target, traversal);
        match result {
            Ok((resolved, height)) => {
                traversal.settle(mark, resolved.clone(), height);
                Ok((
                    Value {
                        uid: at.uid,
                        data: resolved.data,
                        meta: at.meta.clone(),
                    },
                    height,
                ))
            }
            Err(err) => {
                traversal.unwind(mark);
                Err(err)
            }
        }
    }

    /// Builds the structured cycle error for a back-edge to the active
    /// node at stack position `start`: the closed trace in traversal order
    /// plus the source location of every edge forming the cycle.
    /// `edge_locations[i]` is the site of the reference edge
    /// `trace[i] -> trace[i + 1]`, so its length is `trace.len() - 1`.
    fn cycle_error(
        &self,
        at: &Value,
        traversal: &Traversal,
        start: usize,
        path: &[Segment],
    ) -> Result<Value> {
        let target = Segment::display_path(&path[1..]);
        let mut trace = Vec::with_capacity(traversal.stack.len() - start + 1);
        for frame in &traversal.stack[start..] {
            trace.push(Segment::display_path(&frame.path[1..]));
        }
        trace.push(target.clone());
        // The first frame's edge entered the cycle from outside; the
        // cycle's own edges are the ones that led between cycle nodes,
        // closing with the back-edge at the referring site
        let mut edge_locations = traversal.stack[start + 1..]
            .iter()
            .map(|frame| frame.edge.clone())
            .collect::<Vec<_>>();
        edge_locations.push(at.meta.location.clone());

        error::CycleSnafu {
            location: at.meta.location.clone(),
            target,
            trace,
            edge_locations,
        }
        .fail()
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

        if let Some((matched, value)) = deepest
            && 0 < matched
            && matched < segments.len()
        {
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

    /// Resolves all references in a statement. `path` mirrors the symbol
    /// table construction so each value entry is resolved at its canonical
    /// path (node-granular cycle and reuse identity).
    fn resolve_statement(
        &mut self,
        at: &Statement,
        path: Vec<Segment>,
        traversal: &mut Traversal,
    ) -> Result<Statement> {
        let mut node_path = path;
        node_path.push(Segment::Id(at.id.clone()));
        if let StatementData::Labeled(labels, _) = &at.data {
            for label in labels {
                if let Some(text) = label.as_string() {
                    node_path.push(Segment::Label(text.clone()));
                }
            }
        }

        match &at.type_ {
            StatementType::Module(_) => {
                let mut new_children = IndexMap::new();
                for (key, value) in at.get_grouped().unwrap() {
                    new_children.insert(
                        key.clone(),
                        self.resolve_statement(value, node_path.clone(), traversal)?,
                    );
                }

                Ok(Statement::new_module(&at.id, new_children, at.meta.clone()))
            }
            StatementType::Block { .. } => {
                let mut new_children = IndexMap::new();
                let mut new_labels = Vec::new();
                let (labels, children) = at.get_labeled().unwrap();

                for label in labels {
                    let (resolved, _) = self.resolve_value(label, traversal)?;
                    new_labels.push(resolved);
                }

                for (key, value) in children.iter() {
                    new_children.insert(
                        key.clone(),
                        self.resolve_statement(value, node_path.clone(), traversal)?,
                    );
                }

                Ok(Statement::new_block(
                    &at.id,
                    new_labels,
                    new_children,
                    at.meta.clone(),
                ))
            }
            StatementType::Assignment(expected) => {
                // Adopt the resolved target type only when the stored type was
                // derived from the parsed value (i.e. contains a reference);
                // declared types stay enforced against the resolved value
                let adopts_target = contains_reference_impl(expected);
                let value = at.get_value().unwrap();
                let (new_value, _) =
                    self.resolve_entry(value, &node_path, value, traversal, false)?;
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

                Statement::new_assign(&at.id, Some(expected.clone()), new_value, at.meta.clone())
            }
        }
    }

    /// Resolves an interpolated string: each placeholder is resolved with
    /// the same traversal state (cycle, reuse, depth, and missing-target
    /// semantics) as ordinary references and rendered under the scalar-only
    /// interpolation policy.
    /// Resolves an interpolated string: each placeholder is resolved with
    /// the same traversal state (cycle, reuse, depth, and missing-target
    /// semantics) as ordinary references and rendered under the scalar-only
    /// interpolation policy. Holes are siblings: the resulting height is
    /// the deepest hole chain, not their sum.
    fn resolve_template(
        &mut self,
        at: &Value,
        parts: &[TemplatePart],
        traversal: &mut Traversal,
    ) -> Result<(Value, usize)> {
        let mut out = String::new();
        let mut height = 0;
        for part in parts {
            match part {
                TemplatePart::Literal(text) => out.push_str(text),
                TemplatePart::Placeholder(segments, location) => {
                    let (resolved, hole_height) =
                        self.resolve_reference(at, segments, traversal)?;
                    out.push_str(&resolved.render_scalar(location)?);
                    height = height.max(hole_height + 1);
                }
            }
        }

        Ok((
            Value {
                uid: at.uid,
                data: Data::String(out),
                meta: at.meta.clone(),
            },
            height,
        ))
    }

    /// Resolves all references in a value. Dependency nodes are entered via
    /// `resolve_entry`; container children are part of their parent entry's
    /// expansion, so cycles and reuse are tracked at node granularity.
    /// Resolves all references in a value, returning the resolved value and
    /// its dependency height (the number of reference edges in its longest
    /// chain; literals have height 0). Dependency nodes are entered via
    /// `resolve_entry`; container children are part of their parent entry's
    /// expansion, so cycles and reuse are tracked at node granularity.
    fn resolve_value(&mut self, at: &Value, traversal: &mut Traversal) -> Result<(Value, usize)> {
        match &at.data {
            Data::Reference(segments) => {
                let (value, height) = self.resolve_reference(at, segments, traversal)?;
                Ok((value, height + 1))
            }
            Data::Template(parts) => self.resolve_template(at, parts, traversal),
            Data::Table(children) => {
                let uid = at.uid;
                let mut new_children = IndexMap::new();
                let mut height = 0;
                for (key, value) in children.iter() {
                    let (resolved, child_height) = self.resolve_value(value, traversal)?;
                    height = height.max(child_height);
                    new_children.insert(key.clone(), resolved);
                }
                Ok((
                    Value {
                        uid,
                        data: Data::Table(new_children),
                        meta: at.meta.clone(),
                    },
                    height,
                ))
            }
            Data::Array(children) => {
                let uid = at.uid;
                let mut new_children = Vec::new();
                let mut height = 0;
                for value in children.iter() {
                    let (resolved, child_height) = self.resolve_value(value, traversal)?;
                    height = height.max(child_height);
                    new_children.push(resolved);
                }
                Ok((
                    Value {
                        uid,
                        data: Data::Array(new_children),
                        meta: at.meta.clone(),
                    },
                    height,
                ))
            }
            _ => Ok((at.clone(), 0)),
        }
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
        self.reference_matches(segments)
            .first()
            .map(|(_, value)| *value)
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

    /// Validates that all references can be resolved without cycles by
    /// running the same traversal as `apply` and discarding the resolved
    /// output, so validation and evaluation cannot disagree.
    pub fn validate_references(&self) -> Result<()> {
        let mut scope = Self {
            root: self.root.clone(),
            symbol_table: self.symbol_table.clone(),
            path_lookup: IndexMap::new(),
            recursion_limit: self.recursion_limit,
        };
        scope.apply().map(|_| ())
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
