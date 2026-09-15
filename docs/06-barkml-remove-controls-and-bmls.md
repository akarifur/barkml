# BarkML: remove control statements and the unused BMLS schema surface

## Task and context

Implement this change in https://github.com/akarifur/barkml. The user approved removing `$` control statements and the effectively unused BMLS schema facility. Metadata should use ordinary configuration constructs, with schemas interpreted by the consuming application rather than a built-in schema language.

Baseline: BarkML 0.8.5 at `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Inspect the actual checkout and its instructions first. The baseline README says schema checking is downstream, while an example `.bmls` file sketches a schema language: do not assume a functioning BMLS engine exists.

## Required behavior

Migrate examples such as:

```text
$schema = !Stead 1.0.0
```

into ordinary application-defined data, for example:

```text
stead {
    schema = 1.0.0
}
```

The block name and meaning above are an example for a consumer. BarkML must not reserve `stead`, validate this schema version, or load a schema because it encounters such a block. The baseline lexer also reserves `schema` as the unused `KeySchema` token. Remove that BMLS-only reservation so `schema` becomes an ordinary identifier; audit any other schema-only reserved words rather than merely deleting the example file.

Remove control-statement lexing/parsing, control-only AST variants, constructors, traversal, serialization paths, and documentation where no longer needed. Removed control syntax should produce a useful error rather than becoming a different declaration silently. Preserve literal dollar signs inside ordinary strings.

Inventory `.bmls` files, schema-specific documentation/examples, and any genuinely schema-only implementation. Remove obsolete schema syntax/artifacts and update references. Do not leave examples suggesting a built-in schema engine remains available. Do not delete generic validation, location metadata, or typed-value support just because it was once useful to schema work.

## Explicit preservation requirements

The user wants native typed numbers, SemVer values, and SemVer requirements retained. Keep those features, primitive type hints, ordinary block labels, and normal AST metadata intact.

Distinguish `$` controls, optional assignment/value metadata labels, primitive type annotations, and block labels during the inventory. They are different concepts. Do not remove unrelated `!` annotation behavior without demonstrating that it is exclusively part of the retired control/BMLS surface; flag any broader proposed removal for a separate decision.

This is an intentional syntax/API break. Document migration and remove obsolete public examples rather than adding an invisible compatibility parser.

## Acceptance criteria and tests

- Legacy `$name = ...` statements are rejected clearly.
- Equivalent ordinary blocks and assignments parse without special treatment.
- `$HOME` and other literal dollar text in ordinary strings remain unchanged.
- Arbitrary metadata blocks work; no Stead-specific names are reserved.
- `schema` works as an ordinary assignment identifier, block name, table key, and reference-path component after removal of `KeySchema`. Test those forms in separate valid scopes and coordinate reference tests with prompt 03.
- There are no remaining active BMLS examples or public claims of built-in schema-language support.
- Generic AST validation, Serde integration, source metadata, primitive type hints, and native version types remain functional.
- All removed public APIs are covered by migration notes and their callers/tests are updated.

Coordinate with `01-barkml-blocks-only-grouping.md` and `07-barkml-domain-neutral-grammar.md`. Run `cargo fmt --check`, `cargo test`, and applicable Clippy checks; report which schema artifacts actually existed and were removed.

## Starting points

- [Control/assignment construction and type conversion](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/statement.rs)
- [Unused schema example](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/examples/example.bmls)
- [Lexer and reserved schema token](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/lexer.rs)
- [Control/schema documentation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/README.md)
