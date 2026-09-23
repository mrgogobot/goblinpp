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

`+` joins two text values; it still adds two quantities. Mixing text and a quantity is an error: use `to_text(value)` explicitly. `to_text` renders quantities, Booleans, or text for labels—not lossless serialization of scientific values. It refuses arrays; use `str_join` for an array of text. `len(text)` counts Unicode scalar values—not UTF-8 bytes or user-perceived grapheme clusters. Thus `len("π")` is 1, while `len("é")` (letter plus combining mark) is 2. `len(array)` still counts elements. Text operations are case-sensitive, do not normalize Unicode, and do not change existing source hashing rules.

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
