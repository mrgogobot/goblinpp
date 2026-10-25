# Audited files and plots

Alpha.39's opt-in [COMPLETELY_MAD](COMPLETELY_MAD.md) profile also captures
bounded final global values in `RUN_DIR/final-state.json`, separately from
explicit `seal` snapshots and generated outputs. Failed runs label captures
partial; sealing does not encrypt their contents.

Alpha.31 embeds manifest links in general array-plot PNG/SVG; see
[PLOT_PROVENANCE.md](PLOT_PROVENANCE.md). Legacy FITS plots stay unchanged.
Alpha.30 adds general array plotting in both engines. Start with
[ARRAY_PLOTS.md](ARRAY_PLOTS.md): `plot_line`, `plot_scatter`, `plot_histogram`
preserve exact data/settings and support reconstruction verification. They do
not sample rows. The FITS APIs below retain historical sampling behavior.

Goblin++ through alpha.30 can produce text, Markdown, CSV, TSV, JSON, SVG, and PNG artifacts in interpreted and audited compiled runs. Generated files never appear beside the source program. They are created exclusively inside the unique run directory:

```text
RUN_DIR/outputs/filename
```

The run receipt records each output's relative path, media type, byte count, producer, metadata, and SHA-256. `goblin++ verify RUN_DIR` independently checks the bytes and size. A generated filename cannot be absolute, contain a directory, or traverse with `..`; declaring the same filename twice is refused rather than treated as an overwrite.

Generated-output calls do not require `GO_PARANOID` or `seal`. They receive the same path restrictions, create-new behavior, receipt hashing, and independent verification in everyday and paranoid programs. `GO_PARANOID` adds postflight source observation and a receipt self-verification gate; `seal` separately snapshots a named value as a typed artifact.

## Text

### Quantity labels already include units

Interpolating a dimensioned quantity prints its SI unit automatically, including
when an explicit numeric precision is used:

```goblin
length = 5.382196358289262e-35 m
print("length = {length}")
print("rounded length = {length:.2e}")
```

These produce `length = 5.382196358289262e-35 m` and
`rounded length = 5.38e-35 m`. Do not append another literal `m` after the
placeholder: `"{length} m"` deliberately prints `m m`. Literal text is never
silently stripped or deduplicated. If a custom unit label is needed, first divide
by the explicit reference quantity to obtain a dimensionless number:

```goblin
metres = length / (1 m)
print("length = {metres} m")
```

This applies to stored strings and `write_text` too. Correct labels in a fresh
source revision; do not edit or reseal an existing run's stdout or receipt.

`write_text` accepts a filename followed by one or more values. Each value becomes one line. Since alpha.20, source string templates capture values immediately; stored text is never interpolated again at write time.

```goblin
samples = 12
accepted = 9
fraction = accepted / samples

write_text("summary.txt", "Acceptance report", "samples = {samples}", "accepted = {accepted}", "fraction = {fraction:.3f}")
write_text("notes.md", "# Acceptance report", "The accepted fraction is {fraction:.3f}.")

```

Supported extensions are `.txt` and `.md`.

## CSV and TSV

`write_csv(filename, columns, cells...)` groups all cells after the column count into rows. The first row is the header. The number of cells must divide evenly by the declared column count. Values should be passed directly; they do not need to be turned into strings first.

```goblin
write_csv("summary.csv", 2, "metric", "value", "samples", samples, "accepted", accepted, "fraction", fraction)
write_tsv("summary.tsv", 2, "metric", "value", "samples", samples, "fraction", fraction)
```

CSV cells containing commas, quotes, or newlines are quoted and embedded quotes are doubled. TSV control separators inside text cells are replaced with spaces. This first output stage is intended for result tables and modest derived data, not bulk catalogue export.

## JSON

`write_json(filename, key, value, ...)` creates one JSON object. Keys must be unique strings.

```goblin
write_json("summary.json", "samples", samples, "accepted", accepted, "fraction", fraction)
```

Dimensionless quantities become JSON numbers. Dimensioned quantities retain their SI value, dimension vector, and SI unit name instead of silently losing unit information.

## FITS histogram

```goblin
plot_fits_histogram("redshift.png", file, 1, "Z", 40, 50000, "Sampled redshift distribution")
plot_fits_histogram("redshift.svg", file, 1, "Z", 40, 50000, "Sampled redshift distribution")
```

Arguments are:

1. output filename (`.png` or `.svg`);
2. FITS filename;
3. zero-based binary-table HDU;
4. scalar numeric column;
5. number of bins, from 2 through 500;
6. maximum sampled rows, from 1 through 200,000;
7. plot title.

## FITS scatter plot

```goblin
plot_fits_scatter("z_error.png", file, 1, "Z", "Z_ERR", 50000, "Redshift and reported error")
```

Arguments are the output filename, FITS filename, HDU, x column, y column, maximum sampled rows, and title. Both columns must be fixed-width scalar real numeric columns.

## Sampling is explicit

Plots select up to the requested number of rows at deterministic, evenly spaced row indexes across the table. Null and NaN values are skipped. The receipt records:

- the sampling method;
- total population rows;
- requested points;
- examined rows;
- valid plotted points;
- sampled minima and maxima;
- the input FITS SHA-256.

The same source, FITS bytes, and arguments therefore produce the same plot bytes and hash. These plots are descriptive sampled views. They are not silently presented as full-population statistics and do not apply object-class, quality, warning-mask, or survey-selection filters.

## Why SVG and PNG, but not JPEG?

SVG is ideal for scalable plots and PNG is lossless. JPEG changes pixels through lossy compression and can introduce visual artifacts, so alpha.3 does not offer it as an authoritative scientific output. A later presentation-export feature may add JPEG while keeping a lossless original as the sealed authority.

## Verification

After a run:

```console
goblin++ verify RUN_DIR
```

Successful output checks look like:

```text
GENERATED_ARTIFACT_PATH:summary.csv .... PASS
GENERATED_ARTIFACT:summary.csv ......... PASS
GENERATED_ARTIFACT_BYTE_COUNT:summary.csv PASS
```

Editing, replacing, truncating, or deleting an output causes verification to fail.

## Current limits

- Both engines share the Rust output helpers. Audited compiled runs compare native bytes against reference output; standalone binary output has a separate boundary described in [the native I/O guide](DATA_MODULES_NATIVE.md).
- Each output is limited to 64 MiB in this first stage.
- Output filenames are flat and live under `RUN_DIR/outputs`.
- Bulk FITS subsets are available via explicit cuts and `fits_export_csv`/`fits_export_tsv`; see [FITS_SUBSETS.md](FITS_SUBSETS.md).
- FITS writing is not yet implemented; a trustworthy writer must preserve schema, headers, units, and provenance explicitly.
