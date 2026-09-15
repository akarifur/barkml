# Implementation prompt 13: Preserve structured parse errors through loading

## Target and baseline

Implement in [akarifur/barkml](https://github.com/akarifur/barkml), not in the dotfiles repository containing this prompt. The audited baseline is **BarkML v0.8.5**, commit **`9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`**. Read applicable repository instructions, then reinspect current errors, location types, lexer/parser entry points, file loading, and tests. Pinned links are evidence of the baseline defect, not instructions to edit stale line numbers.

Prior audit evidence: **61 unit tests and 2 doctests passed**, but they did not cover these failures. No BarkML tests were run for this documentation-only task. Add the missing diagnostic regressions and obtain fresh implementation-time results.

## Observed behavior

- [`StandardLoader::parse_file`, `src/load/standard.rs:186-193`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L186-L193), constructs a parser and maps any parser error to `Error::Io { reason: format!(...) }`. The original variant, fields, and source chain become inaccessible except as display text.
- The [error enum, `src/error.rs:26-79`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/error.rs#L26-L79), already distinguishes syntax/type/EOF errors and contains structured locations; `Io` has only a `reason` string. Real read failures are separately mapped to `Io` at [standard-loader lines 171-177](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L171-L177).
- [`import` at lines 252-259](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L252-L259) and [`add_file` at lines 294-301](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L294-L301) pass a basename to `parse_file` rather than preserving the supplied full path. Merely forwarding the existing error would therefore still give incomplete file context.
- [Validation-on-load at lines 199-203](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/load/standard.rs#L199-L203) also flattens semantic errors into `Io`. Avoid leaving this neighboring path as a way to erase the same structured diagnostics after parsing succeeds.

## Desired behavior and implementation

1. Preserve the original typed lexical/parser error through file loading, including its expected/found information, context, spans/locations, and any underlying source. Attach file path and logical module identity as structured context without replacing the original error with a formatted string.
2. Compare forwarding an already adequately contextualized error with a dedicated load-context wrapper containing the original error as its source. Choose the smallest coherent design for the current error API. If wrapping, expose the original error through `std::error::Error::source()` and a typed field/accessor; preserve the full chain for parser errors that themselves have causes. Do not require every leaf syntax error to fabricate a source when it naturally has none.
3. Keep physical source identity separate from logical module identity. A file merged into `main` still reports its actual source path, and two files with the same basename in different directories remain distinguishable. Carry `Path`/`PathBuf` without lossy conversion internally where appropriate. Display may format a path, but machine-readable context must not depend on parsing display text or on canonicalization succeeding.
4. For in-memory parsing, use an explicit supplied source name/module and optional path; do not invent a filesystem path. Ensure string/direct-parser errors and file errors have equivalent underlying categories and locations for identical content, with only their source context differing.
5. Genuine filesystem open/read failures remain I/O failures, not syntax failures; preserve available I/O context/source when touching those mappings. Retain any established separate `NotFound` category unless an intentional documented API change is needed. Invalid UTF-8 bytes failing `read_to_string` are a decoding/read failure, not a fabricated parser location. Valid UTF-8 content must retain accurate syntax offsets and line/column mapping.
6. Apply the same context-preserving rule to validation-on-load and other wrappers along the relevant path: semantic validation remains semantic, not I/O. Audit file, directory/discovery, import/add, and in-memory reader entry points for stringification. Do not accidentally alter a validation or resolution failure into a successful load.
7. Keep diagnostics consistent with duplicate declarations, unmatched delimiters, cycles, and configured depth errors as those fixes land. Errors carrying multiple locations must retain all of them. Inspect the public error enum's derives/API compatibility before adding wrappers; choose suitable ownership rather than imposing a specific diagnostics dependency.

## Scope and non-goals

This is error fidelity and source provenance, not a diagnostic-rendering framework, new CLI, forced `miette`/`ariadne` dependency, filesystem security redesign, or blanket rewrite of all error types. Reuse the repository's error conventions. The loader's independent policy on empty files is not the parser-error wrapping defect; preserve or explicitly document that policy rather than silently changing it while fixing propagation.

Respect the accepted language direction: blocks only, no `[sections]`; structured string labels; ordinary root-relative references and explicit f-string interpolation; canonical boolean/null spellings `true`, `false`, and `null`. **Keep all numeric suffixes, SemVer values, SemVer requirement literals, and retained primitives.** Do not restore `$control` or unused BMLS schema features and do not introduce Stead-specific grammar. Separate syntax migrations are outside this fix; use valid fixtures for the current checkout and do not cement obsolete grammar.

## Regression tests and acceptance criteria

- Parse identical malformed content directly and through in-memory loader/file entry points. Use at least an unexpected token, EOF in an open block, and a lexical failure. Assert equivalent underlying typed error categories, relevant fields, and source spans; file errors must not be `Io` solely because parsing happened during a load.
- If a load-context wrapper is chosen, assert that its `source()` exposes the original error and retains its nested cause if present. If already-contextualized errors are forwarded directly, assert preservation of their original typed fields and existing source chain without requiring a new wrapper. Tests must inspect typed fields/accessors, not only rendered substrings. Also check that human-readable output remains useful with path and location.
- Load malformed files under different directories sharing a basename. Assert the exact distinguishable source paths and correct logical module identity, including a file merged into `main`. Use temporary fixtures inside the test environment, not machine-specific paths.
- Test valid multibyte UTF-8 before an error, multiline text/comments, LF and supported CRLF input, an error at EOF after a trailing newline, and a non-ASCII source filename where supported. Assert safe offsets and the project's documented line/column units; do not mix byte offsets with Unicode character columns or slice inside a code point.
- Verify both locations of a duplicate/unclosed-delimiter diagnostic survive the loader wrapper once those errors exist. Validation-on-load preserves a structured semantic failure rather than flattening it into `Io`.
- A missing path keeps the established not-found/I/O distinction; a genuine open/read failure remains I/O with context. Use deterministic injected `Read + Seek` failures rather than permission tests that behave differently as root. Invalid UTF-8 file bytes fail as a read/decoding error without panic or misleading syntax coordinates.
- Valid input loads unchanged, and an error does not publish a malformed module or successful parse-cache entry. Directory/discovery callers preserve the originating file error instead of obscuring it with another string-only wrapper.

## Validation and delivery

Run from the BarkML checkout:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Inspect the current manifest/CI and exercise additional supported feature combinations as appropriate. Report the public error/context representation, any compatibility implications, source/UTF-8 regression coverage, and actual command results. Identify pre-existing lint/test failures separately; do not claim the prior audit counts were reproduced unless they were genuinely rerun.
