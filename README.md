## BarkML

BarkML is a declarative configuration format that is inspired by other languages shuch as toml, hcl and more. It was
created initially to be used with operational tools and generative tooling. The language defaults to utf-8 parsing and
the language supports self-referential value references.


# Language Specification

## Statements

### Metadata

BarkML has no dedicated metadata or control syntax. Metadata is expressed with ordinary
blocks and assignments, and any schema interpretation belongs to the consuming
application — BarkML does not include a schema language or validate schemas.

**Example:**

```
stead {
    schema = 1.0.0
}
```

The block name and fields above are application-defined; BarkML reserves no names for
this purpose.

> **Migration (from ≤ 0.8.x):** `$` control statements such as `$control_id = !MyProgram 1.0.0`
> were removed and now produce a parse error. Replace them with ordinary assignments
> (`control_id = !MyProgram 1.0.0`) or application-defined blocks as shown above. `$`
> inside string literals is unchanged — strings never interpolate, so `"$HOME"` stays literal.

### Comments

BarkML supports the definition of comments by defining lines starting with # followed by one white space.
Any back to back lines with # will be concatenated into a multiline comment.

**Syntax:**

```
# <any text without a newline>
# <...>
```

**Example:**

```
# This is a comment in BarkML
```

### Blocks

BarkML supports grouping and labeling a set of statements as blocks. Blocks can have zero or more labels associated with them. Labels must be **literal strings**; numbers, booleans, null, arrays, tables, bytes, versions, and other value types are rejected as labels.

Block identity is **structured**: the block id plus the ordered sequence of labels. Component boundaries are preserved everywhere (parsing, child storage, lookup, macro resolution, serialization, diagnostics) — identities are never flattened into dot-joined strings, so `app "a.b"` and `app "a" "b"` are distinct.

**Syntax:**

```
<id> ["<label>" ...] {
  <child-statements...>
}
```

**Examples:**

```
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

**Lookup.** Use `Statement::get_child(id, &["label", ...])`, `Walk::walk_block(id, &["label", ...])`, or `Scope::lookup_segments` to address a block by its full ordered label sequence. `Walk::get_blocks(id)` returns each sibling's ordered label sequence. `Scope::lookup("a.b.c")` still accepts dotted paths for convenience: each dotted component matches a statement id *or* a label in order, so a label containing dots cannot be expressed in dotted syntax — use the segment-based APIs for those.

### Assignments

The standard assignment statement will define a single entry of data. Like
with control statements a value can be given an optional label as well, this can
be useful for declaring types for a value for the tool handling the config file.

**Syntax:**

```
[<id> | "<id>"] = (!<label>)? <value>
```

**Examples:**

```
foo = "bar"
"baz" = 3.14
```

## Values

### Integers

Integers are specified as numerical values and unless provided a suffix will be read as a signed 64-bit number.
Numbers can be specified with precision utilizing the following suffices

| Suffix | Precision         |
|--------|-------------------|
| u8     | Unsigned 8-byte   |
| u16    | Unsigned 16-byte  |
| u32    | Unsigned 32-byte  |
| u64    | Unsigned 64-byte  |
| u128   | Unsigned 128-byte |
| i8     | Signed 8-byte     |
| i16    | Signed 16-byte    |
| i32    | Signed 32-byte    |
| i64    | Signed 64-byte    |
| i128   | Signed 128-byte   |

**Examples:**

```
5
-2
5u32
-2i64
```

### Floating Point Numbers

Floating point numbers are read by default as 64-byte floating point unless one of the below suffixes are provided.
Floating point numbers
can be specified with exponents as well.

| Suffix | Precision |
|--------|-----------|
| f32    | 32-byte   |
| f64    | 64-byte   |

**Examples:**

```
3.14
3.14f32
-5.2
5.2e10
5.2e+10
3.1e-10
30.0E+2
```

### Semantic Versions

BarkML supports inline semantic version declarations. However to prevent collision with floating
point numbers, any semantic version must specify at least <major>.<minor>.<patch>

**Examples:**

```
1.0.0
0.1.0-beta.1
0.1.0-prerelease-build.5
```

### Semantic Version Requirements

BarkML also supports the definition of version requirements. The only exception is that currently
the wildcard '*' support is not explicitly supported (it is still inferred when not specifying every part of version).
You also must always specify a requirement operator in barkml.

```
>1.1
^4
~5.3
```

### String Values

Double-quoted strings are the canonical form. They never interpolate: `$HOME`, `${shell_var}`,
and `{braces}` are preserved literally. Escapes follow the documented rules: `\"`, `\\`, `\n`,
`\r`, `\t`, `\b`, `\f`, and `\u{HEX}` (1-6 hex digits forming a valid Unicode scalar). Any other
escape, a trailing backslash, or an unterminated string is a source-aware error.

Triple double quotes form multiline strings. Whitespace is verbatim: no dedenting, no trimming, and
opening/closing newlines and line endings are preserved exactly. Escapes are processed under the
same documented rules as single-line strings. A `"` may appear inside; the string ends at the
first `"""`.

Single-quoted strings remain as a compatibility form with their historical lenient escaping;
their contents are never interpolated either.

**Examples:**

```
"echo $HOME and {name}"
script = """
export EDITOR="nvim"
export PATH="$HOME/.local/bin:$PATH"
"""
'my-string'
```

### String Interpolation

An `f` prefix explicitly enables interpolation in a double-quoted or multiline string.
Placeholders use the same root-relative path grammar as reference expressions, including quoted
selectors (single-quoted inside placeholders, since a double quote would end the literal) and
array indices. `{{` and `}}` render literal braces in `f` strings; braces need no escaping in
plain strings.

Scalars interpolate as canonical text: strings as-is, numeric values without a width suffix,
`true`/`false`/`null`, and SemVer versions and requirements in canonical form. Arrays, tables,
bytes, and symbols are rejected with a type error. Missing targets, cycles, and depth limits
behave exactly like ordinary reference expressions.

**Examples:**

```
host {
    config_dir = "/etc/host"
}
target = f"{host.config_dir}/starship.toml"
configured_script = f"""
export EDITOR="{vars.editor}"
"""
quoted = f"{app['org.mozilla.firefox'].enabled}"
braces = f"{{literal}}"
```

### Byte Data

BarkML supports storing random byte data via base64 encoded byte strings. To avoid any confusion with what
standard of base64 is used between configuration files, BarkML standardizes on expecting URL Safe with No Padding
base64 data. This rust crate will automatically encode and decode any `Vec<u8>` data to and from this encoding standard.

**Examples:**

```
# binarystring in base64
b'YmluYXJ5c3RyaW5n'
```

### Labels

Label values are identifiers prefixed by !. These are primarily used before a value in an assignment statement
or control statement, but also can be used by themselves as a symbol value.

**Examples:**

```
!MyLabel
```

### Booleans

Booleans have exactly one spelling per value: `true` and `false`.

```
enabled = true
disabled = false
```

Former alias spellings (`True`, `TRUE`, `yes`, `on`, `no`, `off`, and case variants) are no
longer boolean literals. Like any other word, they now lex as identifiers, so in value position
they are treated as reference expressions; if no such reference exists, resolution fails with an
error rather than silently producing a boolean. Quoted strings such as `"yes"` or `'on'` are
unaffected and remain strings.

## Null

The only null spelling is `null`.

```
missing = null
```

Former aliases (`nil`, `none`, `Null`, `NULL`, and case variants) are now ordinary identifiers,
same as the removed boolean aliases: they resolve as references or error out, never as null.
Quoted strings containing these words are unaffected.

## Numeric and version literals (retained)

BarkML deliberately keeps typed numeric and version literals as first-class values; they are not
reduced to strings:

- Integer suffixes `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`, `u64`, `u128` with
  their width/sign range checks, plus unsuffixed integers and hexadecimal/octal/binary forms.
- Float suffixes `f32`, `f64`, and exponent forms.
- Native SemVer literals (`1.0.0`, `1.2.3-beta.1`) and SemVer requirements (`^1.2`, `>1.1`,
  `~5.3`) with all supported operators.

## Arrays

BarkML supports dynamic arrays, meaning that the type of the sub entry of any array does not
have to match. Arrays are always wrapped in `[]` and can contain 0 or more values delimited by commas

**Example:**

```
[5, 3.14, 'foo']
```

## Tables

BarkML also supports the definition of tables

**Example:**

```
{
  foo = 5,
  "bar" = 3.14
}
```

## References

BarkML supports self-referential references. At resolution time, references look up and replace
the value with the referenced value. References are root-relative paths: they resolve from the
root of the composed configuration, so forward references and reordering are fine, and a
reference preserves the referenced value's type. `self` and `super` carry no special meaning;
they are ordinary identifiers under the same root lookup.

**Example:**

```
version = "1.0.0"
section {
    val = 5
}
section-b {
    parent-version = version
    other-val = section.val
}
```

Quoted selectors keep punctuation inside a single component, address multi-label blocks, and
index arrays:

```
settings = app["org.mozilla.firefox"].settings
value = vars["key.with.dots"]
artifact-url = artifact["linux", "aarch64"].url
first = items[0]
```

Legacy `m!path` macro references and `m'...'` macro strings were removed; they fail with a
migration error pointing at the equivalent root-relative reference. Use `f"..."` strings for
interpolation: `m'{name}'` becomes `f"{name}"`, and `m!vars.editor` becomes the reference
expression `vars.editor` (or `f"{vars.editor}"` when a string is wanted). Note that legacy macro
strings stringified composite values, while `f` strings reject arrays, tables, bytes, and
symbols by design.

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This library is licensed under the MIT-0 License. See the LICENSE file.
