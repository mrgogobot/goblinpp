# FITS discovery and import

Goblin++ treats FITS structure and scientific interpretation as separate questions. Discover what a file contains before choosing values to analyse.

## Inspect first

For fast structural reconnaissance of a large file:

```console
goblin++ fits-info specObj-dr16.fits --quick
```

This reads every HDU header but deliberately does not hash the full file. Its output is labelled `QUICK_INSPECTION_UNHASHED_NOT_EVIDENCE`.

For a complete file SHA-256 as part of the inspection report:

```console
goblin++ fits-info specObj-dr16.fits
goblin++ fits-info specObj-dr16.fits --json
```

Neither inspection mode creates a run receipt. A normal `.gbl` execution does.

## Index conventions

- HDU indexes are zero-based: the primary HDU is `0`, the first extension is `1`.
- Table row and repeated-column element indexes are zero-based.
- Image axes follow FITS notation and are one-based: `NAXIS1` is axis `1`.

## Functions

| Function | Result |
|---|---|
| `fits_hdu_count(file)` | Number of HDUs |
| `fits_header(file, key)` | Primary-header value (compatible form) |
| `fits_header(file, hdu, key)` | Value from an explicit HDU header |
| `fits_axis(file, axis)` | Primary-image axis length (compatible form) |
| `fits_axis(file, hdu, axis)` | Explicit image-HDU axis length |
| `fits_count(file[, hdu])` | Image element count |
| `fits_pixel(file[, hdu], index)` | Flattened image value |
| `fits_mean(file[, hdu])` | Mean of finite, non-`BLANK` image values |
| `fits_rows(file, hdu)` | Binary-table row count |
| `fits_columns(file, hdu)` | Binary-table column count |
| `fits_column(file, hdu, name, row)` | Scalar numeric, logical, or text cell |
| `fits_column(file, hdu, name, row, element)` | Element of a repeated numeric/logical column |
| `fits_column_valid_count(file, hdu, name)` | Non-null numeric elements |
| `fits_column_mean(file, hdu, name)` | Compensated mean of non-null numeric elements |
| `fits_column_min(file, hdu, name)` | Minimum non-null numeric element |
| `fits_column_max(file, hdu, name)` | Maximum non-null numeric element |

A direct read of a null cell returns `G601`; aggregate statistics skip integer `TNULL` values and floating-point NaNs. Column matching is ASCII case-insensitive. A repeated numeric column requires the fifth `element` argument for a direct cell read, while statistics include every element.

## Catalogue example

```goblin
GO_PARANOID

rows = fits_rows("specObj-dr16.fits", 1)
first_class = fits_column("specObj-dr16.fits", 1, "CLASS", 0)
first_z = fits_column("specObj-dr16.fits", 1, "Z", 0)
valid_z = fits_column_valid_count("specObj-dr16.fits", 1, "Z")
mean_z = fits_column_mean("specObj-dr16.fits", 1, "Z")

print("rows = {rows}")
print("first class = {first_class}")
print("first redshift = {first_z}")
print("valid redshifts = {valid_z}")
print("unfiltered mean redshift = {mean_z}")

seal rows
seal valid_z
seal mean_z
```

The mean above is deliberately labelled unfiltered. Goblin++ does not infer sample selection, quality cuts, object class, warning masks, or cosmological meaning from a column name. Those scientific choices must become explicit language features or explicit source code before the result is interpretable.

## Evidence and large files

The first evidence-grade run over a large FITS file reads the file to hash it and reads it again to build a verified checksum-addressed evidence object. Ensure the project filesystem has enough free space for one preserved copy. Later runs hard-link that object into their run directories when supported, so they do not each consume another full copy.

`--quick` is appropriate for discovering structure, never for making an evidence claim.

## Explicit limits

This release does not read ASCII-table values, random groups, tile-compressed images, bit arrays, complex values, or variable-length array heaps. It reports those structures where possible and refuses unsupported access. WCS, declared FITS units, uncertainty models, filters, and catalogue-specific semantics are not interpreted.
