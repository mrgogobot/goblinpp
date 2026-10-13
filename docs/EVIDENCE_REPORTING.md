# Offline forensic and authorised-security reporting

Alpha.34 adds **reporting**, not disk acquisition, scanning, exploitation or an
automatic vulnerability detector. Use it on evidence copies you are authorised
to analyse. Goblin++ makes no network requests or executes attachments in this
workflow. `GO_PARANOID` remains useful for the `.gbl` analysis that produced your
findings; the separate reporting command always performs its own checks.

## First report: no real target required

From the installed bundle's top-level directory:

```sh
goblin++ evidence-report examples/evidence-report/case.json --output training-report
goblin++ verify-report training-report
```

Open `training-report/report.html` in a browser, or read `report.md` in your
editor. HTML is a deliberately simple, self-contained plain-text presentation,
not a dashboard. It has no scripts or remote fonts, images or stylesheets;
only fixed local styling for wrapping/printing. All
analyst content is escaped; Markdown metacharacters are escaped too. Multiline
declarations are presented on one line with ` / ` separators; original text
remains unchanged in `case.json`. The evidence log is synthetic.

Every destination must be **new** and its parent must already exist. For a second
report use another directory such as `training-report-2`. There is no force or
overwrite option. The command prints `RECEIPT_CORE_SHA256`; retain that hash
independently of the report folder. Later verify against that trusted copy:

```sh
goblin++ verify-report training-report --expected-receipt-sha256 YOUR_64_HEX_DIGEST
```

Replace the placeholder with the actual digest. `--json` is available on both
commands. A nonzero exit means refusal/error, not a successful report.
`goblin++ verify RUN_DIR` still verifies **analysis runs**; `verify-report` verifies
**report bundles**. These are distinct evidence formats, not interchangeable.

## What is preserved

- `case.json`: exact original case-document bytes, not reformatted JSON.
- `evidence/0001.bin`, etc.: exact copies in supplied order, regardless of source
  extension. Attachments are not opened, rendered or executed automatically.
- `report.md` and `report.html`: readable inventory and declared findings.
- `report-receipt.json`: versioned policy, Goblin++ version, producer OS/CPU,
  system UTC creation time, case/evidence/output hashes and byte counts, and a
  canonical receipt-core SHA-256. The receipt does not hash itself circularly.

Creation checks every source digest against the manifest, then self-verifies
the completed bundle. Verification needs only the preserved bundle, not the
original source paths. It checks exact copies and independently reconstructs
both report renderings, rather than merely trusting a supplied output hash.
Unexpected files, nested directories and symlinks in the bundle are refused.
Do not edit or add files inside a completed bundle: create a new report instead.

## Writing your own case

Copy `examples/evidence-report/` to a working directory. Replace the log with
your approved evidence copies and edit `case.json` there. Paths resolve relative
to **that case JSON's directory**, not the terminal's current directory.
Nested ordinary paths such as `logs/auth.txt` are permitted. Absolute paths,
`..`, backslashes, colons and symlink components are refused. If your temporary
directory has a symlink alias (e.g. `/tmp` on macOS), use its real canonical path.

Obtain each digest before declaring it (this does not certify acquisition):

```sh
# macOS
shasum -a 256 logs/auth.txt
# Linux
sha256sum logs/auth.txt
```

Required top-level fields:

| Field | Meaning |
| --- | --- |
| `schema` | Exactly `goblin.evidence-case.v1` |
| `case_id` | Unique identifier for your case |
| `purpose` | `security_assessment` or `forensic_analysis` |
| `title`, `author` | Analyst-supplied title and author credit |
| `authorization` | Your declared permission/basis and limitations; not verified permission |
| `scope` | 1–64 explicitly declared targets/evidence boundaries |
| `summary` | Analyst summary, not independently validated |
| `evidence` | 1–128 objects with `id`, `path`, expected `sha256`, `description`, `collected_utc` |
| `findings` | 0–128 finding objects; zero findings never proves a clean system |

Every finding requires `id`, `title`, `severity`, `confidence`, `observed_utc`,
`observation`, `impact`, `recommendation` and `evidence` (a nonempty list of known
evidence IDs). Severity is explicitly one of `critical`, `high`, `medium`, `low`,
`informational`. Confidence is `confirmed`, `suspected` or `inconclusive`.
**Both are analyst declarations**: no severity score, confidence level, CVSS
rating or conclusion is inferred by the engine.

IDs use 1–64 ASCII letters, digits, underscores or hyphens. Duplicate evidence
IDs/paths, duplicate finding IDs, duplicate/unknown evidence references, unknown
fields, missing fields and duplicate JSON object fields refuse. Do not include
an unsupported field expecting it to appear in a report.

Times must be explicit RFC3339 UTC, e.g. `2026-10-13T14:00:00Z` (or `+00:00`).
Collection/observation times are declarations; the report's creation time is
read from the system clock. Neither is a certified timestamp. Text is UTF-8,
nonempty, <=8192 bytes per declaration; unsafe control and bidirectional override
characters refuse. Code and statements in text are treated only as text.

## Using Goblin++ analysis results

The separate `examples/security_lab.gbl` demonstrates everyday analysis syntax:
it compares a **synthetic** count with an explicit threshold, writes an
observation and seals the count and Boolean. Try both engines:

```sh
goblin++ examples/security_lab.gbl
goblin++ examples/security_lab.gbl --compile
goblin++ verify THE_PRINTED_RUN_DIR
```

Use the actual `RUN_DIR` printed by each execution. Copy relevant outputs into
your case directory, calculate their hashes and reference them in your manifest.
You can include a run receipt as an evidence file, but **hashing a run receipt
alone does not reverify its run**: retain the original complete run separately
and use `goblin++ verify` for it. This stage does not automatically join runs,
derive findings, parse arbitrary logs or validate forensic conclusions.

## Confidentiality, limits and trust

Everything in the bundle is **plaintext**. There is no automatic redaction,
encryption, secret detection or sharing. On Unix new directories use mode 0700
and files 0600 (subject to a more restrictive umask); review OS permissions and
backups yourself. Windows uses inherited OS access controls. Do not bundle
credentials, private keys, personal data or confidential logs without reviewing
what you are retaining and who will receive it. If redaction is needed, create
an explicitly described redacted evidence copy before packaging; retain the
original through your independently governed evidence process.

Case JSON is <=256 KiB. Each evidence file is <=8 MiB, the total evidence is
<=32 MiB, and there are <=128 files/findings. Inputs are ordinary files, not
directories, devices, symlinks or disk images. The implementation holds bounded
evidence copies in memory; it is not an unrestricted catalogue, disk-imaging,
streaming-forensics or process-wide memory-control tool.

Input bytes are opened read-only, but the operating system may update access
times. Analyse properly acquired copies, not original forensic media: this
command is not a hardware/software write blocker.

Invalid input refuses before creating the destination. On an I/O failure after
creation, partial files are preserved with an error naming the destination.
Do not distribute a partial bundle; inspect it and retry with a fresh name.
Creation uses exclusive writes, not replacement. Abrupt termination, disk
failure and hostile filesystem races are not crash-recovery/sandbox guarantees.

`PASS` means supported schema, pinned byte copies and report reconstruction
matched. **It does not mean an assessment passed, a system is safe, permission
was confirmed, the author authenticated, evidence correctly acquired, or a
finding is true or legally admissible.** Complete coordinated rewrites can
produce new consistent hashes. Independently retain the expected receipt core
hash (or use a separately governed signature/custody system). These reports
are not a certified forensic suite or a substitute for validated acquisition.

No new `.gbl` semantics or freeze-policy migration is introduced in this stage;
scientific analysis, `GO_PARANOID` and existing evidence verification remain in
place. Existing VS Code 0.1.27 / JetBrains 0.3.17 plugins remain compatible; no
plugin reinstall is needed. Rust remains 1.92.0, with no new dependencies.
AES-256-GCM and `COMPLETELY_MAD` remain proposals, not shipped controls.

Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).
