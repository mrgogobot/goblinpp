# Array plots: data first, pictures second

Alpha.31 retains these APIs and adds embedded manifest links to PNG/SVG.
See [Embedded provenance](PLOT_PROVENANCE.md) for the one-way hash chain,
historical verification and old-freeze migration.

Alpha.30 adds three functions in the interpreter and native compiler. No Python,
plotting package or network service is required. Existing FITS plotting functions
retain their historical behavior; these new functions do **not** sample data.

```goblin
GO_PARANOID
elapsed = [0 s, 1 s, 2 s, 3 s]
distance = [0 m, 2 m, 5 m, 9 m]
plot_line("motion.png", elapsed, distance, "Motion", "time", "distance")
plot_scatter("positions.svg", elapsed, distance, "Positions", "time", "distance")
plot_histogram("distances.svg", distance, 3, "Distance distribution", "distance")
seal distance
```

## Run and verify

From the extracted package root:

```sh
goblin++ examples/array_plots.gbl
goblin++ examples/array_plots.gbl --compile
```

Each command prints its own `RUN_DIR`. Figures and their `.plot.json` evidence
files live in `RUN_DIR/outputs`. Keep the complete run directory, not just the
picture, to preserve the source, receipt and upstream input evidence.

```sh
goblin++ verify <RUN_DIR>
```

A run made from `examples` can use `goblin++ verify ./runs/<timestamp>_<hash>`.
Use the actual path printed by your run, not an old example's run directory.
`PLOT_RECONSTRUCTION:<filename>` must pass as well as ordinary output hashes.

## Function signatures

| Function | Arguments, in order |
|---|---|
| `plot_line` | filename, x array, y array, title, x label, y label |
| `plot_scatter` | filename, x array, y array, title, x label, y label |
| `plot_histogram` | filename, values array, bin count, title, x label |

Each returns the figure filename as text. It may also be used as a statement:
writing the figure/evidence pair is its explicit effect. Filenames must be flat
`.png` or `.svg` names, not directories. The sidecar is `<figure>.plot.json`.
Neither name may already be used by an output or stream. Both names are checked
before either artifact is added. The functions work without `GO_PARANOID`/`seal`;
plot evidence is always generated. `GO_PARANOID` adds postflight verification;
`seal` separately captures chosen values. No plotting directive is needed.

## Scientific rules

- All values must be finite numbers. Missing values, text, Boolean values,
  nested arrays and mixed dimensions within one axis are refused.
- Arrays contain 1..100,000 values; lines need at least two points. x/y lengths
  must match. x and y may have different dimensions.
- Compatible units normalize to SI. Axis labels append the **actual SI unit**,
  including `[1]` for dimensionless values. Labels are not conversion requests:
  do not label SI metre coordinates "kilometres". Explicitly rescale your data
  when another coordinate scale is needed.
- Line segments follow supplied order. No sorting, thinning, hidden cuts,
  smoothing, fitting or inference. Coincident points can overlap in a finite
  image; every exact value remains in the sidecar.
- Axes are linear and cover all values. Line/scatter axes need not include
  zero; bounds and ticks are preserved. Constant axes expand by 5% of
  `max(abs(value), 1)`; an overflowing endpoint is not used. Unrepresentable
  spans are refused, not clipped. This renderer supports dimension exponents
  -64..64 and refuses larger ones.
- Histogram bins are an explicit dimensionless integer in 2..500. Uniform
  edges cover the full range, expanded for constant data. Bins are left-closed/
  right-open, with the final right endpoint included. Every value contributes
  once. Counts start at zero; no density normalization is implied. Collapsed
  floating-point edges are refused.

Ticks use scientific notation with 3 decimal places, increasing to 16 if distinct
tick values would otherwise share a label. This is display only. Exact
data, bounds and tick values are 16-digit lowercase IEEE-754 bit strings.
Authoritative values and seals are **not rounded** to make pictures prettier.

## Evidence and security boundary

Sidecars preserve the complete typed arrays in order, title, labels, dimensions,
filename, bin request, derived bounds/ticks/edges/counts, layout/settings,
renderer policy and finished figure SHA-256. Receipts hash both files and link
their names/digests. `goblin.array-plots.v2` pins the deterministic renderer and
embedded provenance profile. Historical alpha.30 policy v1 remains verifiable.
New receipts/freezes record it; adopting plotting under an older freeze without
that policy requires an explicit revision.

Verification reads the pair safely, rebuilds derived settings and rerenders the
figure. It compares **exact bytes and metadata**. Updating a changed picture's
checksum alone cannot conceal disagreement with its original data/settings.
Removing the sidecar fails verification. Checksums/reconstruction are not
signatures or authorship proof: someone replacing a complete internally
consistent run still needs to be checked against a trusted external hash or
checkpoint. Verification does not establish a scientific interpretation.

Audited compiled runs compare reference/native files and evidence. Standalone
compiled programs preserve a native manifest and pair, but do not replace a
full audited `goblin++ run`.

## Rendering limits and later stages

The canvas is 960 by 600, with a grid and one blue series. PNG has an embedded
case-sensitive bitmap font: Latin letters, digits and common scientific
punctuation. Unsupported glyphs are **refused**, with a suggestion to use SVG,
not replaced or uppercased. SVG accepts Unicode and escapes markup; viewer fonts
can vary, so visual font appearance is not cross-machine evidence.

Titles allow 70 display cells; complete axis labels allow 55 including prefix
and unit. Non-ASCII characters conservatively count as two cells. Controls and
bidirectional overrides are refused. Long labels are refused, not truncated.
Sidecars are capped at 16 MiB and figures at the existing 64 MiB per-file limit.
These are per-call limits, not a process-wide memory sandbox.

Error bars, multiple series/legends, configurable styles and log axes are later
stages, not features claimed here. JPEG is not supported: PNG/SVG retain crisp
scientific figures without lossy compression.
