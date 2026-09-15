//! End-to-end tests for explicit string interpolation and the coherent
//! string model (docs/04).

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

fn string_of(stmt: &barkml::Statement, path: &str) -> String {
    value_of(stmt, path)
        .as_string()
        .cloned()
        .unwrap_or_else(|| panic!("expected string at {path}"))
}

#[test]
fn plain_strings_do_not_interpolate() {
    let module = resolve(
        "settings {
            value = \"echo $HOME and {name} and ${shell_var}\"
        }",
    );
    assert_eq!(
        string_of(&module, "settings.value"),
        "echo $HOME and {name} and ${shell_var}"
    );
}

#[test]
fn single_quoted_strings_stay_literal_compatibility_form() {
    let module = resolve(
        "settings {
            value = 'plain $HOME {braces} text'
        }",
    );
    assert_eq!(
        string_of(&module, "settings.value"),
        "plain $HOME {braces} text"
    );
}

#[test]
fn escapes_round_trip() {
    let module = resolve(
        r#"settings {
            value = "a\"b\\c\nd\re\tf\bg\ff é"
        }"#,
    );
    assert_eq!(
        string_of(&module, "settings.value"),
        "a\"b\\c\nd\re\tf\u{8}g\u{c}f é"
    );
}

#[test]
fn unicode_escape_round_trips() {
    let module = resolve(
        "settings {
            value = \"\\u{1F600}x\\u{41}\"
        }",
    );
    assert_eq!(string_of(&module, "settings.value"), "😀xA");
}

#[test]
fn invalid_escape_fails() {
    let err = from_str("settings { value = \"bad \\q escape\" }").unwrap_err();
    assert!(
        format!("{err}").contains("escape"),
        "unexpected error: {err}"
    );
}

#[test]
fn trailing_backslash_fails() {
    let err = from_str("settings { value = \"trailing \\\" }").unwrap_err();
    assert!(
        format!("{err}").contains("escape") || format!("{err}").contains("unterminated"),
        "unexpected error: {err}"
    );
}

#[test]
fn unterminated_single_line_string_fails() {
    let err = from_str("settings { value = \"no close }").unwrap_err();
    assert!(
        format!("{err}").contains("unterminated"),
        "unexpected error: {err}"
    );
}

#[test]
fn unterminated_multiline_string_fails() {
    let err = from_str("settings { value = \"\"\"never closed }").unwrap_err();
    assert!(
        format!("{err}").contains("unterminated"),
        "unexpected error: {err}"
    );
}

#[test]
fn multiline_string_is_verbatim() {
    let module = resolve(
        "script = \"\"\"\nexport EDITOR=\"nvim\"\nexport PATH=\"$HOME/.local/bin:$PATH\"\n\"\"\"",
    );
    assert_eq!(
        string_of(&module, "script"),
        "\nexport EDITOR=\"nvim\"\nexport PATH=\"$HOME/.local/bin:$PATH\"\n"
    );
}

#[test]
fn multiline_string_keeps_indentation_without_dedent() {
    let module = resolve("script = \"\"\"\n    indented line\n  \"\"\"");
    assert_eq!(string_of(&module, "script"), "\n    indented line\n  ");
}

#[test]
fn f_string_resolves_simple_placeholders() {
    let module = resolve(
        "host {
            config_dir = \"/etc/host\"
        }
        target = f\"{host.config_dir}/starship.toml\"",
    );
    assert_eq!(string_of(&module, "target"), "/etc/host/starship.toml");
}

#[test]
fn f_string_resolves_multiple_and_repeated_placeholders() {
    let module = resolve(
        "vars {
            a = \"one\"
            b = \"two\"
        }
        out = f\"{vars.a}+{vars.b}={vars.a}\"",
    );
    assert_eq!(string_of(&module, "out"), "one+two=one");
}

#[test]
fn f_string_supports_punctuation_bearing_selectors() {
    let module = resolve(
        "app \"org.mozilla.firefox\" {
            enabled = true
        }
        items = [\"zeroth\"]
        out = f\"{app['org.mozilla.firefox'].enabled}:{items[0]}\"",
    );
    assert_eq!(string_of(&module, "out"), "true:zeroth");
}

#[test]
fn f_string_multiline_resolves() {
    let module = resolve(
        "vars {
            editor = \"nvim\"
        }
        script = f\"\"\"\nexport EDITOR=\"{vars.editor}\"\n\"\"\"",
    );
    assert_eq!(string_of(&module, "script"), "\nexport EDITOR=\"nvim\"\n");
}

#[test]
fn literal_braces_use_doubled_form_in_f_strings() {
    let module = resolve(
        "vars {
            name = \"x\"
        }
        out = f\"{{literal}} {vars.name} {{}}\"",
    );
    assert_eq!(string_of(&module, "out"), "{literal} x {}");
}

#[test]
fn unmatched_brace_in_f_string_fails() {
    let err = from_str("vars { name = \"x\" } out = f\"{vars.name\"").unwrap_err();
    assert!(
        format!("{err}").contains("placeholder") || format!("{err}").contains("expected"),
        "unexpected error: {err}"
    );
}

#[test]
fn unmatched_closing_brace_in_f_string_fails() {
    let err = from_str("out = f\"a}\"").unwrap_err();
    assert!(
        format!("{err}").contains("placeholder") || format!("{err}").contains("'}'"),
        "unexpected error: {err}"
    );
}

#[test]
fn empty_placeholder_fails() {
    let err = from_str("out = f\"{}\"").unwrap_err();
    assert!(
        format!("{err}").contains("placeholder"),
        "unexpected error: {err}"
    );
}

#[test]
fn typed_numbers_render_without_width_suffix() {
    let module = resolve(
        "nums {
            small = 5i8
            big = 1000u64
            ratio = 1.5f32
        }
        out = f\"{nums.small}|{nums.big}|{nums.ratio}\"",
    );
    assert_eq!(string_of(&module, "out"), "5|1000|1.5");
}

#[test]
fn bools_null_and_versions_render_canonically() {
    let module = resolve(
        "misc {
            flag = false
            nothing = null
            ver = 1.2.3
            req = ^1.0.0
        }
        out = f\"{misc.flag}|{misc.nothing}|{misc.ver}|{misc.req}\"",
    );
    assert_eq!(string_of(&module, "out"), "false|null|1.2.3|^1.0.0");
}

#[test]
fn arrays_tables_bytes_and_symbols_are_rejected_in_placeholders() {
    let cases = [
        ("items = [1] out = f\"{items}\"", "array"),
        ("t = { a = 1 } out = f\"{t}\"", "table"),
        ("data = b'aGVsbG8=' out = f\"{data}\"", "bytes"),
        ("sym = :tag out = f\"{sym}\"", "symbol"),
    ];
    for (input, kind) in cases {
        let err = from_str(input).unwrap_err();
        assert!(
            format!("{err}").contains("interpolat"),
            "unexpected error for {kind}: {err}"
        );
    }
}

#[test]
fn nested_interpolation_through_references() {
    let module = resolve(
        "vars {
            editor = \"nvim\"
            cmd = f\"{vars.editor} -c start\"
        }
        out = f\"run: {vars.cmd}\"",
    );
    assert_eq!(string_of(&module, "out"), "run: nvim -c start");
}

#[test]
fn missing_placeholder_target_fails_like_references() {
    let err = from_str("out = f\"{vars.missing}\"").unwrap_err();
    assert!(
        format!("{err}").contains("reference resolution failed"),
        "unexpected error: {err}"
    );
}

#[test]
fn placeholder_cycles_fail() {
    let err = from_str(
        "vars {
            a = f\"{vars.b}\"
            b = f\"{vars.a}\"
        }",
    )
    .unwrap_err();
    assert!(
        format!("{err}").contains("loop") || format!("{err}").contains("recursion"),
        "unexpected error: {err}"
    );
}

#[test]
fn f_string_as_label_or_key_is_rejected() {
    let err = from_str("vars { name = \"x\" } app f\"{vars.name}\" { }").unwrap_err();
    assert!(
        format!("{err}").contains("not allowed") || format!("{err}").contains("literal"),
        "unexpected error: {err}"
    );
}

#[test]
fn legacy_macro_string_rejected_with_migration_error() {
    let err = from_str("settings { value = m'Hello {name}' }").unwrap_err();
    assert!(
        format!("{err}").contains("legacy macro"),
        "unexpected error: {err}"
    );
}

#[test]
fn legacy_macro_reference_rejected_separately() {
    let err = from_str("vars { editor = \"x\" } settings { value = m!vars.editor }").unwrap_err();
    assert!(
        format!("{err}").contains("legacy macro"),
        "unexpected error: {err}"
    );
}

#[test]
fn byte_literals_are_unaffected() {
    let module = resolve("data = b'aGVsbG8='");
    assert_eq!(
        value_of(&module, "data").as_bytes(),
        Some(&b"hello".to_vec())
    );
}

#[test]
fn reference_expressions_preserve_types_not_strings() {
    let module = resolve("nums { value = 42i32 } settings { value = nums.value }");
    assert_eq!(value_of(&module, "settings.value").as_i32(), Some(&42));
}

#[test]
fn f_string_output_round_trips_through_display() {
    let module = resolve("vars { name = \"x\" } out = f\"a {vars.name} b\"");
    let rendered = value_of(&module, "out").to_string();
    assert!(rendered.contains("a x b"), "rendered: {rendered}");
}
