# BarkML: canonical boolean/null spellings while retaining numeric and version literals

## Task and explicit user decision

Implement this change in https://github.com/akarifur/barkml. The user approved simplifying redundant scalar spellings but explicitly rejected removing typed numeric literals, native SemVer literals, or native SemVer requirement literals. Preserve those features as first-class values; do not replace them with strings or make this a numbers-only simplification.

Baseline: BarkML 0.8.5 at `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Reinspect the current checkout and applicable instructions before editing.

## Required behavior

The only boolean and null literal spellings are:

```text
enabled = true
disabled = false
missing = null
```

Remove literal aliases such as `True`, `TRUE`, `yes`, `Yes`, `on`, `OFF`, `nil`, `Nil`, `none`, and `None`. Exhaustively audit the actual lexer rather than treating this list as complete. Decide token behavior consistently with ordinary reference expressions: former aliases may be identifiers/references under the new grammar, but must never silently remain boolean/null literals. Document that distinction; quoted strings containing these words are unaffected.

## Features that MUST remain

Preserve all existing supported numeric suffixes and their type/range semantics:

- `u8`, `u16`, `u32`, `u64`, `u128`
- `i8`, `i16`, `i32`, `i64`, `i128`
- `f32`, `f64`
- Existing unsuffixed integers/floats and exponent forms

Preserve native semantic versions and semantic-version requirements, including supported prerelease/build forms and requirement operators:

```text
port = 8080u16
offset = -2i64
ratio = 3.14f32
schema_version = 1.0.0
release = 1.2.3-beta.1
compatible = ^1.2
minimum = >1.1
patch_series = ~5.3
```

Parsing and reference resolution must retain the distinct AST variants for these values. Do not silently erase numeric width, narrow values with unchecked casts, or replace native version/requirement AST values with string nodes. Keep primitive type hints, byte values, and other unrelated scalar features unchanged.

Distinguish AST preservation from consumer deserialization. Derived AST Serde round trips retain the AST representation; the custom `ValueDeserializer` exposes SemVer and requirements through a string visitor so consumer targets such as `semver::Version` and `semver::VersionReq` can decode them. Generic targets such as `String` or `serde_json::Value` do not preserve every BarkML token category, and need not do so. Preserve the supported conversion contract rather than requiring every Serde target to retain language-level types.

Stead may also accept ordinary strings for provider-specific versions that are not SemVer. That downstream requirement is not a justification for removing BarkML's native version types.

## Acceptance criteria and tests

- `true`, `false`, and `null` parse to their canonical types and serializers emit those spellings.
- Every removed alias no longer parses as a boolean/null literal; cover case variants and identifier boundaries.
- Quoted aliases remain strings.
- Valid minimum/maximum values for each integer width retain their type; overflow and invalid signedness produce errors.
- Existing float suffixes/exponents and their established precision behavior remain covered.
- SemVer, prerelease/build values, and every supported requirement operator keep working.
- Token boundaries distinguish integers, floats, SemVer, requirements, and reference paths after prompt 03.
- Parsing, reference resolution, and supported derived-AST Serde round trips preserve the numeric/version AST variants.
- Consumer deserialization into appropriate Rust numeric types, `semver::Version`, and `semver::VersionReq` still works. Separately characterize documented string/general-value projections; do not assert that those targets preserve BarkML token categories.
- Documentation clearly calls out both the alias removal and deliberate retention of numeric/version features.

Do not expand version-requirement syntax or change numeric coercion policy opportunistically. Run `cargo fmt --check`, `cargo test`, and applicable Clippy checks; report any unrelated existing boundary defects separately.

## Starting points

- [Lexer](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/lexer.rs)
- [Value representation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/value.rs)
- [Language literal documentation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/README.md)
