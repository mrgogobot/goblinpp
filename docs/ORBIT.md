# Orbit: read-only scoped baseline checking

Alpha.35 adds a shared Rust library, `goblin++ orbit` and standalone
`goblin-orbit`. They use the same implementation. Orbit v1 checks the bytes and
entry kinds in **one explicitly selected local directory**. It is not a
vulnerability scanner, rootkit detector, malware detector or clean-system
certificate. No network targets, privilege escalation, execution of inspected
files, repairs, quarantine or automatic re-baselining are provided.

## Install and try a disposable folder

The local macOS arm64 bundle includes both executables and the existing editor
installers. From the extracted bundle folder:

```sh
./install.sh --prefix "$HOME/.local"
rehash
goblin++ --version
goblin-orbit --version
```

Linux builds use the retained Rust 1.92.0 toolchain and locked dependencies
through the same installer. This iteration is locally tested on macOS arm64;
Linux verification is a separate CI gate, not a claimed local result.

Begin with the supplied synthetic folder, not live system files:

```sh
cp -R examples/orbit-lab orbit-work
goblin++ orbit baseline orbit-work --output orbit-baseline \
  --reason "Initial disposable test" \
  --trust-declaration "I created this test folder; not a system certification" \
  --authorization "My own disposable folder only"
```

The command prints `BASELINE_SHA256`. **Save that digest independently** of the
baseline folder, in a trusted record. Set the variable below to that exact digest:

```sh
BASELINE_HASH="paste-the-64-lowercase-hex-digest-here"
goblin-orbit verify-baseline orbit-baseline \
  --expected-baseline-sha256 "$BASELINE_HASH"

goblin++ orbit check orbit-work --baseline orbit-baseline/baseline.json \
  --expected-baseline-sha256 "$BASELINE_HASH" --output orbit-check-1 \
  --author "Test analyst" --authorization "My own disposable folder only"
```

Open `orbit-check-1/report/report.html` or `report.md`. The bounded preview is
not the entire inventory: `orbit-check.json` contains all compared entries and
`observations.json` contains hashes, metadata, limits and coverage issues. Reports
have **no automatically asserted analyst findings**. They preserve observations.

Edit a file in `orbit-work` and repeat with a **new** output directory. This must
produce `REVIEW_REQUIRED`, not proof of an attack. No source file is repaired.

## Outcomes and verification

| Exit | Check status | Meaning |
|---|---|---|
| 0 | `MATCH` | Complete observed inventory agrees in file bytes and entry kinds. Not “safe”. |
| 1 | `REVIEW_REQUIRED` | Complete inventory has changed/missing/added entries. Review legitimate changes too. |
| 2 | `INCOMPLETE` or error | Unreadable/unsupported/unstable entries, exhausted budgets, invalid input or failed output. Never a pass. |

Individual classifications are `MATCH`, `CONTENT_CHANGED`, `MISSING`, `ADDED`,
`UNREADABLE`, `UNSTABLE_DURING_READ`, `UNSUPPORTED` and `NOT_OBSERVED`.
Missing cannot be asserted when coverage is incomplete: unobserved baseline
paths are conservatively `NOT_OBSERVED`. Empty directories are inventoried.
Metadata is recorded to detect some changes **during** reading; permissions,
ACLs, extended attributes, ownership and timestamps are **not** security-audited
or compared between scans. A permission-only change may still be `MATCH`.

Save the printed `RECEIPT_CORE_SHA256` independently as well:

```sh
goblin-orbit verify orbit-check-1 \
  --expected-baseline-sha256 "$BASELINE_HASH" \
  --expected-receipt-sha256 "paste-the-separately-saved-receipt-core-hash"
```

Offline verification validates the pinned baseline, strict observation schema,
coverage, derived classifications, preserved JSON copies and reconstructed report
bytes. It **does not revisit or rehash the inspected originals**. File contents
are not copied into the report. Without an independently trusted report hash,
an attacker who rewrites all observations and receipts can fabricate a consistent
report. A baseline anchor alone does not authenticate later observations.
These are checksum anchors, not digital signatures or certified clocks.
`orbit verify` can verify the consistent copies of an `INCOMPLETE` check;
its zero exit code does not change that recorded coverage status into a pass.
The offline reporter's no-scanning disclaimer describes report rendering;
Orbit's directory observation happens before that separate reporting step.

For a normal Goblin++ science run, continue using `goblin++ verify RUN_DIR`.
Orbit reports are separate artifacts, not `.gbl` runs or custody-ledger events.
No `GO_PARANOID` requirement is weakened or replaced by Orbit.

## Budgets and safety

| Option | Default | v1 hard maximum |
|---|---:|---:|
| `--max-entries` | 1024 | 4096, including directories and unsupported entries |
| `--max-depth` | 16 | 32 |
| `--max-file-bytes` | 256 MiB | 1 GiB |
| `--max-total-bytes` | 1 GiB | 8 GiB of bytes actually read |
| `--max-seconds` | 60 | 3600, cooperative deadline |

Positive integer limits are recorded in each snapshot. File hashing uses a
64 KiB buffer. Directory inventories, paths, diagnostics and JSON are bounded;
JSON files are limited to 8 MiB. Directory enumeration that exceeds its budget
is incomplete; no arbitrary “first N files” baseline is silently trusted.
These bounds are not a whole-process RAM sandbox or hard I/O timeout. A hung
filesystem call may not obey the deadline; use offline, reliable local copies.

No symlinks are followed, including root components, baseline inputs and output
components. FIFOs, sockets, devices, symlinks, unsafe/oversized names and non-UTF-8
names are unsupported. Entries are stably sorted. Unix descriptor-relative
traversal anchors directory handles; regular files are type-checked, opened with
no-follow/nonblocking flags, streamed and checked before/after with pathname
binding checks. This is **not an atomic or continuous snapshot**. Transient
changes restored before checks may escape detection. A privileged attacker,
hostile filesystem, or raced replacement of a regular file by a device can
undermine the checker; do not scan adversarial live mounts. Use properly acquired
offline copies for serious investigations. Output parents must be trusted and
not concurrently writable by an adversary; output handling is not an OS sandbox.

Baselines and reports must be **outside** the inspected directory. Destinations
must be new; parents must exist. Outputs use Unix 0700 directory / 0600 file
permissions and are plaintext. Scope paths, names and metadata can be sensitive.
Read-only means Orbit does not write target file contents or permissions; the OS
may update access times. Interruption/I/O failure can leave partial output;
an interrupted command must not be treated as complete or trusted. Run verification
before use, and retain partial artifacts for review rather than overwriting them.
Orbit scanning supports Linux/macOS only in v1; other Goblin++ features retain
their existing platform behavior. Unsupported platforms refuse Orbit scanning.

## Explicit re-baselining

Never change the saved trusted digest just to silence an unexpected mismatch.
Review legitimate updates, then capture into a **new** directory with the
immediate parent pinned and a required reason:

```sh
goblin++ orbit baseline orbit-work --output orbit-baseline-r1 \
  --reason "Reviewed and approved the legitimate update" \
  --trust-declaration "Review performed against my retained reference" \
  --authorization "My own disposable folder only" \
  --parent-baseline orbit-baseline/baseline.json \
  --expected-parent-sha256 "$BASELINE_HASH"
```

The old baseline is not overwritten. The immediate parent bytes are copied into
`parent-baseline.json`; its exact SHA-256 is linked in the new baseline.
`verify-baseline` validates that immediate parent copy against the link. Retain
the original baseline directories/anchors to inspect deeper ancestry; this is
not an authenticated, automatically traversed full custody chain. Root relocation
is refused rather than silently changing scope. A trust declaration records what
the user claims; capture itself does not establish known-good state.

## Responsibility and deferred features

Only inspect systems/evidence you are authorised to access. The user is
responsible for permission, scope, confidentiality, lawful handling, interpretation
and distribution. Observations do not prove compromise, authorship or legal
admissibility, and this notice is not a substitute for safe defaults, testing,
access controls or appropriate legal review.

Image/audio origin analysis, signed provenance, platform-specific system/package
baselines, hard isolation, signatures and automatic repairs are **not shipped**.
No checksum or absent provenance can tell you an image/audio file is “real”.
Modules are reviewed, compiled-in Rust code; Orbit never loads plugins from the
directories it inspects. Existing VS Code and JetBrains plugins remain compatible;
they have not gained an Orbit UI in this release.

See [Evidence reporting](EVIDENCE_REPORTING.md) and [Security boundaries](SECURITY.md).
Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).
