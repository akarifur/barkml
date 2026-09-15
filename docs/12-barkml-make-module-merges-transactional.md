# Implementation prompt 12: Make module merges transactional

## Target and baseline

Implement in [akarifur/barkml](https://github.com/akarifur/barkml), not in the dotfiles repository containing this prompt. The audited baseline is **BarkML v0.8.5**, commit **`9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`**. Read applicable repository instructions and reinspect the current merge implementation, documentation, loader entry points, AST identity, and tests. Confirm the exact current documented contract before editing; the pinned lines are baseline evidence, not guaranteed current coordinates.

Prior audit evidence: **61 unit tests and 2 doctests passed**, but they did not cover these failures. No BarkML tests were run while writing this documentation-only prompt. Implementation must add transactional regression tests and report new results.

## Observed behavior and documented contract

The baseline [`merge_statements` documentation, `src/load/standard.rs:84-99`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L84-L99), explicitly says it “supports partial merging with rollback on failure.” It describes `left` as the in-place destination and `allow_collisions` as allowing overwrite on conflict.

However, [the implementation at lines 100-156](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L100-L156) mutates earlier children while iterating, then propagates a later recursive conflict with `?`. There is no rollback. A source ordered as “new child, then conflicting child” can return an error while leaving the new child installed.

This is caller-visible: [`add_module`, lines 209-229](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L209-L229), and [`import`/`add_file`, lines 241-309](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L241-L309), merge directly into stored modules, including cached-source paths. [`get_module`, lines 385-388](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L385-L388), exposes the resulting destination.

## Desired behavior and implementation

1. A failed merge leaves the **entire caller-visible destination statement unchanged**, not just the child where the error occurred. This includes existing values, children, insertion order, labels, identifiers, and source metadata/UIDs. The source remains unchanged as well.
2. Establish an explicit staged/commit boundary for each merge. Compare a prevalidated merge plan with a staged copy of only the affected destination module. Prefer a plan whose commit is infallible if validation and application rules can remain one coherent source of truth; otherwise staging the affected module once is a reasonable, documented tradeoff. Do not clone the entire loader/cache/system or clone the entire subtree at every recursive step without justification. An undo log is only appropriate if its complexity and all-error-path correctness are clearly warranted.
3. Preserve successful merge behavior and collision policy. With collisions disabled, existing groups can recursively combine disjoint children, while conflicting values or prohibited shape mismatches error. With collisions enabled, permitted conflicts keep the existing right-hand overwrite behavior and deterministic ordering. Do not change collision semantics to avoid implementing transactions.
4. Apply the guarantee through every path that calls merging, including in-memory modules, file imports, additions to `main`, and cache hits. Parse/validation failure before commit must never publish a partially staged destination. Do not mark a failed merge as a successfully created module.
5. Define the transaction boundary in documentation. The required minimum is **one source-to-destination merge**. Directory/discovery APIs currently iterate files separately ([lines 314-345](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L314-L345)); inspect whether the current API additionally promises batch atomicity. Preserve any stronger promise, but do not silently advertise whole-directory rollback when only each file merge is atomic. Explicitly document whether prior successful file merges remain after a later file fails.
6. Distinguish destination atomicity from cache/statistics accounting. A successfully parsed source may be cached even if its merge conflicts, unless the existing public contract promises otherwise. Document this rather than casually broadening rollback to every counter and cache. No cached state may cause a later attempt to bypass the collision check or publish a partial merge.

## Scope and non-goals

Intentional layered source merging is valid and remains controlled by `allow_collisions`. It is **not** a license for duplicate declarations within one source: those must be rejected before parser insertion loses information. Do not fix transactionality by globally rejecting all same-name blocks or by suppressing collision errors. Use structured block name plus ordered string-label identity consistent with `02-barkml-structured-string-labels.md`, not a dotted display string.

Keep the accepted grammar direction: blocks only, no `[sections]`; ordinary root-relative reference paths and explicit f-string interpolation; canonical boolean/null spellings `true`, `false`, and `null`. **Preserve all numeric suffixes, SemVer values, SemVer requirement literals, and retained primitives.** Do not restore `$control`, unused BMLS schema features, or domain-specific Stead syntax. Separate syntax migrations and a general persistence/transaction framework are outside this fix.

## Regression tests and acceptance criteria

- Construct a destination and a source with an early new key followed by a conflicting existing value under collisions-disabled mode. Snapshot the destination before merging; after `Err`, assert complete structural and metadata equality and exact insertion order. Do not compare only serialized values, which can hide metadata mutations.
- Repeat with a conflict several levels down after earlier changes at both root and nested levels, and with group/value shape mismatches in both directions. Assert both collision locations remain accurate.
- Verify the source is unchanged on success and failure. Retry after a failed merge with a corrected source and verify no earlier failed inserts remain.
- Successful disjoint group merge preserves all expected children/order. Successful collision-enabled merge overwrites intended values and permitted shape mismatches while retaining unrelated children. Collisions-disabled mode still rejects equal-value duplicate assignments across layers where the existing policy considers them collisions.
- Exercise `add_module`, `import`, `add_file`, and retained merge APIs through public loader observations, with cache hit and miss variants. After failure, `get_module`/`read` must expose the pre-merge destination.
- Check the documented directory/batch boundary explicitly: either whole-call rollback if promised or retained prior successful merges with no partial failed-file merge. Test success/failure counters according to their documented meanings.
- Individual source duplicates still fail independently of loader collision permission. Distinct structured label identities remain distinct, and intended identical identities merge according to policy.

## Validation and delivery

Run in the BarkML checkout:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Inspect current features and CI for any additional supported checks. Report the confirmed old contract, chosen staging/commit approach and copying cost, public transaction boundary, focused regressions, and actual full-suite/lint results. Distinguish any pre-existing failures. Do not claim prior audit counts validate this implementation.
