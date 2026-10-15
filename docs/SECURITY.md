# Security and trust boundaries

## Explicit public HTTPS acquisition (local alpha.36 candidate)

`fetch` is a deliberate separate command, not implicit networking during `run`,
`check`, `compile`, local imports or FITS reads. It permits public HTTPS GET on
port 443 to one explicitly named DNS hostname. Returned candidate addresses are
checked before being handed to the connector; private/local/special-use and
transition addresses are refused. Redirects and proxy environment settings are
not followed. Rustls verifies the server certificate/name using platform trust;
there is no insecure switch, credential-header API, cookie jar or client-cert API.

Transfer, header and payload limits are enforced; files go to a new private Unix
directory. Content coding is refused, not automatically decompressed. Successful
payloads are made read-only, but this is not immutable storage against their owner.
Raw paths/queries and response secrets are omitted from receipts and errors;
command lines, shell history and external diagnostic tooling can still expose
what a user supplies. Do not use passwords, tokens or signed-URL secrets here.

Expected-hash matches, explicitly observed-only downloads and failed acquisitions
remain distinct. Offline verification proves receipt/payload consistency, not
authenticated content authorship, honest server metadata, permission to retrieve,
certified time, scientific reliability or absence of malware. Files are never
automatically opened as executable content. Caller-supplied expected hashes need
their own trusted provenance.

This is not an OS egress sandbox, hostile-filesystem atomic acquisition, process-wide
memory/disk cap, cancellation of an OS DNS worker, or protection from compromised
OS trust/routing. DNS and platform trust/revocation services are outside the HTTP
host allowlist. Authorized inline Rust retains its existing full user authority.
Use trusted output parents. Future blinding/encryption profiles are not implied.
See [the exact policy and release gates](NETWORK_ACQUISITION.md).

## Orbit scoped observations

Alpha.35 Orbit reads only an explicitly selected local directory and compares
bytes/entry kinds with an externally hash-pinned baseline. It never follows
symlinks, executes inspected content, escalates, repairs, probes networks or
certifies a system clean. Incomplete/unsupported/unreadable/unstable coverage
is never `MATCH`. Permissions/ACLs are not compared as a security policy.
Reads may update OS access times. Directory handles and metadata checks do not
provide an atomic snapshot or protection from a privileged/hostile filesystem.
Use trusted output parents and acquired offline copies, not adversarial live
mounts. This is not a hard I/O timeout or OS sandbox.

Capture does not establish known-good state: trust/acquisition declarations
remain user claims. Retain baseline and report hashes independently; a rewritten
self-checksum is not an external anchor. Offline report verification checks
copies/consistency, not original contents or observation truth. Baseline anchoring
alone does not authenticate later observations. Partial/interrupted bundles
require review, never implied success. Users remain responsible for authorised
scope, confidentiality and lawful handling. No image/audio origin detector is
included. See [Orbit](ORBIT.md) for exact bounds and the disposable tutorial.

## Offline evidence reports

Alpha.34 `evidence-report` reads declared local evidence copies with expected
hashes and writes a new plaintext report directory. `verify-report` independently
reconstructs its Markdown/HTML, verifies the preserved copies and optionally
compares a trusted external receipt hash. It makes no network requests, performs
no scanning or exploitation and does not execute attachments. Reports contain
escaped inert content, not automatic links to external resources.

Authorisation, scope, timestamps and findings are analyst declarations; hashes
do not authenticate authorship, certify truth/permission/time, validate collection
or establish legal admissibility. It is not a certified forensic suite. Source
paths refuse symlink components and traversal; destinations refuse overwrite.
Unix outputs use private modes, but plaintext snapshots still require your own
access-control/confidentiality review. Inputs/copies have explicit size limits,
and partial I/O failures preserve files rather than claiming success. These are
not hostile-filesystem sandbox, crash-recovery or whole-process RAM guarantees.
See [Evidence reporting](EVIDENCE_REPORTING.md) before using real case data.

## Confidentiality of run evidence

Goblin++ preserves source and run logs. Depending on what a program does, its
run directory and evidence store can also contain unredacted `input()` prompts
and responses, program arguments, imported library and FITS/CSV/TSV bytes, sealed values, and
generated files. These records are not encrypted. Avoid passwords, access
tokens, personal information, or confidential research data unless you have
reviewed the storage location and operating-system access controls. Removing a
secret from the live source does not erase copies already preserved in run
evidence or backups.

`GO_PARANOID`, hashes, freeze receipts, and the custody ledger detect some
changes; none provides confidentiality. See
[Input, arguments, arrays, and slices](INTERACTION_AND_ARRAYS.md) for the exact
interaction evidence behavior.

## Everyday and paranoid evidence

Everyday runs still preserve the starting source, stdout, stderr, a hashed receipt, generated and explicitly sealed artifacts, relevant imports, and normally a checksum-ledger event. If a paranoid run fails its own receipt/evidence check, it records a machinery failure and does not register that unverified run in the ledger. Output-path confinement, frozen-source checks, and exact-hash inline-Rust authorization are not disabled when `GO_PARANOID` is omitted.

`GO_PARANOID` additionally re-reads the ordinary source file at postflight, preserves those observed bytes, compares their SHA-256 with the starting source, and refuses `PASS` on mismatch or unavailability. Failure to preserve postflight evidence is also recorded as a refusal. The runtime independently verifies the completed receipt and evidence before attempting ledger registration. Receipt schema v2 records this postflight state; `verify` continues to accept existing v1 receipts. The observation is not an atomic filesystem snapshot or continuous monitor: a temporary change restored before postflight may escape this check, and later modifications to the live source are outside the preserved run. The preserved starting source remains the execution authority.

Neither mode is an OS sandbox or an authenticated authorship claim. In particular, authorized inline Rust runs with the user's full filesystem and network authority.

## Serial batch I/O

Alpha.28 CSV/TSV batch calls use explicit limits and immutable private disk
snapshots, not whole-file RAM arrays. These limits bound I/O buffers and returned
batches, **not all process memory**: user copies, arrays and logs can still grow.
Snapshots preserve the entire input, not just selected columns, so projection
is not physical blinding. Handles are run-local text identifiers, not secrets.
Temporary staging is cleaned after ordinary completion; if standalone failure
preservation itself fails, `RECOVERABLE_STAGING_DIR` identifies retained staging.
Disk exhaustion, abrupt termination and hostile filesystem races are not
crash-recovery or sandbox guarantees. See [BATCHES_STREAMING.md](BATCHES_STREAMING.md).

## Inline Rust

Inline Rust has the authority of the user who runs the compiled program. It can read, modify, transmit, or delete anything that operating-system account can access. Goblin++ does not pretend a content hash is a sandbox.

The enforced controls are:

- interpreter mode refuses every inline block;
- compiled mode requires the exact SHA-256 of every block;
- the authorization is recorded in the run receipt;
- generated Rust is preserved for review;
- the native binary and generated source are independently hashed;
- a source edit invalidates both the block authorization and any source freeze;
- blocks cannot mutate Goblin++ variables through the supported alpha interface.

Run untrusted inline Rust only inside a real OS-level sandbox or disposable machine. `GO_PARANOID` strengthens evidence and refusal behaviour; it cannot make arbitrary native code harmless.

## FITS and scientific input

FITS files are untrusted binary input. The reader checks HDU/header block boundaries, required structural cards, supported `BITPIX` and `TFORM`, axis and row sizes, arithmetic overflow, truncation, nulls, and numeric scaling. Input symlinks are refused. Reads are positional against the held-open file and memory use is bounded independently of file size.

Ordinary runs hash the complete input and then hash the evidence copy; a concurrent in-place change therefore converts the run to a machinery failure. The checksum-addressed evidence store avoids repeated physical copies through hard links where supported. Damage to a stored object is refused rather than silently replaced. `fits-info --quick` skips hashing and is explicitly marked `QUICK_INSPECTION_UNHASHED_NOT_EVIDENCE`.

Alpha limits should be treated as limits, not invitations to infer unsupported astronomy semantics. Goblin++ does not yet interpret WCS, units named by FITS metadata, uncertainty models, ASCII-table values, compressed images, bit/complex columns, or variable-length arrays. A FITS column name such as `Z` is data, not an endorsed cosmological interpretation.

## Generated output

Alpha.19 extends compiled runs to scientific I/O using a locked, offline Cargo
support build. Shared support sources and native output manifests are hashed.
Native/reference parity checks are not a sandbox or a proof of scientific
correctness. A standalone executable lacks launcher custody/freeze/postflight
checks; use audited `--compile` runs for those checks.

Source programs cannot write arbitrary filesystem paths through the supported output API. Output names must be one ordinary filename: absolute paths, directories, `..`, and duplicate declarations are refused. The runtime creates files exclusively beneath the unique run directory with create-new semantics and a 64 MiB per-file bound. Verification constrains receipt paths back to that output directory before reading them.

Plot metadata binds the imported FITS digest and the deterministic sampling description. A plot is still a visualization, not a substitute for a declared scientific selection rule. JPEG is omitted because lossy pixels are unsuitable as the sole authoritative scientific artifact.

Inline Rust remains outside this boundary. An authorized native block has the user's full operating-system authority and can write anywhere that account permits.

## Local modules and delimited input

Imports accept relative, non-traversing `.gbl` paths below the entry directory,
refuse symlink path components, cycles and duplicate function names, and allow
only function definitions and imports in libraries. They never fetch packages.
Freeze pins raw module bytes; paranoid postflight observes imported hashes.
Like root-source observations, this is not continuous monitoring or an atomic
filesystem transaction.

CSV/TSV reads are bounded UTF-8 snapshots with explicit delimiters, headers and
numeric conversion. Missing or malformed numeric cells fail; values are never
silently skipped or assigned inferred units. These restrictions do not certify
the provenance or scientific validity of the input data.

## Loop limits

Interpreter and compiled loops share a default 1,000,000 loop-body-iteration
limit per run, including nested loops and function calls. Alpha.27 accepts one
main-source, top-level `GO_LOOP_BUDGET` declaration from 1 through 50,000,000;
the requested, effective and used budget is recorded in run evidence. Exceeding
it becomes a preserved machinery failure. This prevents ordinary accidental
infinite loops; it is not a wall-clock timeout or a sandbox. Scientific helpers
retain their separate input/work caps, and authorized inline Rust remains
outside this limit. `GO_PARANOID`, `GO_LOOP_BUDGET` and inline Rust blocks must
be top-level so loop control cannot silently alter their authorization model.

Alpha.27 streaming CSV/TSV reductions use bounded buffers and logical records,
not unrestricted in-memory tables. Imported files are still copied as plaintext
evidence inside the run directory. Large scans can therefore use substantial
disk space and retain sensitive data; streaming is not encryption or physical
blinding. See [RESOURCES_STREAMING.md](RESOURCES_STREAMING.md) for the explicit
file, record and work limits.

## Custody

The alpha ledger is tamper-evident through SHA-256 chaining but does not authenticate authorship. Its CLI reports `CHECKSUM_ONLY`. Ed25519 signing and crash recovery are pending port gates. Back up source and evidence directories independently.

## Dependencies

Release dependencies are pinned by `Cargo.lock`, and Cargo verifies registry package checksums. The release manifest hashes every shipped file except the manifest itself.
