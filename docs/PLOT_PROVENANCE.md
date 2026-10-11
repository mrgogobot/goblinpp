# Embedded plot provenance — alpha.31

Goblin++ keeps ordinary PNG and SVG files. `plot_line`, `plot_scatter` and
`plot_histogram` now embed a public, versioned link to their exact data/settings
manifest. No new plotting arguments or directive are required, and both the
interpreter and native compiler use the same implementation.

```goblin
GO_PARANOID
elapsed = [0 s, 1 s, 2 s]
distance = [0 m, 2 m, 5 m]
plot_line("motion.png", elapsed, distance, "Motion", "time", "distance")
plot_line("motion.svg", elapsed, distance, "Motion", "time", "distance")
seal distance
```

Run the shipped example from the extracted bundle root:

```sh
goblin++ examples/array_plots.gbl
goblin++ verify <RUN_DIR>
```

Replace `<RUN_DIR>` with the path printed by that run. From `examples`, it may
look like `goblin++ verify ./runs/<timestamp>_<source-hash-prefix>`. Keep the
whole run directory, including `receipt.json`, figures and `.plot.json` files.
`PLOT_PROVENANCE:<figure>`, `PLOT_POLICY_MATCH:<artifact>` and
`PLOT_RECONSTRUCTION:<artifact>` must pass, alongside the ordinary byte hashes.
`goblin++ examples/array_plots.gbl --compile` produces equivalent audited plots.

## One-way hash chain

1. Build a manifest core containing exactly the sidecar's `schema`, `input`
   and `derived` fields: versioned renderer policy, complete ordered typed
   data as IEEE-754 bit strings, labels, filename, bins, axes, ticks and layout.
2. Hash that core using Goblin++ canonical JSON: recursively sorted object
   keys, compact separators, ASCII JSON escaping, array order retained, no
   trailing newline. This is `manifest_sha256`.
3. Embed a canonical provenance object in the image, then hash **all finished
   image bytes**, including the metadata.
4. Write the sidecar with the core, manifest digest, provenance schema and
   finished-image digest. Hash the finished sidecar. The final run receipt
   records both artifacts and their linked metadata/digests.

```text
exact data/settings core → manifest hash → embedded image → image hash
           └──────────────────────────────→ sidecar → final receipt
```

Neither the final receipt hash nor the finished sidecar hash is embedded in
the image. Neither image hash nor run timestamp is part of the manifest core.
This avoids circular dependencies and allows identical inputs/settings/names
under the same renderer policy to reproduce identical figure bytes. A
different filename is a different manifest. Separately executed run receipts
still differ: run timestamps, environment and custody events belong there.
User-supplied changing data, such as a captured time in a title, changes the
figure normally. No authoritative data are rounded for reproducibility.

## Embedded profile

Schema: `goblin.plot-provenance.v1`; plot policy: `goblin.array-plots.v2`.

The four mandatory fields are:

| Field | Meaning |
|---|---|
| `schema` | Embedded provenance profile/version |
| `plot_policy` | Exact renderer/evidence policy |
| `manifest_sha256` | Lowercase SHA-256 of the independent core |
| `evidence_name` | Flat companion filename, e.g. `motion.png.plot.json` |

PNG uses an **uncompressed UTF-8 `iTXt` chunk** immediately after `IHDR`, with
keyword `Goblin++ provenance`, empty language/translated keyword and valid
PNG CRC. SVG uses escaped canonical JSON inside
`<metadata id="goblin-provenance">` immediately after the root opening tag.
These are standard metadata mechanisms, not hidden pixel watermarks. Other
software can display the figures without understanding the Goblin++ profile.

The receipt's artifact metadata and sidecar repeat the manifest digest and
profile. Verification reconstructs the core/link and compares exact regenerated
image/sidecar bytes and artifact metadata. A bare image supplies only a locator,
not enough evidence for a verified run; the digest does not uniquely identify
one run if multiple runs used the same data/settings. Match the finished image
against a trusted run receipt to establish that relationship.

## Boundaries and failure behavior

- Stripped metadata reports `PROVENANCE_MISSING` and fails verification of a
  v2 plot. It is not silently accepted as a legacy plot.
- Altered metadata, data/settings, pixels, extra image bytes or substituted
  partners fail the exact checks. Recomputing an image's ordinary hash alone
  cannot rescue its original manifest/reconstruction.
- A mismatch means "not the exact recorded artifact," **not proof of fraud**.
  Resizing, editing and re-exporting can also change bytes.
- This is **checksum-only consistency**, not authentication, secrecy or a
  guarantee of scientific correctness. A complete internally consistent fake
  can only be distinguished using a trusted external original digest/checkpoint
  or a future signature with a trusted public key. Signing is not implemented.
- Provenance JSON is bounded to 4096 decoded bytes; PNG traversal is bounded
  to 4096 chunks and validates CRC, lengths and final IEND. Duplicate metadata,
  unknown/duplicate fields, unsafe names and unsupported encodings refuse.
  The SVG reader recognizes only this canonical profile: no general XML
  parser, entity expansion, scripts, external references or network requests.
  Full reconstruction rejects any additional image content. Existing figure
  (64 MiB) and sidecar (16 MiB) limits remain in force. These are not a
  whole-process memory sandbox.

## Historical evidence and freezes

Alpha.30 policy `goblin.array-plots.v1` is still reconstructed using its original
image/sidecar bytes without retroactively adding metadata. Its receipt does not
claim embedded provenance. New executions use v2. Changing a v2 artifact's
policy to v1 alone cannot bypass the receipt's policy-match check.

An old frozen plotting program must be explicitly revised and its child frozen
before adopting v2. Old non-plot frozen programs remain executable if their
other contracts verify. Original files and receipts are never rewritten.
Existing FITS histogram/scatter outputs retain historical bytes and do **not**
claim this array-plot provenance profile. Existing VS Code 0.1.25 and JetBrains
0.3.15 plugins remain compatible; no plugin reinstall is needed for these
unchanged function signatures. Rust remains 1.92.0; no new dependencies.

Optional signed/self-contained evidence bundles are future work, not a new
image codec or a feature claimed by this release.

Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).
