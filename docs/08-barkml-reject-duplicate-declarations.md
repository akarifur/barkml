# Implementation prompt 08: Reject duplicate declarations before insertion

## Target and baseline

Implement this change in [akarifur/barkml](https://github.com/akarifur/barkml), not in the dotfiles repository containing this prompt. The audited baseline is **BarkML v0.8.5**, commit **`9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`**. Read the target checkout's applicable instructions, inspect its current parser, AST identity model, errors, and tests, and account for changes already applied. The pinned links below are evidence, not instructions to edit stale line numbers.

Prior audit evidence: **61 unit tests and 2 doctests passed**, but they did not cover these failures. No BarkML tests were run while authoring this documentation-only prompt. Add regression coverage and report fresh results during implementation.

## Observed behavior

The parser uses unchecked `IndexMap::insert`, silently discarding earlier declarations:

- [Block children, `src/syn/parser.rs:613-628`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L613-L628) are inserted by `value.inject_id()` without checking an existing entry.
- [Module and legacy section children, `src/syn/parser.rs:707-725`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L707-L725) use the same overwrite pattern. The section branch is historical evidence, not a request to keep sections.
- [Table parsing, `src/syn/parser.rs:383-438`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L383-L438) overwrites both the value map and type map at lines 437-438. It has already captured the key's location at lines 401-406, but does not preserve it for duplicate diagnostics.

For example, `count = 1\ncount = 2` succeeds after losing the first declaration. Checking an already-built map cannot recover the lost information.

## Desired behavior and implementation

1. Reject a repeated declaration identity **within one lexical scope** before replacing any earlier entry. Cover root statements, block children, and table keys. Reuse one clear checked-insertion rule where their semantics agree; audit additional parser maps, including table-type members if still supported, rather than fixing only the example.
2. Return a structured duplicate-declaration error carrying the conflicting identity and **both original and repeated source locations**, including module/file context. Retain key/header locations, not merely the value location after `=`. Keep any map of diagnostic metadata local to the scope or in the existing AST metadata model, rather than inventing a global duplicate registry.
3. Block identity must use the **block name plus an ordered sequence of decoded string labels**, consistent with `02-barkml-structured-string-labels.md`. Do not join labels with dots or use formatted display strings as equality keys. If that change has not landed, introduce or coordinate the minimum structured identity needed; do not perpetuate the lossy identity for convenience.
4. Same block kind with distinct labels is valid. Exact same name and ordered labels in the same scope is a duplicate; repeated unlabeled blocks of the same identity are also duplicates. Label order and boundaries matter. Preserve the target AST's explicit statement/block namespace policy; do not introduce silent overwrites between statement kinds. If that policy is missing, make the conflict rule explicit before implementing it.
5. A second declaration in a separate lexical scope is not a duplicate. **Deliberate loader merges of separate source modules are different operations**: preserve their documented collision policy. A permissive loader must still reject duplicates inside an individual source document before merging.

## Scope and non-goals

This is a parser integrity and diagnostic change, with only the necessary AST identity/error plumbing. It is not a global ban on layered configuration overrides and not a serializer or loader redesign.

Respect the accepted language direction: blocks only, no `[sections]`; string labels with structured identity; ordinary root-relative reference paths and explicit f-string interpolation; only `true`, `false`, and `null` as canonical boolean/null spellings. **Keep every numeric suffix, SemVer values, and SemVer requirement literals**; do not simplify or remove primitives. Do not restore `$control` or unused BMLS schema features. Keep the grammar domain-neutral, with no Stead-specific keywords. Separate syntax migrations need not be implemented wholesale for this fix, but this fix must not cement obsolete syntax.

## Regression tests and acceptance criteria

- Duplicate root assignments, assignments inside one block, and keys inside a table each fail with a typed duplicate error and both exact locations. Include same-value duplicates, differing values/types, quoted versus unquoted equivalent keys, intervening comments, and multiline input.
- Nested table duplicate keys fail before either value/type entry is overwritten. Distinct keys retain insertion order and their original metadata.
- Two identical block identities fail at root and inside a parent; distinct string labels on the same kind succeed. Test an unlabeled duplicate and multiple labels with their order reversed.
- Identity separation tests include one label `"a.b"` versus two labels `"a" "b"`, labels containing escaped quotes, and equal decoded strings written with equivalent escapes. Dotted display output must not determine identity.
- A field reused under two different parents and the same labeled block under different parents succeed; duplicate state must not leak between scopes or separate parse calls.
- Intentional layered merge succeeds or fails according to the loader's collision setting; each individual source containing a duplicate fails regardless of that setting.
- Tests assert structured identity/location fields, not only an error substring. Parsing must never panic or return an apparently valid truncated/overwritten AST on these failures.

## Validation and delivery

Run from the BarkML checkout after adding focused regression tests:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Inspect `Cargo.toml` and CI for additional supported feature combinations and test them as appropriate. Run focused tests while iterating, then the complete commands above. Distinguish pre-existing lint failures from introduced ones; do not mask warnings or claim a command passed when it did not. Summarize the identity rule, error API changes, regression cases, and actual validation results.
