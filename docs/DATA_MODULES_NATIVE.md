# Tables, reusable functions and native scientific I/O

Alpha.19 adds strict CSV/TSV input, local function libraries, and compiled
support for the existing FITS readers and file/plot writers. These are small,
explicit tools: no guessed units, automatic missing-value policy, package
manager, survey cuts, or new FITS formats are implied.

## First experiment

Run the supplied example from the project directory:

```console
goblin++ examples/csv_modules.gbl
goblin++ examples/csv_modules.gbl --compile
goblin++ verify RUN_DIR
```

Replace `RUN_DIR` with the printed directory from each run. Both modes report
three observations, a mean length of `11 m`, and a TSV mean of `3`. Files are
under that run's `outputs` directory. Each run preserves the table bytes,
library bytes, logs, sealed values and their hashes.

The input `measurements.csv` is deliberately simple:

```csv
sample,length_m
A,10
B,11
C,12
```

The program reads its numeric column explicitly:

```goblin
import "lib/measurements.gbl"
numbers = csv_numbers("measurements.csv", "length_m")
average = average_length(numbers)
print("average length = {average}")
seal average
```

The library attaches metres because the experiment defines that unit. A header
such as `length_m` is descriptive text, not an instruction to infer dimensions:

```goblin
g_func average_length(numbers) {
    lengths = []
    for number in numbers {
        lengths = append(lengths, number * 1 m)
    }
    return mean(lengths)
}
```

## CSV and TSV API

| CSV call | Result |
|---|---|
| `csv_rows(file)` | Data row count, excluding the header |
| `csv_columns(file)` | Header/column count |
| `csv_headers(file)` | Array of exact, case-sensitive header strings |
| `csv_column(file, "name")` | Array of strings, including empty cells |
| `csv_numbers(file, "name")` | Checked dimensionless numeric array |

TSV has the identical API with `tsv_` prefixes. Use it for tab-separated files;
the delimiter is never guessed. Paths are relative to the **entry program's
directory**, including calls made by an imported function.

Inputs must be UTF-8 (an initial UTF-8 BOM is accepted), with LF or CRLF records
and a nonempty header of unique, nonempty names. Quoted fields support doubled
quotes and embedded separators/newlines. Ragged rows, malformed quotes and
unknown columns fail. A header-only table is valid and returns empty arrays;
`mean` and `sum` still refuse empty observations.

Text columns preserve empty cells. Numeric columns reject empty cells,
malformed numbers, NaN and infinity: **no row is silently dropped**. They use
the same checked conversion as `parse_number`. Provide an explicit cleaning
policy yourself before analysis when data has missing values. Limits are
16 MiB per input, 100,000 data rows, 1,024 columns and 1,000,000 total cells.
This is everyday experimental-table input, not an enormous-catalogue loader.

The first load snapshots bytes; subsequent calls in a run use that snapshot.
The receipt records the input hash, size, delimiter format and requested
operations/columns. Independent verification reads preserved evidence, not a
possibly changed live input. Symlink inputs are refused.

## Local modules

`import "lib/measurements.gbl"` is a top-level, static declaration. A library
may contain `g_func` definitions and further imports only. Top-level execution,
variables, directives, seals and inline Rust are refused. Functions have the
existing local copy-value scopes and 16-call-depth ceiling. All functions share
one namespace; duplicate names fail rather than shadowing another library.

Imports are resolved relative to the importing file, stay beneath the entry
directory, and must use `.gbl` files. Absolute paths, `..`, `.` components,
backslashes, symlink paths and cycles fail. Repeated imports are deduplicated.
Limits are 32 distinct modules and 1 MiB per module. There are no downloaded
packages, dynamic imports, namespaced imports or module globals in this stage.

`check`, interpreted runs, compiled runs and `compile` resolve the same graph.
The canonical program includes imported function definitions; raw library
bytes are preserved separately under `RUN_DIR/modules`. Verification rebuilds
the graph from those snapshots without needing the original library files.

`freeze` pins each imported path and raw hash in addition to the root source
and registries. Changing even a library comment after freezing refuses the
run as `MODULE_CHANGED_AFTER_FREEZE`. `GO_PARANOID` also checks libraries at
postflight. A revision copies the entry source, **not the libraries**: keep a
versioned library copy and change the new entry's import when making a deliberate
library revision. Do not edit a library shared by frozen programs.

## Compiled FITS and files

All currently implemented FITS calls, `write_text`, `write_csv`, `write_tsv`,
`write_json`, and FITS histogram/scatter plots now compile. Existing scientific
semantics, path restrictions, dimensions, null handling and plot sampling
remain unchanged. This does not add FITS writing, compression, WCS transforms,
implicit `TUNIT` conversion, bulk export or JPEG.

```console
goblin++ examples/output_demo.gbl --compile
goblin++ examples/fits_selection.gbl --compile
```

Native programs contain direct generated Rust operations and call the shared
Rust data/output helpers. They do not launch Goblin++, Python, or an interpreter
process, and do not parse a `.gbl` file at runtime. Shared helper sources and the
locked dependency files are preserved in a hashed compiler-support inventory.

Compilation of a data-using program requires Rust/Cargo and the locked crates
already cached. The generated support build uses `--locked --offline`: it never
silently fetches dependencies. In a source checkout, `cargo build --locked`
can populate the cache first (that command may need internet access). Programs
without data/output calls retain the simpler `rustc` compilation path.

For an audited compiled run, the launcher first performs reference evaluation,
then the native executable runs independently. Input hashes/accesses, output
descriptors, sealed values and output bytes must agree before PASS. This costs
two evaluations; it is a parity gate, not a performance claim. Native output
bytes and the manifest are also independently checked by `verify`. Unrestricted
inline Rust remains a separately authorized trust boundary, not a sandbox.

## Standalone executable boundary

```console
goblin++ compile examples/csv_modules.gbl -o measurements
```

Run the executable with the data files available **relative to its working
directory**, rather than relative to the old source location. It no longer
needs the `.gbl` libraries or an installed Goblin++ engine. Data/output builds
create a fresh `goblin-native-outputs-PID-TIMESTAMP` directory, print its location
on stderr, and write `native-data.json` plus a `native-outputs` subdirectory.
No existing output file is overwritten.

A standalone executable does **not** provide the launcher's full custody
receipt, freeze enforcement or `GO_PARANOID` postflight verification. Use
`goblin++ file.gbl --compile` when those guarantees are needed. The standalone
manifest is useful metadata, not an authenticated custody ledger.

All stored evidence is plaintext. Hashing detects changes relative to recorded
checksums; it does not encrypt data or authenticate an author's identity.
