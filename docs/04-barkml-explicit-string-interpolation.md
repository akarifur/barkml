# BarkML: explicit interpolation and a coherent string model

## Task and context

Implement this approved syntax change in https://github.com/akarifur/barkml. The target use includes dotfiles containing shell variables, braces, quotes, and multiline application configuration. Plain strings must remain literal unless interpolation is explicitly requested.

Baseline: BarkML 0.8.5, commit `9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76`. Inspect the current checkout and instructions before editing; account for the reference and structured-label changes from prompts 02 and 03.

## Required string model

Use these canonical forms:

```text
literal = "echo $HOME and {name}"
target = f"{host.config_dir}/starship.toml"

script = """
export EDITOR="nvim"
export PATH="$HOME/.local/bin:$PATH"
"""

configured_script = f"""
export EDITOR="{vars.editor}"
"""
```

- Double-quoted strings support escapes and do not interpolate.
- Triple-double-quoted strings support multiline text with the same documented escape rules.
- An optional `f` prefix explicitly enables interpolation in either form.
- Replace legacy `m'...'` interpolation rather than maintaining two macro systems.
- Use these forms in new examples and emitted BarkML. Document migration of old single-quoted text; do not accidentally reinterpret its contents. Whether to retain non-interpolated single-quoted strings as a compatibility form is a separate compatibility decision, not a reason to retain legacy macro strings.

Define escaping precisely before implementation: quotes, backslashes, newline/tab/carriage-return escapes, Unicode handling, and invalid/trailing escapes. The exact escape repertoire is an open design decision; a proposed default is `\"`, `\\`, `\n`, `\r`, `\t`, `\b`, `\f`, and Rust-style `\u{HEX}` with 1-6 hex digits representing a valid Unicode scalar. Confirm the chosen grammar, including treatment of delimiter quotes inside multiline strings, before freezing syntax. Reject malformed strings with source-aware diagnostics. Do not silently dedent or trim multiline content; document how opening/closing newlines and line endings are preserved.

Preserve the separate byte-literal feature and its encoding rules; do not remove it accidentally while changing text-string lexing.

## Interpolation semantics

Interpolation initially accepts reference paths, not arbitrary expressions or calls. It must use the same path parser/resolver as ordinary references, including quoted selectors. Document and test quoting inside interpolation placeholders.

The following are **proposed defaults requiring confirmation**, not additional approved requirements:

- Use `{{` and `}}` for literal braces in interpolated strings. Braces need no escaping in plain strings.
- Interpolate string values as text; numeric values as their canonical value without a BarkML width suffix; booleans/null as `true`, `false`, and `null`; SemVer values and requirements using their canonical textual representation.
- Reject arrays, tables, bytes, and symbols in interpolation with a useful type error until an explicit rendering/encoding policy is chosen. This proposal does not remove those value types or their literal syntax.

Existing macro interpolation stringifies composite values, so changing that behavior is a compatibility decision, not merely replacing a prefix. Confirm a complete rendering policy for every retained value category and document migration from old output; do not silently adopt the proposed restrictions. Whatever policy is chosen, ordinary reference expressions must preserve types rather than stringify, and formatting must not accidentally use Rust debug output.

Preserve source spans for literal fragments and placeholders. Missing targets, cycles, and resolution limits must behave consistently with prompts 03, 10, and 11.

## Acceptance criteria and tests

- Plain strings preserve `$HOME`, `${shell_var}`, and `{braces}` literally.
- Ordinary quotes/backslashes and documented Unicode escapes round-trip correctly.
- Multiline strings preserve specified whitespace and newlines without implicit dedenting.
- `f` strings resolve multiple and repeated placeholders, including punctuation-bearing selectors.
- Literal braces in interpolated strings use the documented escaping rule.
- Missing braces, incomplete placeholders, invalid escapes, and unterminated single-line/multiline strings fail without panic.
- Referenced typed numbers and version values produce documented scalar text.
- The confirmed rendering or rejection policy is tested separately for strings, numeric types, booleans, null, SemVer, requirements, arrays, tables, bytes, and symbols where retained. Compatibility changes from old macro output are documented.
- All existing byte, numeric, SemVer, and requirement literal tests remain valid.
- Examples, serializers, and public docs use the new interpolation syntax consistently.
- Legacy `m'...'` interpolation is rejected with a migration-oriented error, not retained as an undocumented second interpolation syntax. Test this independently from removal of `m!`.

Run `cargo fmt --check`, `cargo test`, and applicable Clippy checks. Add regression tests around tokenizer boundaries, not only successful interpolation.

## Starting points

- [Existing string lexer rules](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/syn/lexer.rs#L194-L218)
- [Existing interpolation implementation](https://github.com/akarifur/barkml/blob/9a6a487374ec28ab9a7b8de8858db6bb8e6bcd76/src/ast/scope.rs#L178-L269)
