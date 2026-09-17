# Public-release checklist

This is a **pre-publication source-repository draft**, not a public release. It
may be kept in a private GitHub repository while these gates are reviewed. A
commit or private upload is not a Zenodo release. The checklist keeps decisions
visible instead of silently making them for the author. The current Rust engine
is `0.1.0-alpha.10`; the bundled editor extension source is `0.1.5`.

## Before making the GitHub repository public

- [x] Separate source and deterministic test fixture from builds, run records,
  generated outputs, and user research data.
- [x] Add a citation file naming Malin Hess. ORCID and DOI are intentionally
  absent until provided or minted.
- [x] Add a non-publishing GitHub Actions workflow for tests and build checks.
- [x] Add MIT for software and CC BY 4.0 for original documentation prose and
  diagrams, with explicit scope and matching Rust/VS Code package metadata.
- [x] Confirm Malin Hess is the named rights holder for original project
  material. The GPT-generated project logo is separate branding under
  [BRANDING.md](BRANDING.md); dependency crates retain their own licenses.
- [x] Review the tracked file list and local Git history for obvious secrets,
  private data, third-party source/assets, and redistribution concerns. No
  personal run data or obvious secret was found in this snapshot; repeat the
  review if new material is added before publication.
- [ ] Decide repository visibility and confirm the GitHub account/repository
  name. A private repository is a reversible first upload; public visibility
  is a separate decision.
- [ ] Establish a private security-reporting route and state supported versions.
- [ ] Confirm contribution policy and any required AI-assistance disclosure.
- [x] Make the privacy warning prominent: `input()` responses, program arguments,
  source, imported data evidence, and run logs may be preserved in plaintext.
  `GO_PARANOID` and SHA-256 hashes do not encrypt them.

## Before calling an alpha release ready for researchers

- [ ] Run the workflow on GitHub, including macOS and Linux; fix any failures.
- [ ] Recheck `cargo test --locked`, Rust formatting/linting, and extension
  tests on a clean checkout.
- [ ] Finish the [dependency-license review](docs/DEPENDENCY_LICENSE_AUDIT.md)
  and include required third-party notices with any distributed binary or
  extension package. A manifest-level inventory is not a final notice bundle.
- [ ] Reproduce the source archive and verify its SHA-256 on a clean machine.
- [ ] Rebuild the VS Code extension package and inspect it for both MIT and
  CC BY notices; previously distributed VSIX files are not changed by edits
  to this source repository.
- [ ] Review `docs/PORTING_MATRIX.md` and state unsupported language, FITS,
  custody, and compiler features in release notes. Do not claim parity with
  Python 0.0.7 while its normative-corpus and schema-migration gates remain open.
- [ ] Review `docs/SECURITY.md`; `GO_PARANOID` is evidence policy, not a sandbox,
  and the checksum ledger is not authenticated authorship.
- [ ] Validate representative scientific results against independently known
  values and publish the test data/assumptions that can be redistributed.
- [ ] Tag the exact version in Git; upload only reviewed binaries/assets and
  record their hashes. Do not replace a released tag or asset silently.

## Valuable additions, not automatic public-alpha blockers

- [ ] Design and test **optional file and directory encryption** as described
  in [ENCRYPTION_PROPOSAL.md](docs/ENCRYPTION_PROPOSAL.md). Until implemented,
  make no confidentiality claim for run directories or exported files. This
  becomes a release blocker only if the first public release promises encrypted
  storage or transfer.
- [ ] Continue the pending Rust-port gates in
  [PORTING_MATRIX.md](docs/PORTING_MATRIX.md): normative-corpus parity, schema
  migrations, authenticated ledger authorship, interrupted-head recovery,
  legacy adoption, and broader FITS semantics. These block a claim of stable
  Python-reference replacement, not a candidly limited public alpha.

## Zenodo, after the GitHub release

- [ ] Connect the chosen GitHub repository to Zenodo and enable archiving.
- [ ] Make a GitHub release from the reviewed tag and confirm Zenodo ingests
  the intended archive. Check metadata and authorship before sharing the DOI.
- [ ] Add the minted DOI to `CITATION.cff` only after verifying it, and explain
  which release it identifies.
