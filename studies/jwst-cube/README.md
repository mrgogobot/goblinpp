# JWST cube extraction study tools

Experimental, narrowly scoped native tools using the Goblin++ FITS backend. These are **not shipped `goblin++` language builtins**, a new release, or a general astronomical calibration pipeline. The interpreter/compiler, language version and plugins are unchanged.

## Build and test

From the repository root, with the retained Rust 1.92.0 compiler:

```sh
cargo test --locked --manifest-path studies/jwst-cube/extractor/Cargo.toml
cargo test --locked --manifest-path studies/jwst-cube/regional/Cargo.toml
cargo build --locked --release --manifest-path studies/jwst-cube/extractor/Cargo.toml
cargo build --locked --release --manifest-path studies/jwst-cube/regional/Cargo.toml
```

The executables are in each study crate's `target/release/` unless an explicit `--target-dir` was used. Software follows the repository's MIT license; this does not license astronomical observations or third-party dependencies.

## Extract, verify, export one spatial pixel

Supply a separately retained expected input checksum and a **new** output directory:

```sh
studies/jwst-cube/extractor/target/release/goblin-cube-extract extract \
  input.fits EXPECTED_INPUT_SHA256 new-extraction
studies/jwst-cube/extractor/target/release/goblin-cube-extract verify \
  new-extraction EXPECTED_EXTRACTION_RECEIPT_SHA256
studies/jwst-cube/extractor/target/release/goblin-cube-extract spectrum \
  new-extraction EXPECTED_EXTRACTION_RECEIPT_SHA256 26 25 new-spaxel.csv
```

Use the actual receipt digest printed by extraction; retain it independently. Existing outputs are refused; repeating verification validates existing evidence rather than replacing it. The study receipt format is distinct from standard Goblin++ run receipts. A normal `.gbl` program that reads the resulting CSV still uses `goblin++ verify ACTUAL_RUN_DIR`.

Supported layout: unique SCI/ERR/DQ/WMAP image extensions, matching three-dimensional shapes, linear uncoupled WAVE/um WCS, SCI/ERR in MJy/sr, float32 SCI/ERR/WMAP and supported pseudo-unsigned 32-bit DQ. Unsupported scaling, units, WCS, layouts and resource sizes are refused. No exposure coaddition, smoothing, background subtraction or aperture summation occurs at extraction.

All spatial pixels and channels are retained, including missing/flagged ones. Float words, NaN payloads and signed zero are preserved in `spectra.bin`; CSV nonfinite values are empty cells, never fabricated zeros. Physical DQ values are preserved, not raw signed-storage words interpreted as unsigned flags.

FITS source index is `x + nx*(y + ny*channel)`. Output record offset is `((y*nx+x)*nz+channel)*20` bytes. Each record contains five little-endian u32 words: SCI f32 bits, ERR f32 bits, physical unsigned DQ, WMAP f32 bits and annotation status. Coordinates are zero-based. This is a documented study storage layout, not an authenticated file format.

### Convenience mask — processing policy, not scientific reliability

Science usability: finite SCI, physical DQ exactly zero, finite WMAP and WMAP > 0. Uncertainty usability additionally requires finite ERR > 0. Status bits: 1 nonfinite SCI; 2 nonfinite ERR; 4 negative finite ERR; 8 nonzero DQ; 16 nonfinite WMAP; 32 nonpositive finite WMAP; 64 zero ERR.

WMAP is not treated as inverse variance. Supplied ERR values are not a covariance model. An accepted voxel is not automatically a reliable scientific measurement.

## Matched regional reduction

The regional tool reads the frozen native binary, **not Python extraction results**:

```sh
studies/jwst-cube/regional/target/release/goblin-cube-regional-audit \
  new-extraction protocol.json EXPECTED_PROTOCOL_SHA256 new-regional-output
```

`protocol.json` has schema `goblin.study.matched-apertures.v1`, an independently retained `input_receipt_sha256`, `mask_policy: documented_convenience`, and an `apertures` array of objects with unique names, zero-based `x0`, `y0` and `radius_pixels`. It also supplies `background_ring_pixels`, `cdelt1_deg`, `cdelt2_deg`, `minimum_aperture_fraction: 0.8`, and `minimum_background_rule: n_sky >= min(20,0.5*n_geometric_sky)`.

For the audited protocol, aperture membership is pixel-centre radius `< r`; background membership is `9 <= radius < 14`. The per-channel median background is subtracted from each accepted aperture pixel. Conversion is `abs(CDELT1 radians)*abs(CDELT2 radians)*1e15`, matching the supplied Python procedure, not establishing exact per-pixel solid angle or aperture-loss correction. Fluxes are in nJy; negative flux remains negative. Failing coverage thresholds yield missing flux/background, while valid aperture/sky counts remain recorded. Reductions use compensated summation and preserve shortest round-trip output.

Protocol/input/output hashes are pinned. This is a bounded research reducer, not a hostile-filesystem sandbox, general WCS aperturer, covariance estimator, automatic source detector or uncertainty/significance calculator.

## Completed numerical verification: 14 October 2026

Frozen combined cube SHA-256: `18b78fe42f03e2ed886b94adc582ba7bb2857ad78c4edb6e55a29fc8d6c995b1`.

- 53 × 51 × 941 = 2,543,523 voxels; all SCI/ERR/DQ/WMAP words agree with an independent Astropy decode.
- Native/documented mask agreement: zero mismatches over the complete real cube and 1,050 synthetic cases. The supplied legacy Python mask differs from the convenience policy in 68 synthetic cases; equality on the real cube is **not general equivalence**.
- Coverage is 1,020–1,459 usable pixels on nonempty channels; channel 940 at 5.302499918732792 um has zero usable pixels. All 941 channels remain. The sum is 1,336,349 usable voxel samples; the union of spatial positions is 1,476, **not** the count at every wavelength.
- Seven apertures/control regions × 941 channels were compared under fixed identical pixel geometry, background and mask. Counts, background medians and missingness agree exactly. Maximum full-precision flux difference is 9.094947017729282e-13 nJy, inside the predeclared `1e-9 + 2e-12*abs(Python)` bound. Historical seven-decimal CSV differences agree with its declared rounding tolerance, not a rounded authoritative seal.
- The original Python R2 DQ reader incorrectly interpreted stored words as `>u4` without FITS signed-storage/BZERO decoding. See [V2-A erratum](V2A_ERRATUM.md). R2 stayed untouched; a new cube-only R3 audit supersedes that interpretation.

The large cube, extracted arrays, supplied Python source and private run artifacts stay outside this repository. Local study packages retain their detailed hash-pinned evidence. Code/test publication is not publication of those observations or a claim of independent instrument calibration, source localization, redshift confirmation, astrophysical reliability or authenticated authorship.

References: [Astropy image scaling](https://docs.astropy.org/en/stable/io/fits/usage/image.html), [JWST resampled IFU data](https://jwst-pipeline.readthedocs.io/en/stable/jwst/data_products/science_products.html#resampled-3-d-ifu-data-s3d).
