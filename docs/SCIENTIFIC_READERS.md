# Native scientific data readers — alpha.41

Goblin++ reads a bounded subset of NumPy NPY/NPZ, SciPy sparse NPZ and Astropy ECSV directly in Rust. Interpreter, `run --compile` and standalone compiled programs use the same reader implementation. Python is not required at runtime. These are file-format readers, not NumPy/SciPy/Astropy Python APIs. Existing FITS functions remain available.

## First run

The bundled interpreter needs neither Python nor a Rust installation. From the installed package directory:

```sh
goblin++ run examples/scientific_readers.gbl
```

For data-using `--compile`, install the retained Rust 1.92.0 toolchain and cache the locked dependencies first. The new ZIP/DEFLATE crates may not be in an older installation's cache. From the extracted source folder, run `cargo build --locked` once with network access, then:

```sh
goblin++ run examples/scientific_readers.gbl --compile
```

Generated native builds use Cargo offline. An interpreter-only installation can use every reader without this build step.

Copy the printed `RUN_DIR` into:

```sh
goblin++ verify <RUN_DIR>
```

`verify` checks preserved input bytes and replays the recorded reader operations offline. It does not certify a dataset's scientific validity, the interpretation of columns, or equivalence to every operation in a Python library.

## NumPy arrays

```goblin
shape = npy_shape("measurements.npy")
dtype = npy_dtype("measurements.npy")
order = npy_order("measurements.npy")
values = npy_read("measurements.npy")
ids = npy_text("source_ids.npy")
```

All array values are **flat logical C-order independent copies**. A 2×3 array `[[1,2,3],[4,5,6]]` becomes `[1,2,3,4,5,6]`, even when stored in Fortran order. Obtain shape explicitly; `npy_order` reports physical storage (`C`/`F`), not the returned flat layout. A scalar has shape `[]` and one value; an empty array has no values. This stage does not add shaped tensor operations.

Supported NPY versions: 1.0, 2.0, 3.0. Supported dtypes: signed/unsigned integers of 1/2/4/8 bytes, float32/float64, booleans, fixed-width UTF-8 byte strings and UTF-32 Unicode strings. Explicit little/big endian is decoded; host-dependent `=` byte order is refused. Float32 promotes its actual binary32 value to binary64. Signed zero survives. Non-finite values, invalid Unicode, structured/object/complex/datetime/float16/extended dtypes, malformed headers, truncated or trailing payloads are refused. Object arrays are never unpickled.

Integer numeric conversion is limited to −(2^53−1) through +(2^53−1). Use `npy_text` for exact decimal integer identifiers, including uint64. It also accepts byte/Unicode strings; it refuses float/bool-to-text coercion. NPY has no physical-unit metadata: numbers are dimensionless unless your program assigns meaning explicitly.

## NPZ archives

```goblin
keys = npz_keys("measurements.npz")
shape = npz_shape("measurements.npz", "positions")
values = npz_read("measurements.npz", "positions")
ids = npz_text("measurements.npz", "source_id")
dtype = npz_dtype("measurements.npz", "positions")
order = npz_order("measurements.npz", "positions")
```

Exact keys omit `.npy`; listing is sorted. `npz_keys` validates archive structure, member names and declared limits, **without decoding or CRC-checking every unselected payload**. Selecting a member checks that member's decompressed bytes, CRC and NPY structure. No archive is extracted to disk. Only stored and DEFLATE members are supported; encryption, links, directories, duplicate names, nested paths, non-NPY entries, multidisk and ZIP64 central directories are refused. NumPy's ZIP64 local headers are supported. Keys may not contain path separators, colons or control characters. A legitimate archive using these unsupported structures must be explicitly re-exported.

## SciPy sparse matrices/arrays

```goblin
shape = scipy_sparse_shape("covariance.npz")
format = scipy_sparse_format("covariance.npz")
stored = scipy_sparse_nnz("covariance.npz")
row = scipy_sparse_row("covariance.npz", 0)
values = scipy_sparse_dense("covariance.npz")
```

Reads real CSR, CSC and COO produced by `scipy.sparse.save_npz`, including the sparse-array marker. Shape, member set, pointer lengths/monotonicity/endpoints, integer indices, coordinate bounds and data lengths are checked. Duplicate coordinates are refused rather than summed; canonicalize them explicitly in the producer if that is the intended scientific policy. Explicit stored zero entries count toward `nnz`. BSR/DIA and complex sparse data are unsupported. Row indices are zero-based unitless integers. Dense results use the same flat C-order convention; densification is explicit and bounded. No sparse arithmetic API is implied.

## Astropy ECSV scalar tables

```goblin
rows = ecsv_rows("catalogue.ecsv")
columns = ecsv_columns("catalogue.ecsv")
headers = ecsv_headers("catalogue.ecsv")
ids = ecsv_column("catalogue.ecsv", "source_id")
velocity = ecsv_numbers("catalogue.ecsv", "velocity")
labels = ecsv_read("catalogue.ecsv", "label")
dtype = ecsv_dtype("catalogue.ecsv", "velocity")
original_unit = ecsv_unit("catalogue.ecsv", "velocity")
```

This is an explicit **ECSV 1.0 scalar subset**, not a general YAML or Astropy object loader. It accepts one-line scalar column flow mappings, comma or single-space delimiters, the optional `astropy-2.0` schema and empty `meta: {}`. Supported column datatypes: bool, string, int8/16/32/64, uint8/16/32/64, float32/64. Header names/order must match the data header. Quoted delimited cells are supported. Wrapped/block YAML, constructors, tags, aliases, arbitrary metadata, serialized mixins, subtype arrays and masked/empty cells are refused. Export a plain scalar table explicitly rather than losing these semantics silently.

`ecsv_column` returns parsed cell **text**, preserving exact ID digits; it is not a typed validation of every cell. `ecsv_read` returns typed values; `ecsv_numbers` requires a numeric column. Numeric reads apply the declared unit using Goblin++'s registered unit vocabulary and checked compound units (for example `km`, `m / s`, `kg / m3`, `1 / s`). Values are converted to SI with dimensions retained. Unknown units such as `Jy` are refused on numeric reads; metadata/text can still be inspected. Units on bool/string columns are unsupported. No unit is silently discarded. Missing values, overflow, non-finite values, underflow to zero and integer precision loss are refused. Astropy Time, SkyCoord, WCS and general Quantity mixins are outside this stage.

## Limits and evidence

- 16 MiB original file; 16 MiB per expanded NPZ member; 64 MiB declared total archive expansion; at most 128 archive members.
- At most 100,000 materialized values; NPY at most eight axes; header at most 64 KiB. A sparse row can be read without densifying the whole matrix, but the sparse container still obeys the input and stored-entry limits.
- ECSV at most 100,000 rows, 1,024 columns and 1,000,000 cells; no streaming ECSV in this stage.
- Original-byte cache: at most 32 files and 64 MiB, with at most 64 recorded reader calls per file. Prefer reading a column/array once and computing from the returned copy.

Original files are captured once per run. Later reader calls use those captured bytes. A path cannot be mixed with the existing FITS/CSV/batch readers in the same evaluation. Parsing failures after capture retain their original input evidence. Unreadable/oversized paths that cannot be safely captured do not acquire a claimed input snapshot.

Receipts record original SHA-256 and byte count, container format, operations/selectors, shape/dtype/units where applicable, result hashes or explicit failures, and the versioned implementation/limits policy. `GO_PARANOID` retains its postflight checks; `COMPLETELY_MAD` also retains final-state capture. Freeze receipts pin the reader policy; adopting readers under an older freeze requires an explicit revision. Historical non-reader runs retain their existing verification behavior.

Audited native execution checks input/access/output parity against the interpreter reference. Standalone native programs preserve reader inputs and a native-data manifest under their printed `NATIVE_DATA_DIR`; that manifest is not a full audited-run receipt. Failures preserve captured scientific inputs when an output directory can be created safely.

Evidence is plaintext and checksummed, not encrypted or authenticated authorship. Snapshots can contain sensitive scientific or personal data: sharing a run shares its preserved inputs. Archive safety checks are not an OS sandbox or an antivirus verdict.

## Independent format fixtures

`tests/fixtures/scientific_io/*/REFERENCE.json` records producer versions, synthetic expectations and file hashes. Fixtures were written by NumPy/SciPy and Astropy, then read by Goblin++ with no Python runtime. `tools/generate_scientific_fixtures.py` is an optional developer-only generator; it never overwrites existing fixture folders. These are interoperability and refusal tests, not validation of a scientific hypothesis.

Official format references: [NumPy NPY/NPZ](https://numpy.org/doc/stable/reference/generated/numpy.lib.format.html), [SciPy sparse NPZ](https://docs.scipy.org/doc/scipy/reference/generated/scipy.sparse.save_npz.html), [Astropy ECSV](https://docs.astropy.org/en/stable/io/ascii/ecsv.html).

MAT/HDF5, VOTable/ASDF, general YAML/mixins and Python-function bridging are future scope, not shipped capabilities.

Software/examples: MIT. Tutorial prose: CC BY 4.0. [Goblin++ merch](https://h4k3rl1f3.myspreadshop.co.uk).
