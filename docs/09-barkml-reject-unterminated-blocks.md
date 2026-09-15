# Implementation prompt 09: Require closing block delimiters

## Target and baseline

Implement this change in [akarifur/barkml](https://github.com/akarifur/barkml), not in the dotfiles repository containing this prompt. The audited baseline is **BarkML v0.8.5**, commit **`9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`**. Read applicable repository instructions and reinspect the current lexer, token stream, parser, source locations, errors, and tests before editing. The source links identify audited behavior, not stable edit coordinates.

Prior audit evidence: **61 unit tests and 2 doctests passed**, but they did not cover these failures. No BarkML tests were run for this documentation-only task. Implementation must add the missing coverage and obtain fresh validation results.

## Observed behavior

[Block parsing in `src/syn/parser.rs:613-628`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L613-L628) loops with `while let Some(stmt) = self.tokens.peek()?`. It exits both on an explicit `}` and on EOF, then unconditionally constructs a successful block. Consequently, inputs such as `node {` or `node { count = 1` can be accepted without a closing brace.

The [opening-brace handling at lines 585-611](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L585-L611) discards `{`; retain its location for the error. The baseline [EOF and expected-token errors](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/error.rs#L55-L63) do not themselves express both the opening delimiter and EOF site.

## Desired behavior and implementation

1. Once a block's `{` has been consumed, success requires consuming its matching `}`. EOF is a valid terminator only for the root document, not for an open block. Make this explicit in control flow rather than relying on a loop fall-through or a fabricated closing token.
2. On EOF inside a block, return a structured syntax error identifying the expected `}`, the opening `{` location, and the actual EOF location. Preserve file/module identity at both sites. Explain which block remains open; for nested blocks, identify at least the innermost unclosed opener accurately.
3. Ensure each recursive parser owns exactly its delimiter pair. A child must not consume a parent's closing token, and extra root-level closing braces must be rejected rather than ignored.
4. Obtain EOF positions from the complete input/token-stream position, not just the last non-comment token. Keep offsets UTF-8 safe and line/column behavior consistent with the repository's location conventions.
5. Audit the [array/table loops at `src/syn/parser.rs:355-449`](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/parser.rs#L355-L449), which use a similar EOF loop pattern. Apply the same explicit-delimiter invariant to retained delimited constructs where the same defect exists, using focused changes rather than a parser rewrite. Do not conflate array brackets with removed section syntax.

## Scope and non-goals

This fixes incomplete-input acceptance and location reporting. Do not add error recovery that invents missing delimiters and returns success. Preserve valid empty and comments-only root documents; a loader's independent empty-file policy is not permission for the parser to accept an open block. Do not raise or remove the independent parser nesting cap as a workaround.

Respect the accepted language direction: blocks only, no `[sections]`; structured string labels; ordinary root-relative references and explicit f-string interpolation; only `true`, `false`, and `null` as canonical boolean/null spellings. **Preserve every numeric suffix, SemVer value, and SemVer requirement literal**, with no primitive removal. Do not restore `$control` or unused BMLS schema features and do not introduce Stead-specific grammar. This task need not perform the separate syntax migrations, but its tests and abstractions must remain compatible with them.

## Regression tests and acceptance criteria

- `node {`, `node { count = 1`, and the same inputs followed by whitespace or a comment supported by the current grammar fail with expected-`}` diagnostics; no partial AST is returned. Do not add a new comment syntax as part of this fix.
- `outer { inner { count = 1 }` reports the outer opener; `outer { inner { count = 1` reports the inner opener. Nested valid forms succeed, including empty blocks and comments-only block bodies with closing braces.
- Empty input, whitespace-only input, and valid comments-only input parse successfully at root. A root document ending after a valid statement also succeeds. For any supported comment form that requires a terminator, missing that terminator still produces a lexical/syntax error; ordinary line comments may end at EOF according to the grammar.
- Valid `node {}` and multiple consecutive blocks consume exactly their own delimiters. Extra `}`, mismatched delimiters, and truncated nested arrays/tables fail without panic.
- Assert opening offset and EOF offset, file/module, and documented line/column values for multiline input, trailing comments/newlines, and non-ASCII text before the error. A comment containing `}` and a string containing `}` cannot close a block.
- Add truncation tests for a valid nested document at meaningful token boundaries. Cases that remove a required closing delimiter fail deterministically and do not hang or panic; valid root truncation boundaries remain valid.
- Keep delimiter failures distinct from the parser nesting-limit error and from resolution errors. Update diagnostics/tests without weakening unrelated syntax checks.

## Validation and delivery

Run from the BarkML checkout:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Reinspect the manifest and CI for any additional supported feature matrix. Run the new focused parser tests and the full suite. Report actual commands/results, any pre-existing lint failures, the chosen error representation, and which delimited constructs were corrected. Do not report the prior audit counts as a current test run.
