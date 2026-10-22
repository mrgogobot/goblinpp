# Exact-ID table joins — alpha.38

Combine two bounded CSV catalogues by exact text ID:

```goblin
GO_PARANOID
rows = csv_join("joined.csv", "sources.csv", "measurements.csv", "source_id", "source_id", "left", "source_", "measurement_")
print("joined rows = {rows}")
seal rows
```

Eight required text arguments, in order:

1. Output basename ending in `.csv` (`.tsv` for `tsv_join`).
2. Left input filename.
3. Right input filename.
4. Exact left key header.
5. Exact right key header; it need not have the same name.
6. Join kind: exactly `"inner"` or `"left"`.
7. Nonempty left column prefix, at most 128 UTF-8 bytes.
8. Nonempty right column prefix, at most 128 UTF-8 bytes.

`tsv_join` has the same signature, using tab delimiters for both inputs and
output. Mixed-delimiter joins are not included. Input paths are relative to the
entry program's directory, as with existing table readers; absolute paths are
permitted. Neither function overwrites an input or an existing output.
Generated files belong to the fresh `RUN_DIR/outputs/`, not the source folder.
The return value is the number of output data rows. Calls may also stand alone
when only the file is wanted.

## Exact identity and cardinality

- IDs remain UTF-8 text, never f64. `9007199254740992` and
  `9007199254740993` stay distinct, as do `0007`, `7`, `7.0` and `7e0`.
- No trimming, case folding or Unicode normalization. `A` differs from `a`;
  leading spaces are significant. The decoded field is compared, so quoting
  syntax itself is not part of an ID. Original input bytes are still hashed.
- Version 1 is **one-to-one only**. A duplicate on either side is an error,
  even when that ID would not match. Errors identify the side and 1-based data
  row numbers (header excluded; multiline records count as one data row).
- An empty key or the literal two-character FITS-export null marker `\N`
  is an error on either side. No null-to-null match, missing-row skip, first-row
  choice or many-to-many expansion is allowed. Whitespace-only keys are not
  empty under exact text semantics.
- `inner` deliberately emits only matches; excluded left/right row counts are
  preserved. `left` emits every left row. Both preserve **left input order**,
  independently of right ordering. Right-only rows are counted, not emitted.

All original columns, including **both key columns**, are retained, in left
then right header order, with the explicit prefixes. Duplicate resulting
headers are refused, not automatically renamed. Each output starts with
`__join_matched`: text `true` for a matching right row, `false` otherwise.
Unmatched right fields are empty. Use this flag to distinguish an absent right
row from an existing row with empty cells. Non-key empty cells and `\N` text
remain unchanged. Empty catalogues with valid headers produce valid header-only
outputs where appropriate.

CSV **and TSV** output quote embedded delimiters, quotes and line breaks. TSV
joins do not replace tabs/newlines with spaces. This is deliberately stricter
than the older general-purpose `write_tsv` sanitizer, which is unchanged.
No type inference, physical-unit conversion or spreadsheet-formula sanitizing
is applied: opening untrusted scientific text in a spreadsheet still requires
care. A successful join is an identity operation, not scientific validation.

## Bounds and preserved evidence

This is an eager, serial implementation, not an out-of-core join. Each input
must be an ordinary non-symlink UTF-8 file no larger than 16 MiB, with unique
nonempty headers, at most 100,000 data rows, 1,024 columns and 1,000,000 total
cells including headers. The joined output has these same bounds. Exceeding
them refuses the whole join; no partial join table is published. Existing batch
readers remain the route for large inputs, but cannot be mixed with eager
reads of the same input in one run. Parent-directory symlinks and filesystem
races are not an OS sandbox or hostile-host isolation boundary.

Each generated artifact's receipt metadata records the versioned join policy,
both input hashes/byte counts, key names, prefixes, cardinality, missing/duplicate
rules, order, output headers, matched rows, unmatched rows on **both** sides,
deliberately dropped left rows and output rows. The outer artifact descriptor
hashes the finished table bytes. Raw inputs and role-specific access records
are preserved as ordinary run input evidence. Original catalogues need not
remain available for verification.

```sh
goblin++ run examples/table_joins.gbl
goblin++ run examples/table_joins.gbl --compile
goblin++ verify <RUN_DIR printed by the command>
```

Expected: left join 4 rows, inner join 3 rows; each report has 3 matches,
1 unmatched left row and 1 unmatched right row. In particular `0007` must not
match `7`. The two IDs above 2^53 must receive different measurements.

`TABLE_JOIN_RECONSTRUCTION` rebuilds each joined table from preserved snapshots
and rules and checks bytes, hashes, headers and counts—not merely a receipt
checksum. Native execution uses the same reviewed join implementation and its
output/evidence parity gates; it is not an independent algorithm. Tests also
compare independently written literal expectations. Existing legacy receipts,
non-join programs and numeric behavior are retained.

This remains checksum-based integrity, **not authenticated authorship**.
Someone able to replace every source, input, output and receipt can fabricate a
different internally consistent history. Independently retained receipt hashes
or authenticated provenance are still needed for that threat model. Joins do
not implement physical blinding, table FITS writing, streaming/many-to-many
joins, `COMPLETELY_MAD`, encryption or independently validated WB science runs.
