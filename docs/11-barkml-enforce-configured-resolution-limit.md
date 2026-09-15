# Implementation prompt 11: Enforce the configured resolution-depth limit

## Target and baseline

Implement in [akarifur/barkml](https://github.com/akarifur/barkml), not in the dotfiles repository containing this prompt. The audited baseline is **BarkML v0.8.5**, commit **`9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`**. Read applicable target-repository instructions, inspect the current manifest and APIs, and trace every load, validation, and resolution path before editing. Pinned links describe the audit baseline, not fixed line numbers for the current checkout.

Prior audit evidence: **61 unit tests and 2 doctests passed**, but they did not cover these failures. No BarkML tests were run in this documentation-only task. Add the missing boundary tests and report fresh implementation-time results.

## Observed behavior

- [`LoaderConfig`, `src/load/mod.rs:110-138`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/mod.rs#L110-L138), exposes `max_recursion_depth` and defaults it to 100.
- [`Scope`, `src/ast/scope.rs:8-9`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L8-L9), instead owns a hardcoded 100, used by [the recursive resolver at lines 156-175](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L156-L175). Its [constructor at lines 34-45](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L34-L45) receives no configuration.
- [Default `Loader::load` and `Loader::validate`, `src/load/mod.rs:74-103`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/mod.rs#L74-L103), construct `Scope` without the loader's limit. [Baseline reference validation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L434-L448) only checks direct path existence and cannot establish a transitive depth bound.
- The parser has a separate [nesting limit of 64, `src/syn/parser.rs:8-10`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L8-L10). That is not the loader's resolution-depth setting.

## Desired behavior and exact boundary contract

Use the configured limit everywhere references are evaluated or transitively validated. Define and document the following semantics consistently:

- Depth is the number of **reference dependency edges along the current resolution chain**, not parser nesting, total AST nodes, total substitutions, or cumulative work across siblings. `a = b` with literal `b` has depth 1; `a = b`, `b = c`, with literal `c` has depth 2. These examples use planned ordinary reference syntax.
- A maximum of `N` permits an acyclic chain of exactly `N` edges and rejects a chain requiring `N + 1`, at the edge that would exceed the limit. Literal values need zero edges. A limit of 0 permits literal-only documents but rejects any reference expansion; it does not mean unlimited. Default remains 100 unless the current checkout documents an intentional later change.
- Each explicit f-string reference hole contributes its reference edge and any target dependencies. Multiple independent holes are siblings, not cumulative depth; a literal-only f-string needs no reference expansion. Traversing a table/array/block without following references does not consume this budget.
- Caching/memoization must not make acceptance depend on declaration order or whether a dependency was previously resolved. Retain/check dependency depth information or use an equivalent deterministic design when reusing results. A cached target cannot allow an over-limit chain to bypass the configured bound.

## Implementation requirements

1. Introduce one explicit resolver options/limit path and one source of truth for the default. Propagate `LoaderConfig.max_recursion_depth` through trait defaults, standard-loader builders, custom-loader extension points, `load`, `validate`, and any other entry point that performs reference resolution. Audit direct `Scope` construction, convenience APIs, deferred resolution, cache hits, and validation-on-load rather than patching only one constructor call.
2. Preserve a documented default for direct resolver APIs that lack loader configuration and provide an explicit configured API. Do not silently break custom loaders to obtain the setting; use the repository's compatibility conventions and document any necessary API change.
3. Validation that promises resolvability must traverse dependencies under the same options as evaluation. Pure structural validation need not resolve references; make the distinction explicit. Respect the existing skip-resolution option: do not unexpectedly evaluate references in a path documented as returning raw ASTs. A later explicit resolver call uses its own supplied/default options. Preserve the planned pipeline of unresolved parsing, explicit consumer composition, then reference resolution against the composed root; configuration plumbing must not add premature per-file resolution or reject forward references before composition.
4. Return a structured limit error containing the effective configured limit and the offending dependency location, with useful path context. Safely unwind depth/state on success and error; no underflow, overflow, or stale budget on a later call.
5. Keep cycle detection separate. An active back-edge already encountered is a cycle error, preferably checked before a new expansion is charged; an unseen cycle beyond a smaller limit need not be discovered. The parser's cap of 64 remains independently enforced. Make the error context distinguish parser nesting from reference depth even if an existing error variant is reused.

## Scope and non-goals

Do not claim this depth guard is a total memory, CPU, expansion-size, or adversarial-input sandbox, and do not raise/remove the parser cap to make reference tests pass. Avoid a broad loader redesign or a second competing resolution engine.

Integrate with `03-barkml-reference-expressions.md` and `04-barkml-explicit-string-interpolation.md`: ordinary root-relative paths and explicit f-strings replace `m!`; do not cement obsolete macro grammar. Test legacy entry points only while they remain supported. Keep blocks only (no `[sections]`), structured string labels, and domain-neutral grammar. Use only `true`, `false`, and `null` as canonical boolean/null spellings, but **preserve all numeric suffixes, SemVer values, SemVer requirement literals, and other retained primitives**. Do not restore `$control` or unused BMLS schema features. Separate syntax migrations are not part of this task.

## Regression tests and acceptance criteria

- For limits 0, 1, 2, and a modest custom value, cover chains below, exactly at, and one above the limit, plus literal-only documents. Assert the effective limit, error kind, and source site of the rejected edge.
- Cover the default 100 with flat declarations forming chains of 99, 100, and 101 edges. Include a configured value above 100 to prove the hardcoded constant no longer wins. Keep lexical/AST nesting well below 64; generate flat fixtures or build ASTs directly so parser nesting cannot confound results.
- Exercise direct scope resolution, standard loader `load`, loader validation, builders/config constructors, and any retained convenience/custom-loader APIs that resolve. Verify cache-hit and cache-miss paths obey the same setting.
- Test interpolation-to-reference chains, reference-to-interpolation chains, multiple independent holes, shared/diamond dependencies, and table/array references. Same longest chain must have the same outcome across declaration orders and cache reuse.
- Prove skip-resolution behavior and subsequent explicit configured resolution. Verify a failed limited run does not poison a later run with a larger limit or an independent document.
- Short cycles return cycle errors when visible; a genuinely overdeep acyclic chain returns a limit error. Independently test parser nesting failure and show changing the loader limit does not alter the parser cap.

## Validation and delivery

Run from the BarkML checkout:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Inspect current CI/feature declarations for additional supported checks. Report the entry-point audit, precise documented boundary semantics, API/default compatibility choices, focused regression results, and full-suite/lint outcomes. Identify pre-existing failures explicitly; never use the earlier audit's counts as proof this implementation passed.
