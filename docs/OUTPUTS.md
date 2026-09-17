# Audited files and plots

Goblin++ 0.1.0-alpha.6 can produce text, Markdown, CSV, TSV, JSON, SVG, and PNG artifacts. Generated files never appear beside the source program. They are created exclusively inside the unique run directory:

```text
RUN_DIR/outputs/filename
```

The run receipt records each output's relative path, media type, byte count, producer, metadata, and SHA-256. `goblin++ verify RUN_DIR` independently checks the bytes and size. A generated filename cannot be absolute, contain a directory, or traverse with `..`; declaring the same filename twice is refused rather than treated as an overwrite.

Generated-output calls do not require `GO_PARANOID` or `seal`. They receive the same path restrictions, create-new behavior, receipt hashing, and independent verification in everyday and paranoid programs. `GO_PARANOID` adds postflight source observation and a receipt self-verification gate; `seal` separately snapshots a named value as a typed artifact.

## Text

`write_text` accepts a filename followed by one or more values. Each value becomes one line. Text values use the same `{variable}` interpolation as `print`.

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

- Generated output functions run through the Rust interpreter; compiled output calls are refused.
- Each output is limited to 64 MiB in this first stage.
- Output filenames are flat and live under `RUN_DIR/outputs`.
- Bulk FITS-to-table export awaits explicit row-selection and filtering semantics.
- FITS writing is not yet implemented; a trustworthy writer must preserve schema, headers, units, and provenance explicitly.
