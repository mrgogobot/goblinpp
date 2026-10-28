# Text and `g_strings`

Text was already a Goblin++ value type. This stage adds ordinary operations instead of introducing a separate `g_strings` syntax or type:

```goblin
first = "Ada"
last = "Lovelace"
name = first + " " + last
count = len(name)
print("{name} has {count} characters")

raw = " 2.5 "
number = parse_number(raw)
count = parse_integer("42")
answer = number * 2
print("answer = {answer}")
```

`+` joins two text values; it still adds two quantities. Mixing text and a quantity is an error: use `to_text(value)` explicitly. Since alpha.21, default finite numeric text round-trips the stored f64, including signed zero. Unitless text can be read back with `parse_number`; unitful text includes SI units and is not accepted by that function. See [numeric text](NUMERIC_TEXT.md). `to_text` also renders Booleans or text, but refuses arrays; use `str_join` for an array of text. `len(text)` counts Unicode scalar values—not UTF-8 bytes or user-perceived grapheme clusters. Thus `len("π")` is 1, while `len("é")` (letter plus combining mark) is 2. `len(array)` still counts elements. Text operations are case-sensitive, do not normalize Unicode, and do not change existing source hashing rules.

Since alpha.20, source-literal placeholders capture values immediately, once.
Stored/user/data text is never expanded again during output. Use `"{{name}}"`
for literal `{name}`. See [protected values and migration](PROTECTED_VALUES.md).

The `g_strings` built-ins are:

| Call | Result |
|---|---|
| `str_trim(text)` | Copy without leading/trailing Unicode whitespace |
| `str_contains(text, needle)` | Boolean substring test |
| `str_replace(text, old, new)` | Copy with all non-overlapping matches replaced; `old` cannot be empty |
| `str_split(text, separator)` | Array of text; preserves empty fields; separator cannot be empty |
| `str_join(separator, text_array)` | Text assembled from a one-dimensional array of text |
| `to_text(value)` | Human-readable text rendering |
| `parse_number(text)` | Dimensionless finite floating-point quantity |
| `parse_integer(text)` | Dimensionless exactly representable integer quantity |

`parse_number` trims surrounding whitespace and accepts ASCII decimal syntax with an optional sign, decimal point, and exponent, such as `-3`, `.25`, or `+1.2e3`. It rejects units, embedded whitespace, `NaN`, infinity, overflow, and a nonzero value that underflows to zero. Plain integer spellings outside ±(2^53−1) are refused rather than silently rounded. Decimal/scientific values still have normal `f64` precision limits. There is **no distinct exact integer type** yet; do not use `parse_number` for large catalogue identifiers. Invalid conversion produces a preserved run failure, never a guessed value. Scientific units must be written explicitly in Goblin++ expressions, not smuggled through text conversion.

`parse_integer` accepts only optional `+` or `-` followed by ASCII decimal digits. It refuses decimal points, exponents, units, separators, and values outside ±(2^53−1). The result is still stored as a dimensionless `f64` quantity; this function provides checked syntax and range, not an arbitrary-precision integer type.

New text-producing operations are limited to 1,048,576 UTF-8 bytes per result; split/join support at most 100,000 parts. Both interpreter and native-compiled execution use the same core text implementation and are covered by parity and receipt-verification tests. Source literals, input, arguments, and output evidence retain their existing limits and privacy rules. `input()` and `argv()` still return text; choose `parse_number(...)` or the stricter `parse_integer(...)` explicitly.

## Alpha.40: Unicode, byte sizes, ordering and base conversions

All calls below return independent values in both engines. They obey the existing
1 MiB text limit. They do not mutate variables, normalize Unicode, interpolate
returned text, or change existing `==`/`!=` semantics.

| Call | Contract |
|---|---|
| `str_upper(text)` / `str_lower(text)` | Unicode case conversion using the retained Rust toolchain's tables. Whole-string lowercase handles Greek final sigma. No locale-specific tailoring; mappings can expand text. |
| `str_bytes_len(text)` | UTF-8 payload byte count. No terminator, spare capacity or object overhead. |
| `sizeof(text)` | Exact alias for `str_bytes_len`; text only. Other types refuse. This is **not** C's `sizeof`, allocated memory usage or a file size query. |
| `str_compare(left, right)` | `-1`, `0` or `1`: case-sensitive lexicographic UTF-8 byte order (also Unicode scalar order for valid UTF-8). No natural-language collation, normalization or case folding. |
| `str_to_hex(text)` / `str_from_hex(text)` | Encode/decode UTF-8 bytes as two hex digits per byte. Output lowercase; decode accepts either case. |
| `str_to_octal(text)` / `str_from_octal(text)` | Encode/decode UTF-8 bytes as three octal digits per byte. |
| `str_to_binary(text)` / `str_from_binary(text)` | Encode/decode UTF-8 bytes as eight binary digits per byte. |
| `to_base(integer, base)` | Integer to sign-and-magnitude text, base `2`, `8`, `10` or `16`; no prefixes/padding, lowercase hex. Negative zero renders `"0"`. |
| `from_base(text, base)` | Signed integer digits to a dimensionless numeric value; trims outer whitespace. |

Text codecs use complete fixed-width byte groups with no prefixes, signs,
whitespace or separators. Decoders reject out-of-range bytes (e.g. octal `400`)
and invalid UTF-8 instead of inserting replacement characters. Empty text and
embedded NUL bytes round-trip. This is encoding, **not encryption**.

Integer conversions require dimensionless scalars within ±(2^53−1), with no
fractional part. Parsers reject prefixes such as `0x`, separators and exponents.
They do not create an exact 64-bit integer type or reinterpret IEEE 754 bits.
Large catalogue IDs should remain exact text. Existing `parse_number` and
`parse_integer` decimal contracts are unchanged.

```goblin
name = "Ada π"
characters = len(name)                  # 5 Unicode scalars
bytes = str_bytes_len(name)             # 6 UTF-8 bytes
encoded = str_to_hex("Aπ")             # "41cf80"
restored = str_from_hex(encoded)        # "Aπ"
hex_integer = to_base(65, 16)            # "41"
integer = from_base(hex_integer, 16)    # 65
ordering = str_compare("Ada", "Grace") # -1
print("{characters}; {bytes}; {restored}; {integer}; {ordering}")
```

Use `str_compare(a, b) < 0` for ordering. Existing direct text `<`, `<=`, `>`
and `>=` remain refused, preserving historical behavior. See the runnable
[`text_toolkit.gbl`](../examples/text_toolkit.gbl).
