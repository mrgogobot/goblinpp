# Public-release checklist

This folder is a **local source-repository draft**, not a published release.
The checklist keeps decisions visible instead of silently making them for the
author. The current Rust engine is `0.1.0-alpha.8`; the bundled editor extension
source is `0.1.3`.

## Before a public GitHub repository

- [x] Separate source and deterministic test fixture from builds, run records,
  generated outputs, and user research data.
- [x] Add a citation file naming Malin Hess. ORCID and DOI are intentionally
  absent until provided or minted.
- [x] Add a non-publishing GitHub Actions workflow for tests and build checks.
- [ ] Choose a software license and update both the root license file and
  `vscode/package.json`. Until then, the editor package says `UNLICENSED`.
- [ ] Review the repository file list and history for secrets, private data,
  third-party code, images, fonts, and permissions to redistribute them.
- [ ] Decide repository visibility and confirm the GitHub account/repository
  name. A private repository is a reversible first upload; public visibility
  is a separate decision.
- [ ] Establish a private security-reporting route and state supported versions.
- [ ] Confirm contribution policy and any required AI-assistance disclosure.

## Before calling an alpha release ready for researchers

- [ ] Run the workflow on GitHub, including macOS and Linux; fix any failures.
- [ ] Recheck `cargo test --locked`, Rust formatting/linting, and extension
  tests on a clean checkout.
- [ ] Reproduce the source archive and verify its SHA-256 on a clean machine.
- [ ] Review `docs/PORTING_MATRIX.md` and state unsupported language, FITS,
  custody, and compiler features in release notes. Do not claim parity with
  Python 0.0.7 while its normative-corpus and schema-migration gates remain open.
- [ ] Review `docs/SECURITY.md`; `GO_PARANOID` is evidence policy, not a sandbox,
  and the checksum ledger is not authenticated authorship.
- [ ] Validate representative scientific results against independently known
  values and publish the test data/assumptions that can be redistributed.
- [ ] Tag the exact version in Git; upload only reviewed binaries/assets and
  record their hashes. Do not replace a released tag or asset silently.

## Zenodo, after the GitHub release

- [ ] Connect the chosen GitHub repository to Zenodo and enable archiving.
- [ ] Make a GitHub release from the reviewed tag and confirm Zenodo ingests
  the intended archive. Check metadata and authorship before sharing the DOI.
- [ ] Add the minted DOI to `CITATION.cff` only after verifying it, and explain
  which release it identifies.
