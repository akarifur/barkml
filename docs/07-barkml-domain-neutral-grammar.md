# BarkML: keep application semantics out of the grammar

## Task and context

Apply this architectural boundary and its documentation/tests in https://github.com/akarifur/barkml. BarkML will be used by Stead, a standalone Rust dotfile and application-management tool, but must remain a reusable configuration language.

Baseline: BarkML 0.8.5 at `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Reinspect the current checkout and instructions. This task may primarily require documentation and embedding tests if the boundary already holds; do not invent implementation changes merely to produce a diff.

## Approved division of responsibility

### BarkML owns

- Generic blocks, labeled identities, assignments, tables, arrays, and primitive values.
- Syntax diagnostics and source metadata.
- Ordinary references and explicit interpolation over a caller-supplied configuration/context.
- Generic parsing, traversal, resolution, and serialization/Serde facilities.
- Clear configurable loading/resolution behavior and errors.

### Stead or another consumer owns

- What `app`, `file`, `profile`, `source`, `override`, or `stead` means.
- Provider identities, install/update policies, platform/architecture matching, and host discovery.
- Profile selection, override precedence, and application-level schema validation.
- Secret resolution, package installation, file deployment, and generated-file output.

These block names must remain ordinary identifiers, not grammar keywords:

```text
app "firefox" {
    enabled = true
}

profile "minimal" {
    override {
        app "firefox" {
            enabled = false
        }
    }
}
```

Parsing this document does not select a profile, merge the override, or install Firefox. The example illustrates a possible consumer vocabulary, not built-in BarkML behavior or a finalized Stead schema.

## Embedding contract

Document how a consumer can parse unresolved input, inspect labels and locations, perform its own composition/selection, provide explicit context such as host facts, and request generic reference resolution. Reuse existing APIs where possible and coordinate with prompts 02-04 rather than adding a parallel evaluator.

The parser/resolver must not discover the machine's OS, fetch secrets, invoke commands, or write application configuration on its own. Existing explicit filesystem loading APIs are not forbidden; clearly distinguish them from evaluating a parsed configuration. Do not advertise sandbox guarantees that the implementation does not provide.

Do not add a network import keyword, a built-in Stead provider enum, or a language-wide profile/override engine as part of this task. Preserve existing intentional generic module-merge APIs and document their explicit invocation; they must not be confused with downstream profile policy.

## Acceptance criteria and tests

- Stead-looking blocks parse exactly like arbitrary application-defined block names.
- Unrecognized consumer vocabulary is not a BarkML syntax error simply because it is not a known Stead resource.
- Parsing or generic resolution does not execute application/provider actions.
- Embedding examples demonstrate explicit context/composition without ambient host discovery.
- Documentation separates syntax/type/reference errors from downstream schema errors.
- Labels and source locations remain available for downstream lowering and diagnostics.
- Numeric suffixes, native SemVer, and requirement literals remain generic language features, not Stead-only extensions.
- Any new public embedding examples compile or are clearly marked conceptual if they depend on a downstream application.

Run `cargo fmt --check`, `cargo test` including doctests, and applicable Clippy checks. Summarize which boundaries were already satisfied versus newly enforced.

## Starting points

- [Public parsing entry point](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/lib.rs#L95-L100)
- [Loading and resolution APIs](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/mod.rs)
- [AST-to-Serde conversion](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/de/mod.rs#L23-L60)
