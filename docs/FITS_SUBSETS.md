# Combined FITS cuts and exact-ID subset export

Engine `0.1.0-alpha.23` adds combined scalar-column filtering and selected-column
CSV/TSV export in the interpreter and compiled execution. It does not add a
general exact-integer arithmetic type, a FITS writer, streaming output, or
physical blinding. No new dependency is needed.

## Start with the supplied catalogue

From the package root:

```console
goblin++ examples/fits_subset.gbl
goblin++ examples/fits_subset.gbl --compile
```

Native compilation requires Cargo and the cached locked dependencies, as with
the existing native FITS support. Outputs are new, hashed files inside each
run's `outputs` directory; the input catalogue is never edited.

## Explicit multi-column selection

```goblin
GO_PARANOID

file = "catalog.fits"
low = fits_where("Z", ">=", 0.4)
high = fits_where("Z", "<", 1.1)
good = fits_where("QUALITY", "==", 1)
galaxy = fits_where("CLASS", "==", "GALAXY")
special_id = fits_where("SOURCE_ID", "==", "9007199254740993")
either = fits_any([galaxy, special_id])
cut = fits_all([low, high, good, either])

selected = fits_export_csv("selected.csv", file, 1, ["SOURCE_ID", "Z", "CLASS"], cut)
print("selected rows = {selected}")
seal selected
```

This selects `(Z >= 0.4 AND Z < 1.1 AND QUALITY == 1 AND
(CLASS == "GALAXY" OR SOURCE_ID == 9007199254740993))`.
Choose the science cuts yourself: no survey quality rule, weighting, or
cosmological meaning is inferred. This example requires those named columns;
it is not the schema of the supplied `sample.fits`.

`fits_all` means AND and `fits_any` means OR. Both require a nonempty array of
filters. The builders return independent, versioned filter text; they do not
read a file, execute arbitrary code, or return an array of rows. Store and use
their results. An empty group refuses rather than silently exporting all rows.

## Function contract

| Function | Result |
| --- | --- |
| `fits_where(column, operator, value)` | Predicate for `==`, `!=`, `<`, `<=`, `>`, `>=` |
| `fits_where(column, "is_null")` | Predicate selecting null/NaN cells |
| `fits_where(column, "is_valid")` | Predicate selecting non-null cells |
| `fits_all(filters)` / `fits_any(filters)` | Explicit AND/OR group |
| `fits_export_csv(name, file, hdu, columns, filter)` | Creates CSV; returns selected row count |
| `fits_export_tsv(name, file, hdu, columns, filter)` | Creates TSV; returns selected row count |
| `fits_column_text(file, hdu, column, row)` | Scalar cell text, with exact unscaled integer IDs |

HDU and row indices are zero-based. Column lookup is case-insensitive; output
headers use the catalogue's column names in the requested projection order.
Missing, ambiguous or duplicate projected columns refuse, even on an empty
table. Comparisons use physical scaled values, not raw bytes. FITS `TUNIT`
metadata is recorded but not interpreted; numeric bounds must be dimensionless
and already expressed in the column's physical units.

Supported cells are scalar real numbers, logical values, and fixed ASCII text.
Vector, complex, bit and variable-length columns refuse. Text comparison is
case-sensitive lexicographic comparison after removal of FITS padding spaces.
Logical comparisons require `true`/`false` and `==`/`!=`; numeric `1` is not
silently coerced into `true`.

## Exact catalogue IDs

Unscaled FITS `B`, `I`, `J` and `K` integer columns are decoded as integers,
compared as integers and exported as exact decimal text, including both signed
64-bit extremes. Adjacent IDs above 2^53 remain distinct.

```goblin
id = fits_column_text("gaia.fits", 1, "SOURCE_ID", 0)
cut = fits_where("SOURCE_ID", "==", id)
count = fits_export_csv("one_source.csv", "gaia.fits", 1, ["SOURCE_ID"], cut)
print("source = {id}; matching rows = {count}")
seal id
```

Use quoted signed 64-bit decimal text for large integer bounds. Small numeric
bounds on integer columns must be integral and within ±(2^53−1). Text bounds
on floating/scaled columns refuse; no implicit text-to-number conversion occurs.
Existing `fits_column` and numeric FITS analysis now refuse unsafe integer cells
instead of returning rounded numbers. They remain f64 quantity APIs.

This is exact **transport and selection**, not general 64-bit arithmetic:
`id` above is a string. Do not call `parse_number(id)` or load an exported ID
column with `csv_numbers`; use `csv_column` / `tsv_column` for exact text.
Scaled integer values beyond safe f64 precision refuse, including conventional
unsigned-64 FITS offsets; arbitrary exact scaled/unsigned integer support is
still pending. Floating/scaled science values retain shortest-round-trip f64
serialization, including signed zero; authoritative output is not quantized.

## Nulls and invalid data

- Numeric `TNULL`, NaN, and missing logical bytes are null. Infinity and invalid
  logical/text bytes are errors, not missing values to silently skip.
- Every ordinary comparison with null, including `!=`, is false. Include nulls
  deliberately with `fits_where(column, "is_null")` in an OR group.
- All filter leaves are evaluated for every scanned row, so null counts and
  malformed predicate-cell failures do not depend on short-circuit ordering.
- Null output cells use the reserved marker `\N`. A genuine text cell or column
  name equal to that marker refuses to avoid an ambiguous export. The receipt
  records the marker and per-column null counts; these are not silently empty
  strings. CSV quoting preserves commas, quotes and line breaks. TSV refuses
  text containing tabs/newlines instead of changing it.
- A valid selection of zero rows succeeds with a header-only file and count 0.
  The caller decides whether an empty scientific sample is acceptable.

## Evidence and limits

Each generated artifact has an exact output SHA-256. Its metadata includes
input hash, HDU, full filter, predicate/projection column layout and units,
input/selected/rejected counts, predicate-null row count, output null counts,
stable original row order, serialization policy and resource limits. The
source, compiled support source and input evidence use existing custody rules.
Verify the run independently with `goblin++ verify RUN_DIR`.

The scan uses chunks of at most 8 MiB, handles more than 100,000 rows without
creating a Goblin array, and does not spend language loop-body iterations.
Rows must occupy 1 byte to 8 MiB. Filters permit at most 16 levels, 256 nodes
and 65,536 encoded bytes. Individual `fits_column_text` cells retain the 1 MiB
text limit. The projected output has the existing 64 MiB per-file
limit and is accumulated in memory; exceeding the limit refuses, never writes
a truncated subset. Several successful outputs can still occupy substantial
memory. Larger/streaming output remains a separate backlog item.
The existing CSV/TSV readers still limit input to 16 MiB, 100,000 data rows
and 1,000,000 cells; a larger valid export can exceed those readback limits.
This stage does not claim to remove all large-catalogue workflow limits.

## Extraction is not physical blinding

The projection is explicit but is **not** an `allow_columns` access policy.
The extractor can access the original catalogue and its run evidence retains
that full catalogue. Its receipt states `physical_blinding=false`.

A future blind analysis worker must receive only the approved subset and
manifest, with the original catalogue and extractor evidence held elsewhere
under tested access controls. Inline Rust and undeclared file access must not
be an escape hatch. See [WB3_ROADMAP.md](WB3_ROADMAP.md); `GO_MAD` is not yet
implemented. This extraction step does not claim that WB-3 runs end-to-end.
