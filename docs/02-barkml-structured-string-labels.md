# BarkML: restrict block labels to strings and preserve structured identity

## Task and context

Implement this change in https://github.com/akarifur/barkml. This prompt is stored in a separate dotfiles repository for later use. The user approved string-only block labels whose identity remains structured rather than flattened into dotted strings.

Baseline: BarkML 0.8.5 at `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Inspect the actual checkout and its instructions first; adapt to changes already made by related prompts.

## Required behavior

Support zero or more ordered, literal string labels:

```text
settings {
    editor = "nvim"
}

app "firefox" {
    enabled = true
}

artifact "linux" "aarch64" {
    url = "https://example.invalid/tool"
}
```

Reject numeric, boolean, null, array, table, SemVer, and other non-string block labels. Keep labels literal and statically identifiable in this iteration; do not add reference-valued or interpolated labels as an incidental extension.

Represent block identity as the block name plus an ordered sequence of labels. Preserve component boundaries throughout parsing, child storage, lookup, reference resolution, serialization, and diagnostics. Do not use dot-joining, stringified values, or generated AST UUIDs as semantic identity.

For example, these identities must not collide:

```text
app "a.b" { enabled = true }
app "a" "b" { enabled = false }
```

Different labels for the same block kind are valid. Repeating the same structured identity in one lexical scope is a duplicate declaration, handled by `08-barkml-reject-duplicate-declarations.md`. Deliberate cross-module composition is a separate operation.

## Integration requirements

Audit APIs that assume all children are keyed by a single `String`. Prefer one coherent identity representation rather than local escaping conventions or special cases for Stead. Preserve source locations for individual labels.

Distinguish two Serde paths. The custom `de::from_statement` projection treats labeled blocks as maps of their children and does not expose labels as fields in the consumer's value. Direct serialization/deserialization of the derived `Statement`/`StatementData` AST retains its `Labeled` data, including labels. Preserve that AST round-trip behavior. Provide/document an AST-aware traversal or conversion path for consumers that need resource names; preserve the existing child-map projection where appropriate. Do not inject Stead-specific fields into arbitrary users' structs or claim that all Serde paths lose labels.

Coordinate reference-path representation with `03-barkml-reference-expressions.md`. Public lookup must provide an unambiguous way to address an entire ordered label sequence, even if surface syntax is finalized in that prompt.

## Acceptance criteria and tests

- Zero-label, one-label, and multiple-label blocks work.
- Every previously accepted non-string label category is rejected with a location-aware error.
- Dots, spaces, brackets, Unicode, and escaped quotes within labels survive parsing and identity lookup.
- `("app", ["a.b"])` and `("app", ["a", "b"])` remain distinct through lookup and supported round trips.
- Ordered label sequences remain ordered; labels are not treated as sets.
- Direct derived-AST Serde round trips preserve labels. Tests/documentation separately characterize the child-map projection offered by `de::from_statement`; do not expect that projection to reconstruct resource identity automatically.
- Same-kind/different-label siblings remain accessible without overwriting.
- Typed numbers and version literals remain valid as values; this restriction applies only to block labels.
- Documentation explains breaking label and lookup API changes.

Run `cargo fmt --check`, `cargo test`, and applicable Clippy checks. Include downstream API migration notes.

## Starting points

- [Labeled AST data](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/statement.rs#L14-L34)
- [Existing statement deserialization](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/de/statement.rs#L29-L41)
- [Existing symbol-table construction](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L47-L92)
