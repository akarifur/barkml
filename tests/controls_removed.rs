//! End-to-end tests for removal of `$` control statements and BMLS surface (docs/06).

use barkml::from_str;

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
fn legacy_control_statements_are_rejected_clearly() {
    for input in ["$foo = 3", "$schema = !Stead 1.0.0", "$x: f64 = 3.14"] {
        let err = match from_str(input) {
            Ok(_) => panic!("expected rejection of {input:?}"),
            Err(e) => format!("{e}"),
        };
        assert!(
            err.contains("control statements"),
            "error should mention removed control syntax, got: {err}"
        );
    }
}

#[test]
fn metadata_uses_ordinary_blocks() {
    let module = resolve(
        "stead {
            schema = 1.0.0
        }",
    );
    assert_eq!(
        value_of(&module, "stead.schema").as_version(),
        Some(&semver::Version::new(1, 0, 0))
    );
}

#[test]
fn literal_dollar_text_in_strings_is_preserved() {
    let module = resolve(
        "config {
            home = \"$HOME/.local/bin:$PATH\"
            greeting = \"hello ${name}\"
        }",
    );
    assert_eq!(
        value_of(&module, "config.home").as_string(),
        Some(&"$HOME/.local/bin:$PATH".to_string())
    );
    assert_eq!(
        value_of(&module, "config.greeting").as_string(),
        Some(&"hello ${name}".to_string())
    );
}

#[test]
fn no_stead_specific_names_are_reserved() {
    let module = resolve(
        "stead {
            anything = \"else\"
        }
        other {
            stead = 42
        }",
    );
    assert_eq!(
        value_of(&module, "stead.anything").as_string(),
        Some(&"else".to_string())
    );
    assert_eq!(value_of(&module, "other.stead").as_int(), Some(&42));
}

#[test]
fn schema_as_ordinary_assignment() {
    let module = resolve("schema = 1");
    assert_eq!(value_of(&module, "schema").as_int(), Some(&1));
}

#[test]
fn schema_as_block_name() {
    let module = resolve(
        "schema {
            ver = \"1.0.0\"
        }",
    );
    assert_eq!(
        value_of(&module, "schema.ver").as_string(),
        Some(&"1.0.0".to_string())
    );
}

#[test]
fn schema_as_table_key() {
    let module = resolve(
        "data {
            tbl = {
                schema = \"v2\"
            }
        }",
    );
    let table = value_of(&module, "data.tbl");
    assert_eq!(
        table.as_table().unwrap().get("schema").unwrap().as_string(),
        Some(&"v2".to_string())
    );
}

#[test]
fn schema_as_reference_path_component() {
    let module = resolve(
        "schema {
            ver = 3
        }
        app {
            current = schema.ver
        }",
    );
    assert_eq!(value_of(&module, "app.current").as_int(), Some(&3));
}

#[test]
fn labels_and_type_hints_still_work() {
    let module = resolve(
        "meta {
            tire: version = !Test 1.0.0
            ratio: f64 = 3.14
        }",
    );
    assert_eq!(
        value_of(&module, "meta.tire").as_version(),
        Some(&semver::Version::new(1, 0, 0))
    );
    assert_eq!(value_of(&module, "meta.ratio").as_f64(), Some(&3.14));
}
