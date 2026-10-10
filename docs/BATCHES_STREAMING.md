# Serial table batches — alpha.28

CSV/TSV batches let a program process a large table in order without loading
the entire table into arrays. They work in the interpreter, audited native
execution and standalone compiled programs. This stage is serial: no worker
threads, implicit parallel reductions, FITS batch reader or automatic joins.

Run the complete example:

```console
goblin++ examples/batch_catalogue.gbl
goblin++ examples/batch_catalogue.gbl --compile
```

Both produce four rows, total measurement 11, and an incremental `processed.csv`
inside the new run's `outputs/` directory. The exact, very large IDs stay text.
Never pass catalogue IDs through `parse_number`: f64 cannot represent every ID.

## Verify a preserved run

After execution, copy the printed `RUN_DIR` into `goblin++ verify`:

```console
goblin++ verify "<RUN_DIR>"
```

Replace `<RUN_DIR>` with the actual directory; each run gets a new name.
For example, this command verified the tested alpha.28 terminal run while the
terminal was in the bundle's `examples/` directory:

```console
goblin++ verify ./runs/20261010T212059.403367Z_cf5e6c73fe40
```

Expected final result:

```text
VERIFICATION_STATUS=PASS
THE GOBLIN IS SATISFIED.
```

Relative paths depend on your terminal's current directory. From the bundle
root, that same run is under `examples/runs/`, not `./runs/`. IDE launchers
may print an absolute `RUN_DIR`; use it directly, enclosed in quotes if it
contains spaces.

Verification checks the receipt, preserved source and input evidence, sealed
values, generated output hashes/sizes, batch policy/lifecycle and ledger
evidence without rerunning the calculation. A verification PASS means the
recorded evidence passes integrity checks: it does not prove the scientific
model is correct or authenticate authorship. A preserved failed run can also
verify successfully; check its `RUN_STATUS` separately.

## API

| Function | Arguments and result |
| --- | --- |
| `csv_batch_open(path, columns, rows, batch_bytes, file_bytes)` | Open CSV; return a run-local reader handle. Columns are distinct exact header names in requested order. All three limits are explicit positive integers. |
| `tsv_batch_open(path, columns, rows, batch_bytes, file_bytes)` | Same for TSV. |
| `batch_next(reader)` | Return a flat row-major array of **text** cells. Empty array means observed EOF, never a skipped malformed row. |
| `batch_close(reader)` | Close after observed EOF; early/double close refuses. |
| `csv_stream_open(name, headers, file_bytes)` | Create a CSV writer, write its header, return a run-local handle. |
| `tsv_stream_open(name, headers, file_bytes)` | Same for TSV. |
| `stream_write(writer, cells)` | Write a flat array containing complete rows; return the number written. May also be a statement. Quantities retain rendered SI units. |
| `stream_close(writer)` | Sync and close. Further writes or double close refuse. |

The open/next calls produce values and must be assigned or otherwise consumed.
Reader/writer handles are text tokens, not secrets or security capabilities.
Arrays retain their existing homogeneous-type rules: for a row combining a
text ID and numeric measurement, use `[source_id, to_text(measurement)]`.
They cannot refer to another process's state. Both CSV and TSV use lossless
double-quote escaping, including embedded delimiters, quotes and newlines.
UTF-8 BOM and CRLF input are accepted; all original bytes enter the input hash.
Empty cells remain empty text: there is no automatic missing-value policy.
A header-only table is valid; a file without a header is not.

## Load, process, release, next

```goblin
reader = csv_batch_open("data.csv", ["id", "value"], 4096, 8388608, 2147483648)
writer = csv_stream_open("copy.csv", ["id", "value"], 2147483648)
while true {
    batch = batch_next(reader)
    if len(batch) == 0 { break }
    stream_write(writer, batch)
    batch = []
}
batch_close(reader)
stream_close(writer)
```

Input paths are relative to the saved entry program. Output names are simple
filenames, with matching `.csv`/`.tsv` extension, not arbitrary destination paths.
Existing output names cannot be reused, including names used by `write_csv`.
Do not mix eager table/FITS/scan access and batch access to the same input in
one run, or open the same batch input twice. Open a separate run to reread it.

The reader owns a 64 KiB input buffer and at most one deferred projected row;
it does not retain returned batches. You must release your batch before the
next read, and avoid accumulating arrays, printed rows, or sealed batches.
Ordinary value-copy semantics still apply. **This is not a process-wide RAM
quota**: user arrays, copies, output logs and other subsystems can use memory.
Declared `batch_bytes` includes the returned array slots and String capacities,
not just characters. A single selected row that cannot fit refuses; otherwise
the reader may return fewer than `rows` to honor the byte budget.

## Explicit limits

- Each input and output: a caller-selected limit up to **64 GiB**. This is a
  batch-specific contract; old eager/scan limits are unchanged.
- Returned batch: up to **8 MiB** decoded allocation and **100,000 cells**.
  Requested rows must be at most `100000 / number_of_requested_columns`.
- Encoded input/output logical record: up to **1 MiB**; at most **1024 columns**.
- Each `stream_write` call: up to **100,000 cells**, an **8 MiB** decoded budget,
  and complete rows. Nested arrays refuse. Limits do not truncate data.
- Up to **four active readers**, **four open writers** and **128 total handles**.
- Existing `GO_LOOP_BUDGET` is shared across loops, including batch processing:
  default 1,000,000, maximum explicit 50,000,000. It is not a wall-clock timeout.

Choose limits appropriate to the experiment; 64 GiB is a ceiling, not a promise
that the disk has room. No generic automatic chunking of FITS files is added.

## Evidence and failure

Opening an input streams a full private **disk snapshot** and SHA-256 first,
then rechecks the original hash. Parsing uses that immutable snapshot. This costs
an initial full-file scan and temporary disk space, not a whole-file RAM load.
Full input evidence and generated output are copied into the audited run;
content storage and native/reference runs require additional disk space and I/O.
Large files can therefore require several times their size in free disk space.

Receipts pin `goblin.serial-table-batches.v1`, declared limits, requested columns,
row/batch counts, observed peak batch allocation, lifecycle state, full input
hashes and output hashes/sizes. Verification checks their structure and bindings;
it does not replay the scientific calculation or prove physical blinding.
New freezes pin this policy. Existing alpha.27 freezes without batch calls still
work; adopting batch functions under an older freeze requires a revision.

A PASS requires every input read through EOF and closed, and every writer closed.
If processing fails, staged stream files are preserved as **`name.csv.partial`**
(or `.tsv.partial`), with `metadata.complete=false`. Previously written complete
records remain available; a failed OS write may also leave a trailing fragment.
The summary distinguishes successfully committed bytes/rows from actual file
size. Never treat partial outputs as a complete catalogue. Verification of a
failed run means its evidence is intact, not that the calculation succeeded.

Standalone programs preserve `native-data.json`, stream outputs and batch input
snapshots in the reported `NATIVE_DATA_DIR`; failures report `INCOMPLETE_DATA_DIR`
with partial outputs and input snapshots. They do **not** acquire the launcher's
custody ledger, freeze checks or paranoid postflight simply by being compiled.
Abrupt termination, power loss, disk exhaustion or filesystem attacks can defeat
normal failure preservation; no sandbox or crash-recovery guarantee is implied.

Batch boundaries must not silently change the scientific method: averaging
batch means is generally wrong for unequal batch sizes; medians/quantiles need
global ordering or a separately specified out-of-core algorithm. This API does
not turn per-batch statistics into global statistics.

Privacy: snapshots and outputs are plaintext. Integrity is not encryption.
Scientific PCG32 is not cryptographic. Encryption and `COMPLETELY_MAD` remain
roadmap proposals. Rust stays at 1.92.0, with no dependency additions.

The opt-in real-file memory regression can be repeated on macOS/Linux:

```console
python3 tools/check_batch_memory.py target/release/goblinpp --large --report batch-memory.json
```

It creates disposable synthetic tables, including a file above 1 GiB, checks
exact copied bytes and run verification, and measures the engine's peak RSS.
It needs several GiB of free disk and is separate from routine unit tests.

Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).
