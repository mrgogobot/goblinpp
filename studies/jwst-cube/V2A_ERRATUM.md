# V2-A: R2 cube DQ interpretation invalidated; R3 correction

Date: 14 October 2026. **The apparent ubiquitous cube `REFERENCE_PIXEL` anomaly was introduced by the minimal Python reader, not observed in the telescope data.**

The header declares `BITPIX=32`, `BSCALE=1`, `BZERO=2147483648`. Correct reconstruction is signed big-endian int32 storage plus 2147483648, evaluated in int64 and then represented as uint32. Reading the same stored bytes directly as unsigned uint32 introduces a spurious high bit for this cube's low-valued physical flags. It is not legitimate to fix this by blindly clearing bit 31: a genuine physical bit-31 flag must survive correct decoding.

| Invalid R2 raw-word interpretation | Correct physical DQ | Voxel count |
|---|---|---:|
| `0x80000000` | `0` | 1,336,349 |
| `0x80000001` | `1` | 22,795 |
| `0x80000201` | `513` | 1,184,379 |

Physical bit-31 count: **0**. Corrected fraction over the R2 finite/support selection: **0.0**, not R2's 1.0. All 2,543,523 corrected DQ values agree with Astropy. Known-answer checks cover unsigned extrema and genuine high-bit preservation, plus refusal of unsupported scaling/types.

## What is superseded

R2's cube `DQ_unsigned_counts`, `DQ_science_reference_flag_fraction`, the warnings claiming ubiquitous physical cube `REFERENCE_PIXEL`, and the reason in `scientific_status` asserting that this cube flag is unexplained. The observed raw bytes remain valid observations of **storage**, not physical DQ flags. R2 files and their hashes are preserved without replacement.

This erratum does not promote any line fit or scientific measurement to validated status. The R2 low-bit selection happens to agree with corrected selection and the documented convenience mask **for this cube**. The masks are not equivalent generally. Numerical regional extractions agree under identical declared settings; masking and agreement do not establish calibration, localization, covariance, backgrounds, PSF/aperture losses, associations or line identifications.

The new correction is **R3_DQ_ONLY**, not a full Stage-1 rerun. The `x1d` table-scaling interpretation and its reported DQ counts are **outside this cube-only audit**; they must not be treated as corrected by this result. No new line fitting or physical inference was performed.

Original R2 source SHA-256: `4a69fbd93b8a3947c9820361cc05ac2173e1150110a53ec04aef5ead8b60b84b`.

Original R2 audit SHA-256: `01438dd296edc9ace3dd8ffc6153dc7afcc34e850a05d95b903eeb6cef88adf9`.

Original R2 spectra SHA-256: `a6e897a392be44dbefdee77d02f39be39c1a3e1cdfbb84c07e0a38c26a320204`.

Corrected cube SHA-256: `18b78fe42f03e2ed886b94adc582ba7bb2857ad78c4edb6e55a29fc8d6c995b1` (unchanged).

Primary format reference: [Astropy FITS image scaling](https://docs.astropy.org/en/stable/io/fits/usage/image.html). Checksums are evidence of consistency with retained anchors, not signatures or scientific truth certificates.
