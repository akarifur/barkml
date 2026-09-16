//! End-to-end tests for dependency-cycle detection with structured traces
//! (docs/10).

use barkml::{Error, Loader, Scope, StandardLoader, Statement, from_str};
use std::io::Cursor;

fn resolve(input: &str) -> Statement {
    from_str(input).expect("parse and resolve")
}

fn value_of(stmt: &Statement, path: &str) -> barkml::Value {
    stmt.find_by_path(path)
        .unwrap_or_else(|| panic!("missing statement at {path}"))
        .get_value()
        .expect("statement has no value")
        .clone()
}

/// Parses without resolving references so direct `Scope` APIs can be exercised.
fn parse_unresolved(input: &str) -> Statement {
    let mut cursor = Cursor::new(input.as_bytes());
    StandardLoader::default()
        .add_module("main", &mut cursor, None)
        .expect("parse module")
        .skip_macro_resolution()
        .expect("disable resolution")
        .load()
        .expect("load unresolved")
}

fn expect_cycle(input: &str) -> (String, Vec<String>, Vec<barkml::Location>) {
    let err = from_str(input).unwrap_err();
    match err {
        Error::Cycle {
            target,
            trace,
            edge_locations,
            ..
        } => (target, trace, edge_locations),
        other => panic!("expected cycle error, got: {other}"),
    }
}

#[test]
fn direct_self_reference_is_a_cycle() {
    let (target, trace, edges) = expect_cycle("a {\n    value = a.value\n}");
    assert_eq!(target, "a.value");
    assert_eq!(trace, vec!["a.value", "a.value"]);
    // One edge closes the loop: edge_locations[i] is edge trace[i] -> trace[i+1]
    assert_eq!(edges.len(), trace.len() - 1);
}

#[test]
fn two_node_cycle_reports_closed_trace() {
    let (target, trace, edges) =
        expect_cycle("a {\n    value = b.value\n}\nb {\n    value = a.value\n}");
    // `a` is entered first, so the cycle starts and ends at `a.value`
    assert_eq!(target, "a.value");
    assert_eq!(trace, vec!["a.value", "b.value", "a.value"]);
    // Two edges: a -> b and the back-edge b -> a
    assert_eq!(edges.len(), 2);
    assert!(
        edges
            .iter()
            .all(|location| location.to_string().contains('@'))
    );
}

#[test]
fn longer_cycle_reports_ordered_nodes() {
    let (target, trace, _) = expect_cycle(
        "a {\n    value = b.value\n}\nb {\n    value = c.value\n}\nc {\n    value = a.value\n}",
    );
    assert_eq!(target, "a.value");
    assert_eq!(trace, vec!["a.value", "b.value", "c.value", "a.value"]);
}

#[test]
fn noncyclic_prefix_into_cycle_reports_only_the_cycle() {
    // `entry` is a legitimate prefix; the error trace must start at the
    // first node of the actual cycle
    let (target, trace, _) =
        expect_cycle("entry = a.value\na {\n    value = b.value\n}\nb {\n    value = a.value\n}");
    assert_eq!(target, "a.value");
    assert_eq!(trace, vec!["a.value", "b.value", "a.value"]);
}

#[test]
fn cycle_detection_is_independent_of_declaration_order() {
    // Declaring `b` first enters the cycle through `b.value`; both orders
    // must produce a typed cycle error with a closed trace
    let (target, trace, _) =
        expect_cycle("b {\n    value = a.value\n}\na {\n    value = b.value\n}");
    assert_eq!(target, "b.value");
    assert_eq!(trace, vec!["b.value", "a.value", "b.value"]);
}

#[test]
fn table_entry_cycle_is_detected() {
    let (target, trace, _) = expect_cycle("vars {\n    t = { inner = vars.t.inner }\n}");
    assert_eq!(target, "vars.t.inner");
    assert_eq!(trace, vec!["vars.t.inner", "vars.t.inner"]);
}

#[test]
fn array_element_cycle_is_detected() {
    let (target, trace, _) = expect_cycle("vars {\n    items = [vars.items[0]]\n}");
    assert_eq!(target, "vars.items[0]");
    assert_eq!(trace, vec!["vars.items[0]", "vars.items[0]"]);
}

#[test]
fn mixed_reference_and_interpolation_cycle_is_detected() {
    let (target, trace, _) = expect_cycle("out = vars.a\nvars {\n    a = f\"{out}\"\n}");
    assert_eq!(target, "out");
    assert_eq!(trace, vec!["out", "vars.a", "out"]);
}

#[test]
fn same_display_spelling_in_different_scopes_is_not_a_cycle() {
    // Both references render identically (`y.value`); they are distinct
    // dependencies only through different referring sites
    let module =
        resolve("y {\n    value = \"v\"\n}\nx {\n    a = y.value\n}\nz {\n    a = y.value\n}");
    assert_eq!(value_of(&module, "x.a").as_string(), Some(&"v".to_string()));
    assert_eq!(value_of(&module, "z.a").as_string(), Some(&"v".to_string()));
}

#[test]
fn distinct_dotted_labels_are_distinct_dependencies() {
    let module = resolve(
        "app \"a.b\" {\n    v = \"one\"\n}\napp \"a\" \"b\" {\n    v = \"two\"\n}\nfirst = app[\"a.b\"].v\nsecond = app[\"a\", \"b\"].v",
    );
    assert_eq!(
        value_of(&module, "first").as_string(),
        Some(&"one".to_string())
    );
    assert_eq!(
        value_of(&module, "second").as_string(),
        Some(&"two".to_string())
    );
}

#[test]
fn distinct_array_elements_do_not_collapse_into_one_identity() {
    let module = resolve("items = [\"zero\", \"one\"]\na = items[0]\nb = items[1]");
    assert_eq!(
        value_of(&module, "a").as_string(),
        Some(&"zero".to_string())
    );
    assert_eq!(value_of(&module, "b").as_string(), Some(&"one".to_string()));
}

#[test]
fn distinct_table_entries_do_not_collapse_into_one_identity() {
    let module = resolve("cfg = {\n    host = \"h\"\n    port = 1\n}\na = cfg.host\nb = cfg.port");
    assert_eq!(value_of(&module, "a").as_string(), Some(&"h".to_string()));
    assert_eq!(*value_of(&module, "b").as_int().unwrap(), 1);
}

#[test]
fn diamond_dependencies_resolve() {
    let module = resolve(
        "shared {\n    base = \"s\"\n}\nleft = shared.base\nright = shared.base\njoined = f\"{left}+{right}\"",
    );
    assert_eq!(
        value_of(&module, "left").as_string(),
        Some(&"s".to_string())
    );
    assert_eq!(
        value_of(&module, "right").as_string(),
        Some(&"s".to_string())
    );
    assert_eq!(
        value_of(&module, "joined").as_string(),
        Some(&"s+s".to_string())
    );
}

#[test]
fn diamond_works_regardless_of_dependency_declaration_order() {
    let module = resolve(
        "left = shared.base\nright = shared.base\nshared {\n    base = \"s\"\n}\njoined = f\"{left}+{right}\"",
    );
    assert_eq!(
        value_of(&module, "joined").as_string(),
        Some(&"s+s".to_string())
    );
}

#[test]
fn repeated_interpolation_of_one_dependency_resolves() {
    let module = resolve("name = \"bark\"\na = f\"{name} {name}\"\nb = f\"{name}!\"");
    assert_eq!(
        value_of(&module, "a").as_string(),
        Some(&"bark bark".to_string())
    );
    assert_eq!(
        value_of(&module, "b").as_string(),
        Some(&"bark!".to_string())
    );
}

#[test]
fn reuse_of_already_resolved_container_resolves() {
    let module = resolve("cfg = {\n    host = \"h\"\n    port = 5432\n}\none = cfg\ntwo = cfg");
    let one = value_of(&module, "one");
    let two = value_of(&module, "two");
    assert_eq!(
        one.as_table().unwrap().get("host").unwrap().as_string(),
        Some(&"h".to_string())
    );
    assert_eq!(
        two.as_table().unwrap().get("port").unwrap().as_int(),
        Some(&5432)
    );
}

#[test]
fn missing_paths_stay_missing_reference_errors() {
    let err = from_str("a {\n    value = nonexistent.value\n}").unwrap_err();
    assert!(
        matches!(err, Error::UnknownReference { .. }),
        "expected unknown reference, got: {err}"
    );
}

#[test]
fn long_acyclic_chain_hits_the_configured_limit_not_a_cycle() {
    // Declared head-first so each dependency expands before its target is
    // cached; a genuinely acyclic chain grows the expansion stack
    let mut input = String::new();
    for i in (1..200).rev() {
        input.push_str(&format!("a{i} = a{}\n", i - 1));
    }
    input.push_str("a0 = \"leaf\"\n");
    let err = from_str(&input).unwrap_err();
    match err {
        Error::RecursionLimit { limit, .. } => assert_eq!(limit, 100),
        other => panic!("expected recursion limit, got: {other}"),
    }
}

#[test]
fn ordered_acyclic_chain_stays_shallow_through_reuse() {
    // Leaf-first declaration lets each statement reuse the cached result of
    // the previous one; a 200-link chain resolves without hitting the limit
    let mut input = String::from("a0 = \"leaf\"\n");
    for i in 1..200 {
        input.push_str(&format!("a{i} = a{}\n", i - 1));
    }
    let module = from_str(&input).expect("ordered chain resolves");
    assert_eq!(
        value_of(&module, "a199").as_string(),
        Some(&"leaf".to_string())
    );
}

#[test]
fn short_visible_cycle_wins_over_the_limit() {
    // A tight limit cannot mask a visible back-edge: the cycle check runs
    // before another expansion is charged
    let unresolved = parse_unresolved("a {\n    value = b.value\n}\nb {\n    value = a.value\n}");
    let mut scope = Scope::with_limit(&unresolved, 2);
    match scope.apply() {
        Err(Error::Cycle { target, trace, .. }) => {
            assert_eq!(target, "a.value");
            assert_eq!(trace, vec!["a.value", "b.value", "a.value"]);
        }
        other => panic!("expected cycle error even at limit 2, got: {other:?}"),
    }
}

#[test]
fn custom_limit_via_scope_is_honored() {
    // Head-first 9-link chain with a limit of 5: expansions nest past 5
    let mut input = String::from("a9 = a8\n");
    for i in (1..9).rev() {
        input.push_str(&format!("a{i} = a{}\n", i - 1));
    }
    input.push_str("a0 = \"leaf\"\n");
    let unresolved = parse_unresolved(&input);
    let mut scope = Scope::with_limit(&unresolved, 5);
    match scope.apply() {
        Err(Error::RecursionLimit { limit, .. }) => assert_eq!(limit, 5),
        other => panic!("expected recursion limit at 5, got: {other:?}"),
    }
}

#[test]
fn load_validate_and_direct_resolver_agree_on_cycles() {
    let input = "a {\n    value = b.value\n}\nb {\n    value = a.value\n}";
    assert!(from_str(input).is_err());

    let unresolved = parse_unresolved(input);
    let mut scope = Scope::new(&unresolved);
    let outcome = scope.apply();
    assert!(
        matches!(outcome, Err(Error::Cycle { .. })),
        "apply must report the cycle, got: {:?}",
        outcome.map(|_| ())
    );
    assert!(
        matches!(scope.validate_references(), Err(Error::Cycle { .. })),
        "validate must report the cycle"
    );
}

#[test]
fn load_validate_and_direct_resolver_agree_on_success() {
    let input = "shared {\n    base = \"s\"\n}\nleft = shared.base\nright = shared.base";
    let resolved = resolve(input);
    assert_eq!(
        value_of(&resolved, "left").as_string(),
        Some(&"s".to_string())
    );

    let unresolved = parse_unresolved(input);
    let scope = Scope::new(&unresolved);
    scope.validate_references().expect("validation passes");
}

#[test]
fn repeated_apply_calls_stay_clean() {
    let input = "shared { base = \"s\" }\nleft = shared.base\nright = shared.base";
    let unresolved = parse_unresolved(input);
    let mut scope = Scope::new(&unresolved);
    let first = scope.apply().expect("first apply");
    let second = scope
        .apply()
        .expect("second apply must not see stale state");
    assert_eq!(
        value_of(&first, "left").as_string(),
        value_of(&second, "left").as_string()
    );
}

#[test]
fn retry_after_failure_does_not_report_stale_state() {
    let bad = "a {\n    value = b.value\n}\nb {\n    value = a.value\n}";
    let unresolved = parse_unresolved(bad);
    let mut scope = Scope::new(&unresolved);
    assert!(matches!(scope.apply(), Err(Error::Cycle { .. })));
    // A retry on a fresh scope with the good input succeeds; the failed
    // traversal must not have poisoned anything
    let good = parse_unresolved("x { v = \"leaf\" }\ny = x.v");
    let mut good_scope = Scope::new(&good);
    let resolved = good_scope.apply().expect("retry resolves");
    assert_eq!(
        value_of(&resolved, "y").as_string(),
        Some(&"leaf".to_string())
    );
}

#[test]
fn composed_sources_form_a_valid_forward_dependency() {
    let mut one = Cursor::new(b"vars {\n    editor = \"nvim\"\n}".as_ref());
    let mut two = Cursor::new(b"settings {\n    editor = vars.editor\n}".as_ref());
    let module = StandardLoader::default()
        .add_module("main", &mut one, None)
        .expect("parse first source")
        .add_module("main", &mut two, None)
        .expect("parse and compose second source")
        .load()
        .expect("composed forward dependency resolves");
    assert_eq!(
        value_of(&module, "settings.editor").as_string(),
        Some(&"nvim".to_string())
    );
}

#[test]
fn composed_sources_expose_a_cycle() {
    let mut one = Cursor::new(b"settings {\n    value = a.value\n}".as_ref());
    let mut two = Cursor::new(b"a {\n    value = settings.value\n}".as_ref());
    let err = StandardLoader::default()
        .add_module("main", &mut one, None)
        .expect("parse first source")
        .add_module("main", &mut two, None)
        .expect("parse and compose second source")
        .load()
        .unwrap_err();
    match err {
        Error::Cycle { target, trace, .. } => {
            assert_eq!(target, "settings.value");
            assert_eq!(trace, vec!["settings.value", "a.value", "settings.value"]);
        }
        other => panic!("expected cycle error, got: {other}"),
    }
}
