//! End-to-end tests for reference expressions (docs/03).

use barkml::{Segment, from_str};

fn resolve(input: &str) -> barkml::Statement {
    from_str(input).expect("parse and resolve")
}

fn value_of(stmt: &barkml::Statement, path: &str) -> barkml::Value {
    stmt.find_by_path(path)
        .unwrap_or_else(|| panic!("missing statement at {path}"))
        .get_value()
        .expect("statement has no value")
        .clone()
}

#[test]
fn reference_resolves_to_target_value() {
    let module = resolve(
        "vars {
            editor = \"nvim\"
        }
        settings {
            editor = vars.editor
        }",
    );
    assert_eq!(
        value_of(&module, "settings.editor").as_string(),
        Some(&"nvim".to_string())
    );
}

#[test]
fn forward_and_backward_references_are_equivalent() {
    let backward = resolve(
        "vars {
            editor = \"nvim\"
        }
        settings {
            editor = vars.editor
        }",
    );
    let forward = resolve(
        "settings {
            editor = vars.editor
        }
        vars {
            editor = \"nvim\"
        }",
    );
    assert_eq!(
        value_of(&backward, "settings.editor"),
        value_of(&forward, "settings.editor")
    );
}

#[test]
fn lookup_is_root_relative_not_lexical() {
    // A similarly named local declaration must not shadow the root path
    let module = resolve(
        "vars {
            name = \"root-name\"
        }
        settings {
            vars {
                name = \"inner-name\"
            }
            chosen = vars.name
        }",
    );
    assert_eq!(
        value_of(&module, "settings.chosen").as_string(),
        Some(&"root-name".to_string())
    );
}

#[test]
fn self_and_super_are_ordinary_identifiers() {
    let module = resolve(
        "self {
            value = \"via-self\"
        }
        super {
            value = \"via-super\"
        }
        settings {
            a = self.value
            b = super.value
        }",
    );
    assert_eq!(
        value_of(&module, "settings.a").as_string(),
        Some(&"via-self".to_string())
    );
    assert_eq!(
        value_of(&module, "settings.b").as_string(),
        Some(&"via-super".to_string())
    );
}

#[test]
fn references_in_arrays_and_tables() {
    let module = resolve(
        "vars {
            a = 1
            b = \"two\"
        }
        settings {
            list = [vars.a, vars.b]
            data = {
                first = vars.a
                nested = vars.b
            }
        }",
    );
    let list = value_of(&module, "settings.list");
    let items = list.as_array().unwrap();
    assert_eq!(items[0].as_int(), Some(&1));
    assert_eq!(items[1].as_string(), Some(&"two".to_string()));
    let tbl = value_of(&module, "settings.data");
    let table = tbl.as_table().unwrap();
    assert_eq!(table.get("first").unwrap().as_int(), Some(&1));
    assert_eq!(
        table.get("nested").unwrap().as_string(),
        Some(&"two".to_string())
    );
}

#[test]
fn references_preserve_widths_and_versions() {
    let module = resolve(
        "vars {
            small = 5u8
            big = 9007199254740993i64
            ver = 1.2.3
            req = ^1.2
        }
        settings {
            small = vars.small
            big = vars.big
            ver = vars.ver
            req = vars.req
        }",
    );
    assert_eq!(value_of(&module, "settings.small").as_u8(), Some(&5));
    assert_eq!(
        value_of(&module, "settings.big").as_i64(),
        Some(&9007199254740993i64)
    );
    assert_eq!(
        value_of(&module, "settings.ver").as_version(),
        Some(&semver::Version::new(1, 2, 3))
    );
    assert_eq!(
        value_of(&module, "settings.req").as_require(),
        Some(&semver::VersionReq::parse("^1.2").unwrap())
    );
}

#[test]
fn references_preserve_composite_values() {
    let module = resolve(
        "vars {
            data = {
                \"key.with.dots\" = 7
                plain = 8
            }
        }
        settings {
            copy = vars.data
            dotted = vars.data[\"key.with.dots\"]
        }",
    );
    let copy = value_of(&module, "settings.copy");
    let table = copy.as_table().unwrap();
    assert_eq!(table.get("key.with.dots").unwrap().as_int(), Some(&7));
    assert_eq!(table.get("plain").unwrap().as_int(), Some(&8));
    assert_eq!(value_of(&module, "settings.dotted").as_int(), Some(&7));
}

#[test]
fn multi_label_selectors_address_complete_sequences() {
    let module = resolve(
        "artifact \"linux\" \"aarch64\" {
            url = \"arm-build\"
        }
        artifact \"linux\" \"x86_64\" {
            url = \"amd-build\"
        }
        artifact \"linux\" {
            url = \"generic-linux\"
        }
        settings {
            arm = artifact[\"linux\", \"aarch64\"].url
            amd = artifact[\"linux\", \"x86_64\"].url
            generic = artifact[\"linux\"].url
        }",
    );
    assert_eq!(
        value_of(&module, "settings.arm").as_string(),
        Some(&"arm-build".to_string())
    );
    assert_eq!(
        value_of(&module, "settings.amd").as_string(),
        Some(&"amd-build".to_string())
    );
    assert_eq!(
        value_of(&module, "settings.generic").as_string(),
        Some(&"generic-linux".to_string())
    );
}

#[test]
fn quoted_dotted_label_is_a_single_component() {
    let module = resolve(
        "app \"a.b\" {
            enabled = true
        }
        app \"a\" \"b\" {
            enabled = false
        }
        settings {
            dotted = app[\"a.b\"].enabled
            split = app[\"a\", \"b\"].enabled
        }",
    );
    assert_eq!(value_of(&module, "settings.dotted").as_bool(), Some(&true));
    assert_eq!(value_of(&module, "settings.split").as_bool(), Some(&false));
}

#[test]
fn array_indices_are_zero_based() {
    let module = resolve(
        "items = [\"a\", \"b\", \"c\"]
        settings {
            first = items[0]
            last = items[2]
        }",
    );
    assert_eq!(
        value_of(&module, "settings.first").as_string(),
        Some(&"a".to_string())
    );
    assert_eq!(
        value_of(&module, "settings.last").as_string(),
        Some(&"c".to_string())
    );
}

#[test]
fn out_of_range_index_fails() {
    let err = from_str(
        "items = [\"a\"]
        settings {
            first = items[1]
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("index"),
        "unexpected error: {err}"
    );
}

#[test]
fn empty_array_index_fails() {
    let err = from_str(
        "items = []
        settings {
            first = items[0]
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("index"),
        "unexpected error: {err}"
    );
}

#[test]
fn quoted_numeric_key_does_not_index_arrays() {
    let err = from_str(
        "items = [\"a\"]
        settings {
            first = items[\"0\"]
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("selector"),
        "unexpected error: {err}"
    );
}

#[test]
fn numeric_index_on_non_array_fails() {
    let err = from_str(
        "vars {
            name = \"x\"
        }
        settings {
            broken = vars.name[0]
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("selector") || format!("{err}").contains("reference"),
        "unexpected error: {err}"
    );
}

#[test]
fn missing_reference_fails_with_location() {
    let err = from_str(
        "settings {
            editor = vars.editor
        }",
    )
    .unwrap_err();
    let message = format!("{err}");
    assert!(
        message.contains("vars.editor"),
        "unexpected error: {message}"
    );
    assert!(!message.is_empty());
}

#[test]
fn chained_references_and_shared_dependencies() {
    let module = resolve(
        "base {
            value = \"shared\"
        }
        middle {
            value = base.value
        }
        consumer-a {
            value = middle.value
        }
        consumer-b {
            value = base.value
        }",
    );
    for path in ["consumer-a.value", "consumer-b.value"] {
        assert_eq!(
            value_of(&module, path).as_string(),
            Some(&"shared".to_string())
        );
    }
}

#[test]
fn references_inside_copied_composites_resolve() {
    let module = resolve(
        "vars {
            x = \"v\"
        }
        base {
            data = {
                inner = vars.x
            }
        }
        settings {
            copy = base.data
        }",
    );
    let copy = value_of(&module, "settings.copy");
    let table = copy.as_table().unwrap();
    assert_eq!(
        table.get("inner").unwrap().as_string(),
        Some(&"v".to_string())
    );
}

#[test]
fn ambiguous_selectors_fail() {
    // x "k" { v = 1 } and x { "k" = { v = 2 } } are both addressable by
    // x["k"].v; the selector must be rejected as ambiguous, not silently
    // resolved by insertion order
    let err = from_str(
        "x \"k\" {
            v = 1
        }
        x {
            \"k\" = {
                v = 2
            }
        }
        settings {
            value = x[\"k\"].v
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("matches"),
        "unexpected error: {err}"
    );
}

#[test]
fn missing_head_reports_unknown_reference() {
    // A typo in the head segment must report a missing reference, not a
    // misleading index/selector error borrowed from an unrelated array
    let err = from_str(
        "items = [\"a\"]
        settings {
            value = nonexistent[9]
        }",
    )
    .unwrap_err();
    let message = format!("{err}");
    assert!(
        message.contains("nonexistent[9]"),
        "unexpected error: {message}"
    );
}

#[test]
fn declared_types_are_enforced_against_resolved_references() {
    let err = from_str(
        "vars {
            name = \"text\"
        }
        settings {
            count: int = vars.name
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("assign") || format!("{err}").contains("type"),
        "unexpected error: {err}"
    );
}

#[test]
fn reference_cycles_fail() {
    let err = from_str(
        "a {
            value = b.value
        }
        b {
            value = a.value
        }",
    )
    .unwrap_err();
    let message = format!("{err}");
    assert!(message.contains("loop"), "unexpected error: {message}");
}

#[test]
fn legacy_macro_forms_fail_with_migration_error() {
    for input in [
        "settings {\n    value = m!vars.editor\n}\n",
        "settings {\n    value = m'Hello {name}'\n}\n",
    ] {
        let err = from_str(input).unwrap_err();
        let message = format!("{err}");
        assert!(
            message.contains("legacy macro"),
            "unexpected error for {input:?}: {message}"
        );
    }
}

#[test]
#[allow(clippy::approx_constant)]
fn literals_keep_their_tokenization() {
    // Numeric suffixes, semver, requirements, booleans, and null must not
    // become references
    let module = resolve(
        "settings {
            small = 5u8
            wide = 3.14f32
            ver = 1.2.3
            req = >=1.2.0
            flag = true
            nothing = null
        }",
    );
    assert_eq!(value_of(&module, "settings.small").as_u8(), Some(&5));
    assert_eq!(value_of(&module, "settings.wide").as_f32(), Some(&3.14f32));
    assert_eq!(
        value_of(&module, "settings.ver").as_version(),
        Some(&semver::Version::new(1, 2, 3))
    );
    assert!(value_of(&module, "settings.req").as_require().is_some());
    assert_eq!(value_of(&module, "settings.flag").as_bool(), Some(&true));
    assert!(value_of(&module, "settings.nothing").is_null());
}

#[test]
fn parse_unresolved_then_resolve_explicitly() {
    use barkml::{Loader, Scope, StandardLoader};
    use std::io::Cursor;

    let mut cursor =
        Cursor::new(b"settings {\n    editor = vars.editor\n}\nvars {\n    editor = \"nvim\"\n}");
    // Parse without resolution
    let mut loader = StandardLoader::default();
    loader.add_module("main", &mut cursor, None).unwrap();
    let unresolved = loader.read().unwrap();
    assert!(
        unresolved
            .find_by_path("settings.editor")
            .and_then(|s| s.get_value())
            .map(|v| v.as_reference().is_some())
            .unwrap_or(false),
        "value should remain an unresolved reference"
    );

    // Compose/resolution against the supplied tree
    let mut scope = Scope::new(&unresolved);
    let resolved = scope.apply().unwrap();
    assert_eq!(
        value_of(&resolved, "settings.editor").as_string(),
        Some(&"nvim".to_string())
    );
    // Segment kind check on the unresolved AST
    let reference = unresolved
        .find_by_path("settings.editor")
        .unwrap()
        .get_value()
        .unwrap()
        .as_reference()
        .unwrap()
        .clone();
    assert_eq!(
        reference,
        vec![
            Segment::Id("vars".to_string()),
            Segment::Id("editor".to_string()),
        ]
    );
}
