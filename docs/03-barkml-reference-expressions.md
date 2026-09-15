# BarkML: replace macro replacements with reference expressions

## Task and context

Implement this change in https://github.com/akarifur/barkml, not in the repository holding this prompt. The approved direction replaces `m!path` with ordinary reference expressions in value position. BarkML remains a small declarative language, not a general-purpose evaluator.

Inspection baseline: 0.8.5 at `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Reinspect the current checkout, instructions, tests, and related changes before implementation.

## Required syntax and semantics

```text
vars {
    editor = "nvim"
}

settings {
    editor = vars.editor
}
```

Quoted text is literal text. An unquoted identifier path in value position is a reference, except for reserved literal spellings. Preserve all numeric suffixes, SemVer literals, and SemVer requirement literals; lexer precedence must not turn them into references.

Use root-relative paths initially. Remove implicit `self`/`super` lookup behavior from the new reference model, documenting migration from legacy macros. Do not add ambient environment lookup, command substitution, arbitrary calls, or reference-triggered filesystem/network access.

Support dot fields and quoted selectors so punctuation stays inside a component:

```text
settings = app["org.mozilla.firefox"].settings
value = vars["key.with.dots"]
```

Coordinate structured selectors with `02-barkml-structured-string-labels.md`. Define and document unambiguous addressing for multi-label blocks before freezing the grammar. A proposed spelling for a complete multi-label selector is `artifact["linux", "aarch64"]`; this spelling is implementation guidance, not an already-approved language decision. Do not fall back to splitting labels on dots.

Array-element addressing is also an open syntax decision, distinct from permitting references inside arrays or referencing an entire array value. Proposed guidance is zero-based `items[0]`, with numeric indices distinct from quoted keys and label selectors. Confirm this addition before implementing it; otherwise explicitly defer array-element selectors. Do not silently reuse legacy dotted numeric paths or present array indexing as already approved.

A reference preserves the referenced value's type, including arrays, tables, typed numbers, SemVer, and requirements. Copying a reference must not serialize and reparse its target or erase source provenance.

## Evaluation boundary

Parsing must be possible without resolving references. Callers such as Stead must be able to compose/select configuration and supply an explicit context before resolution. The resolver operates on that supplied tree/context, not on Stead-specific profile rules.

Allow forward references. Resolve against the composed configuration so incidental file or declaration order does not decide whether a reference exists. Preserve missing-target, wrong-selector, and incompatible-target diagnostics with the referring location and useful target information.

Use explicit reference/path AST nodes, not a string encoding shared indiscriminately with interpolation. Share traversal/resolution primitives with `04-barkml-explicit-string-interpolation.md`. Coordinate cycle handling with prompt 10 and configurable limits with prompt 11; do not create competing resolution algorithms.

## Acceptance criteria and tests

- Backward and forward references resolve equivalently after reordering independent declarations.
- A path inside a nested block resolves from the document root, even when a similarly named local declaration exists. `self` and `super` never perform implicit relative traversal; if permitted as ordinary identifiers, they follow the same root lookup rule.
- References can appear in assignments, arrays, and tables.
- References preserve scalar widths/types, composite values, and version values.
- Quoted dotted labels/keys remain a single component.
- Missing references and invalid selectors for the approved path grammar produce actionable errors, not panics.
- If the proposed array-index syntax is approved, test zero/last indices, empty arrays, out-of-range and negative/non-integer indices, and distinction from quoted numeric keys. If deferred, reject such selectors clearly rather than leave undocumented partial support.
- Chained references, shared dependencies, and cycles exercise the same resolver used by interpolation.
- A caller can parse unresolved input, compose it, and then resolve it explicitly.
- Numeric literals, version literals, requirement operators, and canonical booleans/null retain their intended tokenization.
- Legacy `m!` examples and documentation are migrated; removed forms fail clearly.

Document any reference-order or lookup API compatibility changes. Run `cargo fmt --check`, `cargo test`, and applicable Clippy checks.

## Starting points

- [Existing complete-tree symbol construction](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L36-L92)
- [Existing replacement/interpolation implementation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L156-L269)
- [Loading/resolution boundary](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/mod.rs)
