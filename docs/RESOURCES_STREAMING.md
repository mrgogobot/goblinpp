# Audited execution budgets and streaming tables (alpha.27)

Goblin++ keeps finite resource limits. A larger analysis can explicitly request a
larger **shared loop-body entry budget**, without disabling evidence preservation:

```goblin
GO_PARANOID
GO_LOOP_BUDGET 50000000

total = 0
for index in range(400000) {
    total = total + 1
}
print("processed = {total}")
seal total
```

The default is 1,000,000 entries; the hard maximum is 50,000,000. The declaration
accepts one unsigned decimal integer from 1 through 50,000,000. It may occur only
once, at top level in the main source. It is a run-wide configuration, not an
instruction that resets a counter when reached. Modules, functions, loops and
conditional blocks cannot declare their own budgets.

All `for`, array `for` and `while` loops, including loops in called functions,
share one counter. Entering an outer loop body and an inner loop body counts as
two entries. `break` and `continue` do not undo entries. A loop stopped by its
condition uses no additional entry. The next entry after the budget is exhausted
is refused with `G203`, and the failed run remains preserved. Interpreter and
generated native code enforce the same limit; the launcher checks their exact
counter parity. A standalone compiled program retains its declared limit.

This is an iteration bound, **not a wall-clock timeout, memory quota or sandbox**.
Inline Rust is not made safe by a loop budget. Scientific helper functions have
their own separately declared bounds; this counter measures source-level loop
body entries, not hidden arithmetic operations.

## Numeric CSV/TSV scanning without a whole-table array

```goblin
GO_PARANOID
summary = csv_scan_stats("measurements.csv", "temperature")
rows = summary[0]
total = summary[1]
average = summary[2]
minimum = summary[3]
maximum = summary[4]
print("rows = {rows}; mean = {average}")
seal summary
```

`tsv_scan_stats(path, column)` uses a tab delimiter. Both calls require **exactly
two text arguments**. The case-sensitive column name must match a header exactly.
They return five dimensionless values in the order
`[count, sum, mean, minimum, maximum]`. Units are not inferred from headers.
The sum uses ordered Neumaier compensated accumulation; mean is sum / count.
There must be at least one data row. Missing, empty, invalid or non-finite numeric
values are refused, never silently dropped. Overflow is refused.

Scanning is genuinely incremental: one record is parsed at a time using a fixed
64 KiB input buffer. No catalogue-sized rows or numeric arrays are retained.
Headers and the current record are bounded independently. Limits are:

| Bound | Streaming scan |
|---|---:|
| Original file | 1 GiB |
| Encoded logical record, including record terminator | 1 MiB |
| Columns per record | 1,024 |

The original `csv_numbers`, `csv_column` and other array-returning table readers
retain their existing 16 MiB, 100,000-row and 1,000,000-cell limits. Scanning does
not turn them into unlimited array readers. Current streaming is a numeric-column
reduction API, **not** arbitrary streaming joins, a row iterator, filtering or
subset export.

CSV and TSV require UTF-8, a nonempty unique header and consistent row widths.
LF and CRLF record endings, fully quoted fields, doubled quotes and embedded
newlines inside quoted fields are supported. One leading UTF-8 BOM is accepted
and remains part of the original file hash. Bare CR record endings, NUL bytes,
ragged rows, duplicate headers and malformed quoting are refused. Inputs must
be ordinary non-symlink files.

## Evidence and historical behavior

New alpha.27 run receipts record `resource_policy` and `resources`: requested
budget (or null for the default), effective budget and used loop entries. The
verifier checks the declared main-source budget and the hard bound; native
receipts additionally cross-check the independent native counter manifest.
These are checksum and consistency checks, not authenticated execution proofs.

Streaming input descriptors use the same data-import custody mechanism as other
scientific inputs. SHA-256 is accumulated over the **exact bytes read by the
parser**, not over a different read made before parsing. Evidence is copied from
the same opened file in bounded chunks and rehashed; if it changed, the run
cannot pass. Later scans of the same path cannot silently adopt changed data or
a different delimiter. Successful input snapshots remain independently verifiable
if the original file is subsequently edited or removed.

New freezes pin the resource policy as well as exact source bytes (including the
budget declaration). Historical receipts remain verifiable without invented
resource metadata. A legacy freeze without a resource policy may retain its
unchanged source and original default 1,000,000-entry limit. Alpha.27-and-newer
freezes cannot downgrade by removing that policy. Changing a budget declaration
changes source bytes and requires an explicit revision; adopting a different
pinned policy also requires an explicit revision and a new freeze.

The **separate parser-policy migration** still applies: alpha.27 introduces
`goblin.compound-units-loop-budget.v2` because `GO_LOOP_BUDGET` previously could
name a variable, function, parameter or loop variable (and appear in implicit
multiplication). Preserved alpha.25/26 evidence using
`goblin.compound-unit-literals.v1` is verified with its original grammar. Executing
or compiling an old v1-frozen source under v2 requires an explicit revision and
new freeze, even when its resource limit itself is unchanged. Rename any old
`GO_LOOP_BUDGET` identifiers in the revised source. New alpha.27 evidence cannot
downgrade to v1 by replacing its parser-policy field.
