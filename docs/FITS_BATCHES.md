# Serial FITS batches — alpha.32

Read a large **uncompressed fixed-width binary table** a batch at a time, process
its cells, write complete output rows incrementally, release the batch, repeat.
The interpreter and native compiler use the same checked reader. No implicit
filters, sampling, row drops, threads, FITS writer, gzip reader or joins are added.

## Start with the bundled example

From the package root:

```sh
goblin++ examples/fits_batches.gbl
goblin++ examples/fits_batches.gbl --compile
goblin++ verify "<RUN_DIR>"
```

Replace `<RUN_DIR>` with the actual directory printed by that run. `verify`
checks the preserved program, inputs, seals, outputs and policy links. A preserved
failed analysis can verify successfully: integrity is not scientific success.
The example uses the supplied synthetic `sample.fits`, not a scientific verdict.

## API

```goblin
reader = fits_batch_open("catalogue.fits", 1, ["SOURCE_ID", "Z"], 4096, 8388608, 2147483648)
writer = csv_stream_open("selected.csv", ["SOURCE_ID", "Z"], 2147483648)
while true {
    batch = batch_next(reader)
    if len(batch) == 0 { break }
    written = stream_write(writer, batch)
    batch = []
}
batch_close(reader)
stream_close(writer)
```

`fits_batch_open(path, hdu, columns, rows, batch_bytes, file_bytes)` takes six
required arguments. HDUs are zero-based (the primary HDU is usually 0; the first
catalogue extension is often 1, but inspect your file rather than assuming).
Column names are **exact and case-sensitive**, distinct and in caller-selected
order. The other arguments are explicit positive integer budgets.

`batch_next` returns a flat row-major array of text. For two columns, offsets
0/1 belong to the first row, 2/3 to the second, and so on. An empty batch means
observed EOF, never a silently skipped malformed row. All readers must reach EOF
and close; all writers must close before PASS. `tsv_stream_open` is also available.
These writers quote delimiters, quotes and newlines rather than silently cleaning
text. Reader handles are run-local identifiers, not security capabilities.

## Values, missing data and units

- Unscaled `B/I/J/K` integers become exact decimal **text**, including signed
  64-bit IDs above 2^53. Keep IDs as text; do not call `parse_number` on them.
- `E/D` floats and safely representable scaled integers become shortest-roundtrip
  finite f64 text. Scaling follows TSCAL/TZERO. Large scaled integer IDs refuse;
  they are never approximated. Infinity/overflow refuses with HDU/column/row context.
- TNULL, floating NaN and supported logical-null bytes produce the marker `\N`.
  No row is dropped. In source, compare with `"\\N"`. Count or explicitly handle
  missing cells before numeric conversion. Literal ASCII text equal to this marker
  refuses to avoid ambiguity. Empty ASCII text stays empty, not null.
- Logical values become `true` or `false`. Fixed-width ASCII text is checked for
  ASCII/NUL validity and right-trimmed of FITS space padding; other text is preserved.
- TUNIT and the selected column's type/scaling/null metadata are recorded in the
  receipt. **No unit is automatically converted or attached to the returned text.**
  Check the dataset schema, then explicitly attach the known unit when converting
  a measurement, for example `distance = parse_number(cell) * (1 m)` for metres.
- Variable-length fields, numeric vector fields, complex/bit fields, ASCII TABLE
  HDUs, images and tile-compressed image tables are not this API's supported input.
  Supported scalar fields may be projected while unsupported unselected fields
  remain untouched. Requested duplicate or missing column names refuse.

## Resource and custody contract

The existing serial-table lifecycle policy stays `goblin.serial-table-batches.v1`;
the additional FITS policy is **`goblin.serial-fits-batches.v1`**.

- Input/output files: caller-selected budget up to 64 GiB each.
- Returned batch: caller-selected decoded allocation budget up to 8 MiB and
  100,000 cells. The requested row count must fit the selected column count.
- Raw row: 1 byte to 1 MiB. Raw read buffer: about 64 KiB, or one wider row.
- Metadata: at most 128 HDUs and 1 MiB total padded raw headers. Duplicate valued
  headers refuse; selected column names must be unambiguous. These are input-size
  bounds, not a claim that parsed metadata occupies only 1 MiB.
- At most four open readers, four open writers, 128 total handles per run, shared
  with CSV/TSV. Existing eager/scan limits and `GO_LOOP_BUDGET` remain unchanged.

A private full disk snapshot is hashed and checked before parsing; processing
reads that snapshot, not a moving original. Full snapshot bytes are preserved
in run evidence. This needs disk space for input and custody copies even though
RAM does not hold the whole catalogue. Opening the same input twice or combining
batch and eager/FITS/scan readers for it in one run refuses.

The receipt records the input hash/size, HDU, selected columns, column metadata,
requested budgets, delivered row/batch counts, observed decoded peak and lifecycle.
Verification binds the preserved FITS schema and total rows back to those records,
in addition to byte hashes; this is **not replay of arbitrary science arithmetic**.
Native runs compare reference/native I/O evidence and output hashes. Standalone
native programs preserve their snapshots and manifests too.

Late failures preserve generated stream outputs with `.partial` names and
`complete=false`, even if a writer had already closed. There is no automatic
resume, truncation recovery or silent row skipping. Failed open/metadata validation
after a successful snapshot preserves that input and a failed reader summary.

Release user arrays before the next read: `batch = []`, and release any derived
arrays too. The reader itself does not retain returned batches. A program can
still accumulate arrays, seals or printed text: these budgets are **not a
whole-process RAM sandbox**. This serial stage makes no parallelism guarantee.

## Freeze and compatibility

New freezes pin the FITS batch policy. Existing alpha.31-and-earlier non-FITS-batch
receipts/freezes still verify without this field. An older freeze cannot adopt
`fits_batch_open` without an explicit revision and child freeze. Alpha.32 evidence
cannot downgrade by removing its required policy. New builtin names cannot
silently override an old user function; historical canonical parsing is retained,
but execution collisions refuse explicitly.

The [actual alpha.31 compatibility gate](verification/FITS_BATCH_COMPATIBILITY_alpha32.json)
uses the hash-checked archived engine to create interpreter/native energy, CSV
batch and plot evidence, then verifies it with alpha.32. It also checks an old
energy freeze and explicit revision of an old unpinned FITS-batch freeze.

## Limits of the guarantee

The snapshot contains the **whole input**, including unselected columns. Projection
is not physical blinding or encryption. SHA-256 and local custody chains establish
checksum integrity, not authenticated authorship or scientific correctness.
`GO_PARANOID` adds checks; it does not sandbox inline Rust or external processes.

Use `tools/check_fits_batch_memory.py` for the optional synthetic large-file/RSS
regression gate. Measurements describe that tested workload and platform, not
all programs or a certified end-to-end WB-3 analysis.

The [local alpha.32 measurement](verification/FITS_BATCH_MEMORY_alpha32.json)
projects/exports 268,960 rows from a 1,075,847,040-byte synthetic FITS file,
preserves exact IDs and independently verifies the run. Peak child RSS for that
largest fixture was 9,650,176 bytes on macOS arm64; the 2/64 MiB fixtures stayed
below 12 MiB. These are measured fixtures, not guaranteed RAM use for any program.

Spread the word with optional [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).
