# Implementation prompt 10: Detect reference cycles with dependency traces

## Target and baseline

Implement in [akarifur/barkml](https://github.com/akarifur/barkml), not in the dotfiles repository holding this prompt. The audited baseline is **BarkML v0.8.5**, commit **`9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`**. Read applicable repository instructions and reinspect the current AST, resolution engine, loader/validation entry points, diagnostics, and tests. Follow current symbols and behavior rather than blindly applying the pinned line numbers.

Prior audit evidence: **61 unit tests and 2 doctests passed**, but they did not cover these failures. No BarkML tests were run during this documentation-only task. Produce new regression coverage and report actual implementation-time test results.

## Observed behavior

- [Reference and interpolation resolution, `src/ast/scope.rs:156-269`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L156-L269), recursively follows macro values; it does not mark the dependency active before descending. In direct reference resolution, lines 187-196 also copy the referring value's UID onto the target payload, so call-site identity cannot safely stand in for dependency identity.
- [`resolve_value`, lines 372-411](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L372-L411), checks `visit_log` before descent but only inserts after recursive work. Active cycles therefore recurse until the separate depth limit, while a global completed-visit set can confuse legitimate reuse with a loop.
- [`validate_macros`, lines 434-448](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L434-L448), checks direct target existence instead of traversing dependencies. Validation and actual resolution need a coherent cycle rule.

## Desired behavior and implementation

1. Model dependency state as unseen, **active on the current traversal stack**, or successfully resolved. Mark a node active before following its dependencies. An edge back to an active node is a cycle; an edge to a resolved node is valid reuse. Never use a set of all previously visited nodes as the cycle criterion.
2. Key dependency identity by the actual target node/canonical structured path in the correct scope, not a copied caller UID or a lossy dot-joined display string. Reuse the structured identity established for string labels and reference paths; preserve separate call-site metadata for diagnostics.
3. Maintain a dependency stack with reference-edge locations. Return a structured cycle error with a useful closed trace such as `a -> b -> c -> a`, the repeated target, and source locations for the dependencies forming the cycle. Include module/path context and stable ordering. Do not make callers scrape a formatted message to recover the cycle.
4. Resolve references inside all supported value containers and explicit interpolation using the same traversal state. Only cache successful results; clear/unwind active state on every error path. A failed application must not poison subsequent resolution attempts.
5. Have both validation and evaluation share the cycle-aware engine or dependency walk rather than maintaining incompatible algorithms. Preserve normal unknown-reference/type errors; an unresolved path is not a cycle.
6. Keep a separately configured depth/resource guard for long acyclic chains. A detected active back-edge is a cycle error, not a generic recursion-limit error. Check back-edges before charging another expansion when the edge is visible at the boundary; do not promise to diagnose an unseen cycle beyond a smaller configured limit without traversing it.

## Syntax integration and non-goals

The accepted replacement for `m!` is **ordinary root-relative reference expressions**, as described in `03-barkml-reference-expressions.md`; interpolation belongs only in **explicit f-strings**, as described in `04-barkml-explicit-string-interpolation.md`. Implement this graph rule at the semantic reference layer, not by further entrenching `m!` string parsing. If the syntax migration has not landed, use AST-level tests plus fixtures for entry points currently supported; keep any retained legacy entry points tested until explicitly removed. Do not add relative `self`/`super` syntax or arbitrary expression evaluation as part of this fix.

Preserve the pipeline **unresolved parse -> explicit consumer composition -> reference resolution**. Detect cycles against the consumer's composed root, allowing forward references; do not resolve each source prematurely during parsing or invent implicit composition to make lookups succeed. Add a test where separately parsed sources form a valid forward dependency after explicit composition and one where composition exposes a cycle.

Keep blocks only (no `[sections]`), structured string labels, and a domain-neutral grammar. The canonical boolean/null spellings are `true`, `false`, and `null`, but **all numeric suffixes, SemVer values, and SemVer requirement literals must remain supported**. Do not remove other primitives, restore `$control`, or reintroduce unused BMLS schema features. This is a dependency-resolution fix, not a language migration or total memory sandbox.

## Regression tests and acceptance criteria

- Direct self-reference, a two-node cycle, a longer cycle, and a noncyclic prefix leading into a cycle each return a typed cycle error without reaching the default depth-limit failure. Assert the closed cycle's ordered nodes and the dependency source locations, not just the word "cycle".
- Cycles through explicit interpolation, mixed ordinary references/interpolation, table entries, and array elements are detected wherever those references are supported. Keep tests for any retained legacy API/grammar paths; ordinary non-f strings with brace text create no dependencies after the syntax migration.
- Valid diamond dependencies, two fields referencing the same value, repeated interpolation of one dependency, and reuse of an already-resolved container all succeed. Check the resolved values, not only `is_ok()`.
- Same display spelling in different scopes, distinct structured labels containing dots, and references to different table/array targets cannot collapse into one cycle identity.
- Missing paths remain missing-reference errors. A long acyclic chain produces the separately configured limit error, while a short visible cycle remains a cycle error.
- `load`, validation, and direct resolver APIs agree. Repeated successful `apply` calls and a retry after failure do not report stale active-state cycles. Shared results must not lose the metadata needed to attribute distinct reference sites.
- Tests are deterministic regardless of harmless declaration order changes; diamonds must not begin failing because a dependency was resolved earlier.

## Validation and delivery

Run in the BarkML checkout:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Inspect the current manifest/CI and run additional supported feature combinations where appropriate. Report the chosen dependency identity/state model, diagnostic shape, validation/evaluation integration, and actual regression/full-suite results. Separate pre-existing lint failures from new ones and do not portray the earlier audit as verification of this change.
