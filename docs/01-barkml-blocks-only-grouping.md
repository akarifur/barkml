# BarkML: use blocks for all structural grouping

## Task and context

Implement this change in https://github.com/akarifur/barkml, not in the dotfiles repository that stores this prompt. BarkML is being considered as the configuration syntax for Stead, a standalone Rust dotfile and application-management tool.

The user has approved removing TOML-style `[section]` syntax because blocks already provide explicit, nestable grouping. Keep tables as assignable values; they are not redundant with named structural blocks.

Inspection baseline: BarkML 0.8.5, commit `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Reinspect the current checkout and applicable agent instructions before editing. Other prompts may already have changed the implementation.

## Required behavior

Replace:

```text
[settings]
editor = "nvim"
```

with:

```text
settings {
    editor = "nvim"
}
```

Both unlabeled and labeled blocks remain supported, including nested blocks. Keep the distinction between a declaration and an assigned table:

```text
file "starship" {
    contents = {
        add_newline = false
    }
}
```

Remove section parsing and section-only AST/type/walker/serialization behavior where it is no longer needed. Preserve the implicit document/module root and genuine module-loading functionality; a module root is not a section. Do not merely stop accepting sections while leaving public APIs and examples that still imply they are supported.

Reject legacy section headers with a useful migration-oriented diagnostic. Do not reject array literals or array type syntax merely because they also use square brackets. Update serializers/formatters so they never emit removed section syntax.

## Scope and compatibility

This is an intentional syntax/API break. Document the old-to-new form and follow the repository's versioning policy. Do not silently reinterpret an old section header as another construct.

Preserve primitive type hints, numeric-width suffixes, SemVer literals, SemVer requirement literals, arrays, tables, and byte values. Coordinate with `02-barkml-structured-string-labels.md` and `09-barkml-reject-unterminated-blocks.md`; do not duplicate their identity or delimiter implementations.

## Acceptance criteria and tests

- Empty, nested, sibling, unlabeled, and labeled blocks parse correctly.
- Braces establish scope; a statement after a closing brace belongs to its parent scope.
- Legacy quoted and unquoted section headers fail clearly.
- Arrays and tables still parse, including tables nested in arrays.
- Existing section-based examples/tests are migrated without changing their intended data.
- Public documentation and any emitted BarkML no longer advertise sections.
- Existing typed numeric and version literal tests remain passing.

Run `cargo fmt --check`, `cargo test`, and the repository's applicable Clippy checks. Report intentional API breaks and migration changes separately from test results.

## Starting points

- [Section parser](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L707-L725)
- [Statement AST](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/statement.rs#L14-L34)
- [Language documentation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/README.md)
