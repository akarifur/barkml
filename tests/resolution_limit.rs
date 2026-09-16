//! Conformance tests for the configured reference-resolution depth limit
//! (docs/11).
//!
//! Depth is the number of reference dependency edges along the current
//! resolution chain: a limit of `N` permits an acyclic chain of exactly `N`
//! edges and rejects one requiring `N + 1`, at the edge that would exceed
//! the limit. All fixtures are lexically flat (nesting depth 1) so the
//! parser's independent 64-level nesting cap cannot confound results.

use barkml::{
    DEFAULT_RECURSION_LIMIT, Data, Error, Loader, LoaderConfig, Scope, StandardLoader,
    StandardLoaderBuilder, Statement,
};
use std::io::Cursor;

/// Builds a flat, head-first reference chain of exactly `edges` edges over
/// a literal leaf: `a{edges} = a{edges - 1}` ... `a0 = "leaf"`. Head-first
/// order forces full stack growth (no cache reuse shortens the chain).
fn chain(edges: usize) -> String {
    assert!(edges >= 1, "use literal_only() for zero-edge documents");
    let mut input = String::new();
    for i in (1..=edges).rev() {
        input.push_str(&format!("a{i} = a{}\n", i - 1));
    }
    input.push_str("a0 = \"leaf\"\n");
    input
}

fn literal_only() -> &'static str {
    "x = \"leaf\"\ny = 1\n"
}

/// Parses without resolving references so direct `Scope` APIs can be
/// exercised under explicit limits.
fn parse_unresolved(input: &str) -> Statement {
    let mut cursor = Cursor::new(input.as_bytes());
    let mut loader = StandardLoader::default();
    loader
        .add_module("main", &mut cursor, None)
        .expect("parse module");
    loader
        .skip_macro_resolution()
        .expect("disable resolution")
        .load()
        .expect("load unresolved")
}

/// A loader configured with `limit`, holding the parsed `input` as its
/// main module, ready for `load`/`validate` entry-point tests.
fn limited_loader(input: &str, limit: usize) -> StandardLoader {
    let mut cursor = Cursor::new(input.as_bytes());
    let mut loader = StandardLoaderBuilder::new()
        .max_recursion_depth(limit)
        .build();
    loader
        .add_module("main", &mut cursor, None)
        .expect("parse module");
    loader
}

/// Parses `input` without resolving references, via a loader configured
/// with `limit`.
fn unresolved_with_limit(input: &str, limit: usize) -> Statement {
    let mut loader = limited_loader(input, limit);
    loader
        .skip_macro_resolution()
        .expect("disable resolution")
        .load()
        .expect("load unresolved")
}

fn value_of(stmt: &Statement, path: &str) -> barkml::Value {
    stmt.find_by_path(path)
        .unwrap_or_else(|| panic!("missing statement at {path}"))
        .get_value()
        .expect("statement has no value")
        .clone()
}

fn expect_limit_error(input: &str, limit: usize) -> barkml::Location {
    let unresolved = parse_unresolved(input);
    let mut scope = Scope::with_limit(&unresolved, limit);
    match scope.apply() {
        Err(Error::RecursionLimit {
            limit: got,
            kind,
            location,
            ..
        }) => {
            assert_eq!(got, limit, "error must carry the configured limit");
            assert_eq!(kind, "reference depth");
            location
        }
        other => panic!("expected reference-depth limit error, got: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Boundary semantics for limits 0, 1, 2, and a custom value
// ---------------------------------------------------------------------------

#[test]
fn limit_zero_permits_literals_only() {
    let unresolved = parse_unresolved(literal_only());
    Scope::with_limit(&unresolved, 0)
        .apply()
        .expect("literal-only document needs no reference edges");

    // A single edge is already over a limit of 0; the rejected edge is the
    // reference on line 1
    let location = expect_limit_error("a = b\nb = \"leaf\"\n", 0);
    assert_eq!(location.line, 0, "offending edge is the first reference");
}

#[test]
fn limit_one_boundary() {
    Scope::with_limit(&parse_unresolved(&chain(1)), 1)
        .apply()
        .expect("chain of exactly 1 edge is permitted at limit 1");

    // Chain a2 = a1 = a0: resolving a2 pushes a2 (free root), the edge to
    // a1 is permitted, and the edge a1 -> a0 is the one rejected — the
    // failing reference site is a1's `= a0` at line 1
    let location = expect_limit_error(&chain(2), 1);
    assert_eq!(location.line, 1);
}

#[test]
fn limit_two_boundary() {
    Scope::with_limit(&parse_unresolved(&chain(2)), 2)
        .apply()
        .expect("chain of exactly 2 edges is permitted at limit 2");
    expect_limit_error(&chain(3), 2);
}

#[test]
fn custom_limit_five_boundary() {
    Scope::with_limit(&parse_unresolved(&chain(5)), 5)
        .apply()
        .expect("chain of exactly 5 edges is permitted at limit 5");
    expect_limit_error(&chain(6), 5);
}

#[test]
fn below_limit_chains_resolve_to_the_leaf() {
    for (edges, limit) in [(1, 2), (2, 5), (4, 5)] {
        let resolved = Scope::with_limit(&parse_unresolved(&chain(edges)), limit)
            .apply()
            .unwrap_or_else(|e| panic!("chain {edges} under limit {limit}: {e}"));
        assert_eq!(
            value_of(&resolved, &format!("a{edges}")).as_string(),
            Some(&"leaf".to_string())
        );
    }
}

// ---------------------------------------------------------------------------
// Default limit boundary: exactly 99, 100, and 101 edges
// ---------------------------------------------------------------------------

#[test]
fn default_limit_is_one_hundred_everywhere() {
    assert_eq!(DEFAULT_RECURSION_LIMIT, 100);
    assert_eq!(StandardLoader::default().max_recursion_depth(), 100);
    assert_eq!(LoaderConfig::default().max_recursion_depth, 100);
    assert_eq!(
        StandardLoaderBuilder::new().build().max_recursion_depth(),
        100
    );
}

#[test]
fn default_limit_allows_exactly_ninety_nine_and_one_hundred_edges() {
    for edges in [99usize, 100] {
        Scope::with_limit(&parse_unresolved(&chain(edges)), DEFAULT_RECURSION_LIMIT)
            .apply()
            .unwrap_or_else(|e| {
                panic!("chain of {edges} edges must resolve at the default limit: {e}")
            });
    }
}

#[test]
fn default_limit_rejects_one_hundred_one_edges() {
    let location = expect_limit_error(&chain(101), DEFAULT_RECURSION_LIMIT);
    // Head-first chain: the rejected edge is the 101st, whose source is
    // `a100 = a99` on document line 100 (0-based)
    assert_eq!(location.line, 100);
}

#[test]
fn default_limit_applies_to_from_str_resolution() {
    match barkml::from_str(&chain(101)).unwrap_err() {
        Error::RecursionLimit { limit, kind, .. } => {
            assert_eq!(limit, DEFAULT_RECURSION_LIMIT);
            assert_eq!(kind, "reference depth");
        }
        other => panic!("expected recursion limit, got: {other}"),
    }
}

#[test]
fn configured_limit_above_default_no_longer_loses_to_the_constant() {
    let loader = limited_loader(&chain(101), 120);
    let module = loader
        .load()
        .expect("101-edge chain resolves when the limit is raised to 120");
    assert_eq!(
        value_of(&module, "a101").as_string(),
        Some(&"leaf".to_string())
    );
}

// ---------------------------------------------------------------------------
// Entry-point parity: direct Scope, load, validate, builder
// ---------------------------------------------------------------------------

#[test]
fn load_validate_and_direct_scope_agree_on_the_limit() {
    let input = chain(6);

    match limited_loader(&input, 5).load() {
        Err(Error::RecursionLimit { limit, kind, .. }) => {
            assert_eq!(limit, 5);
            assert_eq!(kind, "reference depth");
        }
        other => panic!("load must enforce limit 5, got: {other:?}"),
    }

    match limited_loader(&input, 5).validate() {
        Err(Error::RecursionLimit { limit, .. }) => assert_eq!(limit, 5),
        other => panic!("validate must enforce limit 5, got: {other:?}"),
    }

    let unresolved = unresolved_with_limit(&input, 5);
    let mut scope = Scope::with_limit(&unresolved, 5);
    match scope.apply() {
        Err(Error::RecursionLimit { limit, .. }) => assert_eq!(limit, 5),
        other => panic!("direct apply must enforce limit 5, got: {other:?}"),
    }

    // The same document resolves everywhere at a sufficient limit
    limited_loader(&input, 6)
        .load()
        .expect("load succeeds at limit 6");
    limited_loader(&input, 6)
        .validate()
        .expect("validate succeeds at limit 6");
    Scope::with_limit(&unresolved_with_limit(&input, 6), 6)
        .apply()
        .expect("apply succeeds at limit 6");
}

#[test]
fn config_constructor_propagates_the_limit() {
    let config = LoaderConfig {
        max_recursion_depth: 5,
        ..LoaderConfig::default()
    };
    let input = chain(6);
    let mut cursor = Cursor::new(input.as_bytes());
    let mut loader = StandardLoader::new(config);
    loader
        .add_module("main", &mut cursor, None)
        .expect("parse module");
    match loader.load() {
        Err(Error::RecursionLimit { limit, kind, .. }) => {
            assert_eq!(limit, 5);
            assert_eq!(kind, "reference depth");
        }
        other => panic!("config-constructed loader must enforce limit 5: {other:?}"),
    }
}

#[test]
fn validate_references_uses_the_configured_limit() {
    let unresolved = parse_unresolved(&chain(3));
    let scope = Scope::with_limit(&unresolved, 2);
    match scope.validate_references() {
        Err(Error::RecursionLimit { limit, .. }) => assert_eq!(limit, 2),
        other => panic!("validate_references must enforce its scope limit: {other:?}"),
    }
    Scope::with_limit(&unresolved, 3)
        .validate_references()
        .expect("validation passes when the chain fits");
}

// ---------------------------------------------------------------------------
// Interpolation and container references
// ---------------------------------------------------------------------------

#[test]
fn interpolation_hole_counts_its_reference_edge() {
    // f-string with a literal-only body needs no expansion; one hole over a
    // literal target is exactly 1 edge and fails at limit 0
    Scope::with_limit(&parse_unresolved("a = f\"plain\"\nb = \"x\"\n"), 0)
        .apply()
        .expect("literal-only f-string needs no reference expansion");
    expect_limit_error("a = f\"{b}\"\nb = \"x\"\n", 0);
}

#[test]
fn reference_to_interpolation_chain_respects_limit() {
    // ref -> f-string -> ref chain of 2 edges
    let input = "a = b\nb = f\"{c}\"\nc = \"leaf\"\n";
    Scope::with_limit(&parse_unresolved(input), 2)
        .apply()
        .expect("ref-to-interpolation chain of 2 edges fits limit 2");
    expect_limit_error(input, 1);
}

#[test]
fn interpolation_to_reference_chain_respects_limit() {
    // f-string -> ref -> ref chain of 3 edges
    let input = "a = f\"{b}\"\nb = c\nc = d\nd = \"leaf\"\n";
    Scope::with_limit(&parse_unresolved(input), 3)
        .apply()
        .expect("interpolation-to-reference chain of 3 edges fits limit 3");
    expect_limit_error(input, 2);
}

#[test]
fn multiple_independent_holes_are_siblings_not_cumulative() {
    // Two holes over literal targets: each is a 1-edge sibling chain
    let input = "a = f\"{x}-{y}\"\nx = \"1\"\ny = \"2\"\n";
    let resolved = Scope::with_limit(&parse_unresolved(input), 1)
        .apply()
        .expect("independent holes must not accumulate depth");
    assert_eq!(
        value_of(&resolved, "a").as_string(),
        Some(&"1-2".to_string())
    );
}

#[test]
fn shared_and_diamond_dependencies_count_the_longest_chain_only() {
    // d -> b -> shared (2 edges); c -> shared (1 edge): diamond depth is 2
    let input = "shared = \"s\"\nb = shared\nc = shared\nd = f\"{b}+{c}\"\n";
    let resolved = Scope::with_limit(&parse_unresolved(input), 2)
        .apply()
        .expect("diamond depth is the longest chain, not the sum");
    assert_eq!(
        value_of(&resolved, "d").as_string(),
        Some(&"s+s".to_string())
    );
    expect_limit_error(input, 1);
}

#[test]
fn table_and_array_traversal_is_not_reference_depth() {
    // Traversing a table/array does not consume budget; the reference edge
    // into the table is the only charged edge
    let input = "cfg = {\n    host = \"h\"\n    items = [\"zero\", \"one\"]\n}\nhost = cfg.host\nfirst = cfg.items[0]\n";
    let resolved = Scope::with_limit(&parse_unresolved(input), 1)
        .apply()
        .expect("selector traversal is not reference depth");
    assert_eq!(
        value_of(&resolved, "host").as_string(),
        Some(&"h".to_string())
    );
    assert_eq!(
        value_of(&resolved, "first").as_string(),
        Some(&"zero".to_string())
    );
}

#[test]
fn chain_through_a_table_reference_counts_each_edge() {
    // t2 -> t1 -> cfg (2 reference edges); entering the table target does
    // not itself charge budget, the edges do
    let input = "cfg = { inner = \"v\" }\nt1 = cfg\nt2 = t1\n";
    Scope::with_limit(&parse_unresolved(input), 2)
        .apply()
        .expect("table-reference chain of 2 edges fits limit 2");
    expect_limit_error(input, 1);
}

// ---------------------------------------------------------------------------
// Declaration order, cache reuse, and failure isolation
// ---------------------------------------------------------------------------

#[test]
fn same_chain_same_outcome_regardless_of_declaration_order() {
    // Head-first forces nested expansion; leaf-first reuses cached results.
    // Both orders must succeed under the same limit and yield the leaf.
    let mut head_first = String::new();
    for i in (1..=8).rev() {
        head_first.push_str(&format!("a{i} = a{}\n", i - 1));
    }
    head_first.push_str("a0 = \"leaf\"\n");

    let mut leaf_first = String::from("a0 = \"leaf\"\n");
    for i in 1..=8 {
        leaf_first.push_str(&format!("a{i} = a{}\n", i - 1));
    }

    for (label, input) in [
        ("head-first", head_first.as_str()),
        ("leaf-first", leaf_first.as_str()),
    ] {
        let resolved = Scope::with_limit(&parse_unresolved(input), 8)
            .apply()
            .unwrap_or_else(|e| panic!("{label} order must resolve: {e}"));
        assert_eq!(
            value_of(&resolved, "a8").as_string(),
            Some(&"leaf".to_string())
        );
    }

    // And both orders fail identically one below the chain length
    for (label, input) in [
        ("head-first", head_first.as_str()),
        ("leaf-first", leaf_first.as_str()),
    ] {
        let unresolved = parse_unresolved(input);
        let mut scope = Scope::with_limit(&unresolved, 7);
        assert!(
            matches!(scope.apply(), Err(Error::RecursionLimit { limit: 7, .. })),
            "{label} order must fail at limit 7"
        );
    }
}

#[test]
fn skip_resolution_returns_unresolved_then_explicit_limit_applies() {
    let input = chain(3);
    let unresolved = parse_unresolved(&input);
    // The skip-resolution path must not have expanded references
    let raw = unresolved
        .find_by_path("a3")
        .and_then(|s| s.get_value().cloned())
        .expect("a3 exists");
    assert!(
        matches!(raw.data, Data::Reference(_)),
        "skip_macro_resolution must return unresolved references"
    );

    // A later explicit resolver call uses its own supplied limit
    Scope::with_limit(&unresolved, 3)
        .apply()
        .expect("explicit resolution succeeds at limit 3");
    match Scope::with_limit(&unresolved, 2).apply() {
        Err(Error::RecursionLimit { limit, .. }) => assert_eq!(limit, 2),
        other => panic!("explicit resolution must enforce limit 2: {other:?}"),
    }
}

#[test]
fn failed_limited_run_does_not_poison_a_larger_later_run() {
    let unresolved = parse_unresolved(&chain(3));
    let mut scope = Scope::with_limit(&unresolved, 1);
    assert!(
        matches!(scope.apply(), Err(Error::RecursionLimit { .. })),
        "first run exceeds limit 1"
    );
    // A fresh scope with a larger limit on the same statement succeeds:
    // no stale depth budget survives the failure
    let resolved = Scope::with_limit(&unresolved, 3)
        .apply()
        .expect("later run with a larger limit must succeed");
    assert_eq!(
        value_of(&resolved, "a3").as_string(),
        Some(&"leaf".to_string())
    );

    // An independent document is unaffected too
    let other = parse_unresolved("x = \"leaf\"\ny = x\n");
    Scope::with_limit(&other, 1)
        .apply()
        .expect("independent document resolves");
}

// ---------------------------------------------------------------------------
// Cycle detection stays separate; parser cap stays independent
// ---------------------------------------------------------------------------

#[test]
fn short_visible_cycle_wins_over_the_limit_error() {
    let unresolved = parse_unresolved("a = b\nb = a\n");
    let mut scope = Scope::with_limit(&unresolved, 2);
    match scope.apply() {
        Err(Error::Cycle { .. }) => {}
        other => panic!("visible back-edge must be a cycle, not a limit error: {other:?}"),
    }
}

#[test]
fn parser_nesting_cap_is_independent_of_the_loader_limit() {
    // 70 nested tables exceed the parser's fixed 64-level nesting cap; the
    // loader's reference-depth limit must not alter that
    let mut nested = String::new();
    for _ in 0..70 {
        nested.push_str("t = {\n");
    }
    nested.push_str("leaf = \"v\"\n");
    for _ in 0..70 {
        nested.push_str("}\n");
    }

    let mut cursor = Cursor::new(nested.into_bytes());
    #[allow(clippy::result_large_err)]
    let result = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let mut loader = StandardLoaderBuilder::new()
                .max_recursion_depth(200)
                .build();
            loader.add_module("main", &mut cursor, None).map(|_| ())
        })
        .expect("spawn probe thread")
        .join()
        .expect("probe thread");
    let err = result.expect_err("parser nesting must fail regardless of loader limit");
    match err {
        // add_module maps parse failures through Io; the nested cause text
        // identifies the parser-nesting limit and its fixed cap of 64
        Error::Io { reason } => {
            assert!(
                reason.contains("parser nesting limit exceeded"),
                "unexpected error: {reason}"
            );
            assert!(
                reason.contains("maximum depth of 64"),
                "unexpected error: {reason}"
            );
        }
        other => panic!("expected parser-nesting limit error, got: {other:?}"),
    }
}
