# Public-release checklist

This is the reusable checklist for changes after the public alpha.12 release.
A commit is not a new release, and each new version still needs its own review,
tag, assets, and preservation record. The checklist keeps decisions visible
instead of silently making them for the author. The current working Rust engine
is `0.1.0-alpha.13`; the bundled editor extension source is `0.1.8`.

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
- [x] Keep everyday text and prompted input in the language-support gates:
  `input("Please enter your name:")`, text concatenation, `len(text)`,
  `str_trim`/`str_contains`/`str_replace`/`str_split`/`str_join`, `to_text`, and
  checked `parse_number` and `parse_integer`. Interpreter/compiler and preserved-interaction tests
  are in `tests/interaction.rs` and `tests/g_strings.rs`; these operations are
  implemented, not deferred to a future release. Input remains plaintext
  evidence, not a password prompt.

## Before calling an alpha release ready for researchers

- [ ] Run the workflow on GitHub, including macOS and Linux; fix any failures.
- [ ] Recheck `cargo test --locked`, Rust formatting/linting, and extension
  tests on a clean checkout.
- [x] Finish the macOS arm64 [dependency-license review](docs/DEPENDENCY_LICENSE_AUDIT.md)
  and include the exact locked crate notices and Rust standard-library notices
  in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and `third-party/`.
  Regenerate this target-specific bundle whenever dependencies, toolchain,
  features, target, or packaging change. The extension has no npm dependencies
  and its VSIX contains both project license files.
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
