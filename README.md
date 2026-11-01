# Alpha.45 local — two-computer LAN starter

Adds `net_pair_credentials(new_directory, server_ipv4)` for a dedicated 90-day
TLS pair with the server's actual private IPv4 SAN, separate role credentials and
no retained CA signing key. The setup computer is trusted and initially sees
both keys. A production PKI/CSR/revocation workflow remains separate.

Start with [the LAN quickstart](examples/network/lan/QUICKSTART.md). Run the
source examples directly; separate compiled client/server programs are optional.
The bundle includes the runtime, sources and Linux build instructions. Existing
networking, recipient encryption and scientific workflows remain available.
Rust stays 1.92.0; dependency versions are retained. This is not a throughput
benchmark or independently audited production service framework.

Simple client/server examples: start with [NETWORK_EXAMPLES.md](NETWORK_EXAMPLES.md)
and the clearly named `examples/network/` folder.

# Alpha.44 local — TLS networking and recipient encryption

Adds callable `net_*` client/server functions over mutually authenticated TLS 1.3,
plus age X25519 `e2e_*` recipient-message encryption. Localhost is allowed by default;
other numeric endpoints require explicit permission. Messages, credentials and
operation durations are bounded. Functions require `GO_PRIVATE` to keep plaintext
and keys outside scientific receipts. These are tested alpha facilities, not an
independent security certification or a complete authenticated messenger.

Start with [the networking guide](docs/NETWORKING.md) and
`examples/network_setup.gbl`, `network_server.gbl`, `network_client.gbl` and
`network_relay.gbl`. Both interpreter/native execution and IDE Run are supported.
VS Code 0.1.35 / JetBrains 0.3.25 include completion/help. Rust stays 1.92.0;
existing locked dependency versions are retained.

# Alpha.43 local: callable encryption/decryption

Use the file-protection functions directly from `.gbl` code with `GO_PRIVATE`:
`input_secret`, `encrypt_text`, `encrypt_file`, `decrypt_text`, `decrypt_file`,
`read_text` and `display_file`. Both interpreter and native compilation support
these functions; updated VS Code 0.1.34 and JetBrains 0.3.24 mask secret input
through the normal Run action. [Code guide](docs/PRIVATE_FUNCTIONS.md).

Private scripts create no scientific receipts and cannot silently weaken
`GO_PARANOID`/`COMPLETELY_MAD`. Standard age encryption and the existing CLI
file actions remain available. Rust remains 1.92.0; no dependency upgrades.

# Alpha.42 — confidential file commands and Base64 (local, unreleased)

Adds standard age-v1 passphrase encryption/decryption, hidden password entry,
no-overwrite file outputs, authenticated-before-display text previews, and four
file actions in both IDE plugins. These confidential operations run separately
from scientific evidence: no plaintext input copies or password receipts.
`str_b64`, `str_to_b64` and strict UTF-8 `str_from_b64` work in both engines;
Base64 is encoding, not encryption. Literal `-key` is supported with a warning;
hidden entry is the default. See [File protection](docs/ENCRYPTION.md).

Rust remains 1.92.0. New locked crypto/password dependencies are used. ZIP and TLS certificate
dependencies receive security fixes; the reader policy records ZIP 9.0.0. No public publication or independent security
certification is claimed by this local candidate. The previous alpha.41 release
remains preserved unchanged.

# Goblin++ Rust Engine 0.1.0-alpha.41 (local, unreleased)

Alpha.41 adds native NumPy NPY/NPZ, SciPy sparse NPZ (CSR/CSC/COO), and an explicit Astropy ECSV scalar subset. No Python runtime is required. Inputs are bounded, snapshotted and replay-verified; exact IDs and declared units receive checked conversions. See [scientific readers](docs/SCIENTIFIC_READERS.md) and `examples/scientific_readers.gbl`. VS Code 0.1.32 / JetBrains 0.3.22 add completion and help.

## Previous stage: alpha.39

Alpha.39 introduces the first `COMPLETELY_MAD` evidence-profile stage:
automatic bounded final-state seals, separate explicit snapshots, partial-failure
labels, native/reference parity and offline verification. It implies
`GO_PARANOID`, refuses inline Rust, and does not claim encryption or scientific
validity. See [the profile guide](docs/COMPLETELY_MAD.md) and
[the worked example](examples/completely_mad.gbl).

```sh
goblin++ run examples/completely_mad.gbl
goblin++ run examples/completely_mad.gbl --compile
goblin++ verify <RUN_DIR printed by the chosen run>
```

VS Code 0.1.30 and JetBrains 0.3.20 add directive completion/help/highlighting.
Rust remains pinned at 1.92.0 with no new dependencies. Older receipts remain
verifiable; old freezes require an explicit parser-policy migration revision.

## Historical stage: alpha.38

Alpha.38 adds **exact-ID CSV/TSV table joins** in both execution engines.
`csv_join` / `tsv_join` preserve catalogue IDs as text, require explicit inner
or left joins and column prefixes, refuse missing/duplicate keys and record
matched/unmatched counts. Offline verification reconstructs joined tables
from preserved input snapshots. This first stage is bounded, one-to-one and
serial; it does not silently expand rows or claim scientific validation.
See [the table join guide](docs/TABLE_JOINS.md).

```sh
goblin++ run examples/table_joins.gbl
goblin++ run examples/table_joins.gbl --compile
goblin++ verify <RUN_DIR printed above>
```

VS Code 0.1.29 and JetBrains 0.3.19 add join completion/help/highlighting.
Rust remains 1.92.0, with no new dependencies. Existing numerical audits and
scientific features are retained. At alpha.38, `COMPLETELY_MAD` was still proposed.

## Previous stage: alpha.37 — named numerical consistency audits

Alpha.37 adds **explicit named scalar consistency audits** with
`GO_NUMERIC_AUDIT "numeric_audit.json"` and `run --compile`. The interpreter
reference and native execution must agree exactly on selected finite scalar
values and dimensions. Configuration snapshots, frozen policy and offline
report reconstruction preserve failures and incomplete comparisons. This is
same-machine agreement, **not extra precision, independent algorithms or proof
of scientific correctness**. Rust remains 1.92.0; no dependencies are added.
VS Code 0.1.28 and JetBrains 0.3.18 add completion/help and highlighting.
Start with [the numerical audit guide](docs/NUMERIC_AUDIT.md) and:

```sh
goblin++ run examples/numeric_audit.gbl --compile
goblin++ verify <RUN_DIR printed above>
```

At alpha.37, `COMPLETELY_MAD` was a future profile, not a shipped directive.
No higher precision, interval arithmetic or encryption is implied.

## Previous stage: alpha.36 — explicit HTTPS acquisition

Alpha.36 adds an **explicit public HTTPS acquisition command**, `goblin++ fetch`,
with a named permitted host, checked public DNS destinations, verified TLS,
bounded streaming to a new private directory and offline `verify-fetch` receipts.
Use an independently obtained expected SHA-256 or explicitly select observed-only
recording. Redirects, proxies, cookies, credential headers and automatic content
decoding are disabled. No automatic `.gbl` networking, uploads, servers, 3-D
compression or integrated cube loading is added. See [secure downloads](docs/NETWORK_ACQUISITION.md).
This is a local, unreleased candidate; Rust remains 1.92.0. New locked HTTP/TLS
dependencies are used, and existing editor plugins remain compatible.

Alpha.35 added **Orbit read-only scoped baseline checks**, shared by
`goblin++ orbit` and standalone `goblin-orbit`. Explicit local scope, bounded
streaming hashes, no-follow traversal, independently pinned baselines and
preserved observation reports distinguish matches, review-required changes and
incomplete coverage. `MATCH` is not a clean-system verdict. No repairs,
escalation, network probing or image/audio origin detection is included.
Start with the disposable walkthrough in [Orbit](docs/ORBIT.md).
Both executables are included in the local installer; current editor plugins
remain compatible. Rust stays 1.92.0; the already locked libc version is now
also a direct Unix dependency for descriptor-relative traversal. No dependency
versions, science semantics or Chinese/community-review documents are changed.

Alpha.34 is [published on GitHub](https://github.com/mrgogobot/goblinpp/releases/tag/v0.1.0-alpha.34)
and [preserved on Zenodo](https://doi.org/10.5281/zenodo.23349396).

Alpha.34 adds **offline forensic/authorised-security reporting**, not scanners
or exploitation. `goblin++ evidence-report CASE.json --output NEW_DIRECTORY`
preserves expected-hash-pinned evidence copies and renders Markdown/HTML.
`goblin++ verify-report DIRECTORY` checks copies and independently reconstructs
report bytes; optionally match a separately retained receipt core hash.
Findings, confidence, severity, authorisation and collection times remain analyst
declarations. SHA-256 is byte integrity, not truth, signing or legal admissibility.
Start with [Evidence reporting](docs/EVIDENCE_REPORTING.md) and the synthetic
`examples/evidence-report/case.json`. Existing editor plugins remain compatible;
no new language semantics, dependencies, compiler upgrade or Chinese edits.
Exact-ID joins/richer plots/real science fixtures remain separate next stages.

Alpha.33 is [published on GitHub](https://github.com/mrgogobot/goblinpp/releases/tag/v0.1.0-alpha.33)
and [preserved on Zenodo](https://doi.org/10.5281/zenodo.23343997).

Alpha.33 adds element-wise numeric array operators and mathematical functions:
`sqrt(values)`, `a * b`, and explicit scalar repetition, with equal array lengths,
strict dimensions, independent copies and zero-based failing-element diagnostics.
No truncation, automatic reductions, matrix products, masks or hidden row drops.
New runs/freezes pin the array policy; historical receipts still verify, while
older whole-program freezes require an explicit revision before execution.
Start with `examples/elementwise_arrays.gbl` and [Array maths](docs/ARRAY_MATH.md).
VS Code 0.1.27 and JetBrains 0.3.17 explain the same rules. Rust remains 1.92.0;
dependencies and Chinese/community-review snapshots are unchanged. Exact-ID
joins, richer plotting and real WB/entropic regression fixtures remain next stages.

Alpha.32 is [published on GitHub](https://github.com/mrgogobot/goblinpp/releases/tag/v0.1.0-alpha.32)
and [preserved on Zenodo](https://doi.org/10.5281/zenodo.23326933).

Alpha.32 adds bounded serial `fits_batch_open` for fixed-width BINTABLE columns,
sharing the CSV/TSV incremental writers and EOF/close lifecycle. Exact catalogue
IDs remain text; null markers, units, budgets, full input snapshots and partial
outputs are audited. Interpreter/native parity and historical evidence are kept.
Start with `examples/fits_batches.gbl` and [FITS batch guide](docs/FITS_BATCHES.md).
VS Code 0.1.26 and JetBrains 0.3.16 add completion/help/highlighting. Rust stays
1.92.0; no new dependencies. No gzip, FITS writing, threading, physical blinding
or whole-process memory sandbox is added. Prior alpha.31 is published on
[GitHub](https://github.com/mrgogobot/goblinpp/releases/tag/v0.1.0-alpha.31).

Alpha.31 embeds a versioned provenance link in standard PNG/SVG array plots.
Exact data/settings form a canonical manifest core; its SHA-256 is embedded
before the finished image is hashed. Receipts link image, sidecar and manifest
digests. Verification checks metadata and reconstructs exact bytes. No circular
hashes, hidden timestamps, new codec, signing or authorship claim. Alpha.30 plot
evidence remains verifiable; frozen old plotting programs require an explicit
revision/child freeze to adopt policy v2. Existing FITS plots are unchanged.
See [Plot provenance](docs/PLOT_PROVENANCE.md). Both existing alpha.30 editor
plugins remain compatible; Rust stays 1.92.0 with no new dependencies.

Alpha.30 adds array-based `plot_line`, `plot_scatter` and `plot_histogram`
in PNG/SVG, with strict numeric/unit checks and no silent sampling or dropped
rows. Each figure includes exact data/settings evidence; `goblin++ verify`
rebuilds and checks the pair. Interpreter/native output parity is enforced.
See [Array plotting guide](docs/ARRAY_PLOTS.md) and `examples/array_plots.gbl`.
VS Code 0.1.25 and JetBrains 0.3.15 include completion/help. Rust stays 1.92.0;
no new dependencies. Multi-series, error bars and log axes are future stages.

Alpha.29 adds `time_date()` for captured system local date/time and
`time_date("utc")` for UTC, as ordinary text in **DD-MM-YYYY:HH:MM:SS** format.
Use it in everyday programs or scientific exports; stored values never refresh
on output. Audited receipts preserve every reading and its offset. Compiled
audit comparisons replay the same readings; standalone programs read their own
clock. This is system time, **not certified time or a monotonic timer**.
See [Captured date/time](docs/CLOCK_TIME.md) and `examples/clock_time.gbl`.
Both editor plugins include completion/help. Rust stays at 1.92.0.

Alpha.28 adds serial CSV/TSV batches and incremental file writers in both
engines: **load, process, release, next**. Explicit row/decoded-byte/file budgets,
exact catalogue-ID text, immutable disk snapshots, required EOF/close lifecycle,
and labelled `.partial` failure outputs are audited. Batch files may exceed
1 GiB, up to an explicit 64 GiB ceiling; existing eager/scan limits stay unchanged.
Start with `examples/batch_catalogue.gbl` and
[Serial batch guide](docs/BATCHES_STREAMING.md). No threads or whole-process RAM
quota are claimed. Rust stays at 1.92.0; no new dependencies.

Alpha.27 adds explicit-stream IID bootstrap/resampling, standard-normal draws,
flat row-major matrix operations, covariance and Cholesky/multivariate-normal
sampling, plus bounded CSV/TSV scans and an audited `GO_LOOP_BUDGET` declaration.
Both execution engines share the scientific helpers; successful compiled runs
compare reference/native outputs, RNG traces, helper summaries and loop counts.
No automatic confidence interval, covariance repair, random seed or unit
coercion. See [resampling](docs/RESAMPLING.md),
[matrices/covariance](docs/MATRICES_COVARIANCE.md), and
[resources/streaming](docs/RESOURCES_STREAMING.md). Start with
`examples/inference.gbl` and `examples/streaming_catalogue.gbl`.

This is a bounded foundation, not a complete out-of-core analysis system:
normal transforms can differ in their last bits across platforms; mixed-unit
covariance requires explicit standardization; streaming currently supplies
numeric column reductions plus alpha.28 serial batches, not automatic joins or
streaming bootstrap.
Rust stays pinned at 1.92.0; no new dependencies. At that stage, encryption and
`COMPLETELY_MAD` were separate proposals. Historical Chinese documents and
community-review material remain unchanged.

Alpha.27 versions the loop-budget grammar separately. Historical receipts remain
verifiable with their original parser; existing alpha.25/26 freezes require an
explicit revision and child freeze before adopting the new parser policy.

Alpha.26 adds explicitly seeded **scientific PCG32 randomness** in both engines:
`rng_seed`, `rng_word`, `rng_uniform` and `rng_integer`. Algorithm/version,
seed/stream, mappings and replayable draw evidence are preserved in receipts;
new freezes pin the RNG policy. No hidden seed or reseeding. See
[Randomness guide](docs/RANDOMNESS.md) and `examples/seeded_random.gbl`.
This RNG is **not suitable for cryptography**. Encryption and strict research
profiles remain future stages. No custom IDE is
planned; the existing VS Code/JetBrains plugins remain supported.

Alpha.25 adds compound-unit input in both engines: `1.13e-10 m/s^2`,
`3 kg*m^2/s^2` and `7 kg/(m*s^2)`. **`3 m^2` now means three square metres;
use `(3 m)^2` to square the whole quantity.** Review old unfrozen quantity powers.
Historical evidence uses the legacy parser; old freezes require an explicit
revision before execution under the new parser policy. See
[Compound units and migration](docs/COMPOUND_UNITS.md) and
`examples/compound_units.gbl`. Rust stays at 1.92.0, including pinned CI checks.
Alpha.24 statistics and alpha.23 FITS extraction are retained. No new
dependencies were added. `GO_MAD` remains future work.
Local tests do not replace this stage's cross-platform CI.

Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).

<img src="assets/goblinpp-logo.png" alt="Goblin++ goblin mascot with the motto A Pragmatic Language for Curious Minds; Built on Rust; Ideas Compile Here" width="300">

Goblin++ is an evidence-first scientific language. This release begins the audited migration from the Python 0.0.7 reference implementation to a native Rust engine.

> **Privacy warning:** Run evidence can preserve source code, `input()` prompts
> and responses, command-line arguments, imported libraries and FITS/CSV/TSV data, generated files,
> and stdout/stderr in plaintext. Do not use passwords, tokens, or confidential
> research data without reviewing where that evidence is stored and who can
> access it. `GO_PARANOID` and SHA-256 protect integrity, not confidentiality.
> See [Security and trust boundaries](docs/SECURITY.md).

## Downloads and documentation

- [简体中文：安装、学习与参考 / Simplified Chinese reader guide](docs/zh-CN/START_HERE.md)
- [Community translation review (unverified proposals)](community-review/README.md)
- [Goblin++ Alpha.12 Day-One Tutorial](docs/tutorial/Goblin++_Alpha12_Day-One_Tutorial.pdf)
- [Install the VS Code extension](vscode/README.md)
- [Build or install the JetBrains IDE plugin](jetbrains/README.md)
- [Electrical-engineering examples and function guide](docs/ELECTRICAL_ENGINEERING.md)
- [Statistics: sums, distributions, standard deviation and ECDF](docs/STATISTICS.md)
- [Seeded scientific RNG, explicit streams and evidence replay](docs/RANDOMNESS.md)
- [Compound-unit input and historical freeze migration](docs/COMPOUND_UNITS.md)
- [CSV/TSV, local modules and compiled scientific I/O](docs/DATA_MODULES_NATIVE.md)
- [Serial CSV/TSV batches and incremental outputs](docs/BATCHES_STREAMING.md)
- [Lossless numeric text and explicit presentation precision](docs/NUMERIC_TEXT.md)
- [Numeric comparison, math identity and reproducibility](docs/NUMERIC_REPRODUCIBILITY.md)

The ordinary command interprets a saved `.gbl` file:

```console
goblin++ experiment.gbl
```

Native compilation is explicit:

```console
goblin++ experiment.gbl --compile
goblin++ compile experiment.gbl -o experiment
```

The first form runs the native artifact and preserves the generated Rust, compiled binary, stdout, stderr, sealed scientific artifacts, and hashes in the run directory. The second form produces a standalone binary without running it.

Alpha.19 supports CSV/TSV input, local `import` function libraries, and native
FITS/file/plot calls. Data-using compilation requires Cargo and cached locked
crates; it builds offline. Standalone output does not include the launcher's
custody/freeze/postflight guarantees. See the linked guide before choosing a mode.

## Repository status

This source repository includes the Rust engine, its tests and examples, the
VS Code extension in [`vscode/`](vscode/), and the JetBrains plugin source in
[`jetbrains/`](jetbrains/). It does **not** contain a prebuilt
binary, personal run directories, or scientific input data beyond the small
deterministic FITS test fixture. The first public alpha is preserved on GitHub
and Zenodo; active development continues here. The remaining promotion gates are in
[RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md).

## Build and install

Rust 1.92 or newer is required.

```console
cargo test --locked
./install.sh --prefix "$HOME/.local"
goblin++ --version
```

This source checkout builds the command locally using the checked-in lockfile.
The installer uses a bundled macOS arm64 binary only when one is present in a
separate release archive; it builds from source here. You may pass
`--build-from-source` to make that choice explicit.

Cargo's internal binary target is named `goblinpp` because Rust crate identifiers cannot contain `+`. The installer exposes the intended command name, `goblin++`.

## Everyday and paranoid programs

Neither `GO_PARANOID` nor `seal` is required for an everyday program:

```goblin
samples = 12
accepted = 9
fraction = accepted / samples
print("accepted fraction = {fraction:.3f}")
write_text("answer.txt", "fraction = {fraction:.3f}")
```

The generated file remains confined to the unique `RUN_DIR/outputs` directory and is recorded and verified by SHA-256. Completed runs preserve source, logs, and a receipt; cleanly verified runs are registered in the checksum ledger. A failed paranoid self-check blocks ledger registration. `seal` is an optional declaration that snapshots a named scientific value as its own typed artifact; it is not needed for `print` or file output.

`GO_PARANOID` is an opt-in stricter evidence policy. It is source syntax and part of the canonical program:

```goblin
GO_PARANOID

mass = 1 kg
energy = mass * c^2
print("Energy = {energy}")
seal energy
```

In addition to baseline evidence, paranoid runs observe and preserve the source bytes again after execution. A different end-of-run hash refuses `PASS` and records a verifiable protocol violation; a missing source also refuses `PASS`. The receipt is independently checked before ledger registration. This is an end-of-run observation, not continuous monitoring: edits restored before that check cannot be detected by it. Frozen-byte enforcement, output confinement, data hashing, and exact-hash authorization for arbitrary inline Rust apply in either mode. `GO_PARANOID` is not an OS sandbox; see [SECURITY.md](docs/SECURITY.md).

## Electrical engineering

Alpha.17 adds electric current as the sixth SI base dimension, electrical
units such as amperes, volts, ohms, farads, and henries, and 18 `ee_` helpers:

```goblin
current = ee_current(12 V, 4.7 kohm)
milliamps = ee_in_unit(current, "mA")
print("current = {milliamps:.3f} mA")
tau = ee_rc_time_constant(10 kohm, 100 uF)
node_ok = ee_kcl_balanced([3 mA, -2 mA, -1 mA], 0.01 mA)
```

Ohm's law, signed power and energy, ideal passive components, resistance
networks, and explicit KCL/KVL residuals run in both execution engines. The
function guide documents signs, dimensional requirements, conversion output,
and model scope. See [ELECTRICAL_ENGINEERING.md](docs/ELECTRICAL_ENGINEERING.md)
and `examples/electrical.gbl`.

## Everyday control flow

`for`, `while`, `break`, `continue`, `if`/`else if`/`else`, and `switch`/`case`/`default` are Goblin++ syntax in both interpreter and compiled mode; no Python or inline Rust is needed. Boolean expressions use short-circuit `and`, `or`, and `not`:

```goblin
GO_PARANOID
total = 0
for i in range(1, 6) {
    if i % 2 == 0 {
        continue
    }
    total = total + i
}
for value in [2, 4, 6] {
    if value > 4 { break }
    total = total + value
}
remaining = 3
while remaining > 0 {
    remaining = remaining - 1
}
print("total = {total}")
seal total

if total > 10 and not total == 99 {
    verdict = "high"
} else {
    verdict = "low"
}
switch verdict {
    case "high" { code = 1 }
    default { code = 0 }
}
print("verdict = {verdict}; code = {code}")
```

`range` uses a half-open stop and accepts one, two, or three dimensionless integer arguments. `for value in array` iterates an independent array value. `%` is checked integer remainder and follows the dividend's sign. Comparisons and Boolean operators never infer numeric truthiness. `switch` uses exact, dimension-aware equality, first match, and no fallthrough. Nested loops share a one-million-iteration run limit, and exceeding it creates a preserved, verifiable failure. See [CONTROL_FLOW.md](docs/CONTROL_FLOW.md) and `examples/everyday_alpha12.gbl`.

## Scientific mathematics

Core mathematical functions work identically in the interpreter and native
compiler:

```goblin
root = sqrt(81)
distance = hypot(3 m, 4 m)
smallest = min(3 kg, 1 kg, 2 kg)
signal = sinr(pi / 2)
half = sind(30)
decades = log10(1000)
```

`abs`, `min`, `max`, and `hypot` preserve compatible dimensions. `sqrt`
accepts only non-negative values whose unit exponents are all even, so
`sqrt((3 m)^2)` is `3 m` while `sqrt(3 m)` is refused. `floor`, `ceil`,
`round`, `exp`, `ln`, and `log10` require dimensionless inputs. Trigonometric
names state their angle convention: `sind`/`cosd`/`tand` use degrees,
`sinr`/`cosr`/`tanr` use radians, and inverse or `atan2` names use the same
suffix for their result. `deg2rad` and `rad2deg` perform explicit conversion.
The old unsuffixed names temporarily retain radians behavior but emit a
preserved `G302` migration warning. Invalid domains and non-finite results
become explicit, preserved failures.
See [MATH.md](docs/MATH.md) and `examples/scientific_math.gbl`.

## Vectors, coordinates, and motion

The science helpers preserve dimensions and make conventions visible in the
function name:

```goblin
distance = au2m(1)
direction = spherical2cartesiand(2 m, 90, 45)
speed = velocity([10 m, 4 m, 0 m], 2 s)
normal = cross([1 m, 0 m, 0 m], [0 m, 1 m, 0 m])
omega = angular_velocityd(360, 2 s)
```

Distance conversion covers astronomical units, parsecs, and light-years in
both directions. `magnitude`, `dot`, and `cross` operate on homogeneous arrays.
Polar and spherical conversion uses explicit degree/radian suffixes; spherical
inclination is measured from +z and azimuth from +x in the x-y plane.
Velocity addition is never implicit: choose `velocity_add_galilean` or the
one-dimensional `velocity_add_relativistic_collinear`. Rotational helpers cover
angular velocity, tangential velocity, centripetal acceleration, and angular
momentum. These are Euclidean/elementary mechanics helpers, not curved
spacetime or general relativity. See
[VECTORS_COORDINATES_KINEMATICS.md](docs/VECTORS_COORDINATES_KINEMATICS.md) and
`examples/vector_motion.gbl`.

## Chemistry foundation

Common laboratory calculations use a small, versioned chemistry registry
rather than a new forest of keywords:

```goblin
water_molar_mass = chem_molar_mass("H2O")
water_amount = chem_moles(36.03 g, water_molar_mass)
stock = chem_concentration(0.1 mol, 100 mL)
diluted = chem_dilution(stock, 10 mL, 100 mL)
print("water amount = {water_amount}")
```

Both execution engines support exact-symbol atomic lookup, bounded formula
molar mass, mass/amount conversion, amount concentration, and explicit
dilution. Laboratory units include litres and microlitres, nanometres,
ångströms, common pressure units, and daltons. The element registry identity
and hash are preserved in run and freeze evidence. This foundation does not
infer isotopes, charge, hydrates, reactions, pH, equilibrium, kinetics, or
biological sequence semantics. See [CHEMISTRY.md](docs/CHEMISTRY.md) and
`examples/chemistry.gbl`.

## G funk: user-defined functions

Declare a function with `g_func`, then call it by name. `return` supplies its value:

```goblin
g_func energy(mass) {
    return mass * c^2
}

result = energy(1 kg)
print("Energy = {result}")
seal result
```

Functions work in the interpreter and native compiler. They may be declared after their call site. Parameters and local assignments stay inside the function; array arguments are independent copies. Every reached path must return a value. Recursion is limited to 16 active calls. Keep `GO_PARANOID`, `seal`, and authorized inline Rust at top level; seal the returned result in the caller. Functions may read FITS and write run-confined outputs in either engine; these native programs require Cargo and cached locked dependencies. See [FUNCTIONS.md](docs/FUNCTIONS.md) and `examples/functions.gbl`.

## Interaction and arrays

`input("prompt")` reads text; `argc` and `argv(i)` expose command-line arguments after `--`. Interaction evidence is preserved, so never enter secrets. Arrays support `[]`, indexing, indexed assignment, half-open slicing, `len`, copy-returning `append`, and direct `for value in array` iteration. Slices and iteration values are **independent copies**, not Go-style shared views. See [INTERACTION_AND_ARRAYS.md](docs/INTERACTION_AND_ARRAYS.md), `examples/greeting.gbl`, `examples/program_args.gbl`, and `examples/arrays.gbl`.

```goblin
greeting = input("Please enter your name:")
print("Hello, {greeting}!")
```

## Text operations: g_strings without a new mode

Text values now support `+` for concatenation and `len(text)` for Unicode-scalar length. `parse_number(text)` explicitly converts finite, unitless decimal input; `parse_integer(text)` accepts only signed or unsigned decimal integers within the exact `f64` range. `to_text(value)` renders a value for labels. `str_trim`, `str_contains`, `str_replace`, `str_split`, and `str_join` cover everyday text work in both execution modes:

```goblin
name = str_trim("  Ada  ")
message = "Hello, " + name
answer = parse_number("2.5") * 2
count = parse_integer("42")
print("{message}; answer = {answer}")
```

This is not a new `g_strings` type: it is a small, explicit set of operations on existing text values. Neither parser accepts units or integers beyond ±(2^53−1). See [STRINGS.md](docs/STRINGS.md) and `examples/strings.gbl` for the full contract.

## Native scientific data import

Inspect an unfamiliar FITS file before writing a program:

```console
goblin++ fits-info catalogue.fits --quick
goblin++ fits-info catalogue.fits --json
```

`--quick` reads only structural headers and explicitly reports that no checksum was computed. Without it, `fits-info` streams the entire file through SHA-256. Neither form creates run evidence.

The native importer handles primary images, image extensions, and fixed-width binary tables without Python or CFITSIO. HDU and row indexes are zero-based; image-axis indexes retain the FITS one-based convention:

```goblin
hdus = fits_hdu_count("catalogue.fits")
name = fits_header("catalogue.fits", 1, "EXTNAME")
rows = fits_rows("catalogue.fits", 1)
columns = fits_columns("catalogue.fits", 1)
first_class = fits_column("catalogue.fits", 1, "CLASS", 0)
mean_z = fits_column_mean("catalogue.fits", 1, "Z")
minimum_z = fits_column_min("catalogue.fits", 1, "Z")
maximum_z = fits_column_max("catalogue.fits", 1, "Z")
valid_z = fits_column_valid_count("catalogue.fits", 1, "Z")
stats = fits_select_stats("catalogue.fits", 1, "Z", 0.4, 0.6, "Z", "WEIGHT")
```

Existing primary-image calls remain valid. Each also accepts an explicit HDU:

```goblin
width = fits_axis("observation.fits", 0, 1)
pixels = fits_count("observation.fits", 0)
first = fits_pixel("observation.fits", 0, 0)
mean = fits_mean("observation.fits", 0)
```

Image values support `BITPIX` 8, 16, 32, 64, -32, and -64. `BSCALE`, `BZERO`, and integer `BLANK` are honored. Binary-table access supports `L`, `B`, `I`, `J`, `K`, `A`, `E`, and `D` fixed-width columns, including repeats, `TSCAL`, `TZERO`, and integer `TNULL`. Numeric statistics skip null integers and floating-point NaNs and use compensated summation.

`fits_select_stats` adds a bounded, full-table summary with an explicit
half-open numeric selection and optional positive weights. Its four-element
result reports selected rows, usable rows, sum of weights, and weighted mean;
it does not choose survey cuts or weights for you. See [FITS.md](docs/FITS.md)
and `examples/fits_selection.gbl` before using it for scientific work.

Inputs are opened read-only and processed with bounded memory. A run streams the entire input through SHA-256 and preserves a second verified copy in `.goblin/imports/SHA256.fits`; run directories hard-link that checksum-addressed object when the filesystem permits, avoiding one full duplicate per run. The bundled `examples/sample.fits` is deterministic and reproducible with `tools/generate_sample_fits.rs`.

ASCII tables can be discovered but not read. Random groups, compressed images, bit columns, complex columns, variable-length arrays, WCS interpretation and automatic unit conversion remain explicit limits. Alpha.19 supports the implemented FITS calls in compiled programs; see [native I/O](docs/DATA_MODULES_NATIVE.md).

See [FITS.md](docs/FITS.md) for the complete function and evidence contract.

## Audited files and plots

Generated files are run artifacts, not untracked side effects. They are written beneath `RUN_DIR/outputs`, recorded with their media type, size, producer, metadata, and SHA-256, and independently checked by `verify`.

```goblin
write_text("summary.txt", "rows = {rows}", "mean Z = {mean_z}")
write_csv("summary.csv", 2, "metric", "value", "rows", rows, "mean_z", mean_z)
write_tsv("summary.tsv", 2, "metric", "value", "rows", rows)
write_json("summary.json", "rows", rows, "mean_z", mean_z)

plot_fits_histogram("redshift.png", file, 1, "Z", 40, 50000, "Sampled redshift distribution")
plot_fits_scatter("z_error.svg", file, 1, "Z", "Z_ERR", 50000, "Redshift and reported error")
```

FITS plots use deterministic even-row sampling and preserve the sample method and counts in the receipt. SVG and PNG are supported. JPEG is intentionally omitted from the authoritative path because it is lossy. Filenames cannot escape the run directory, duplicate declarations are refused, and each output is capped at 64 MiB in this first stage.

Generated-output calls work with or without `GO_PARANOID`; the same confinement, duplicate-name refusal, size caps, and verification apply in both modes.

See [OUTPUTS.md](docs/OUTPUTS.md), `examples/everyday.gbl`, and the executable `examples/output_demo.gbl` tutorial.

## Inline Rust

Inline Rust is arbitrary native code. It therefore uses visible boundaries and exact-byte authorization:

```goblin
GO_PARANOID
x = 1

RUST_INLINE_BEGIN
println!("reviewed native operation");
RUST_INLINE_END

seal x
```

First obtain the block hash without executing it:

```console
goblin++ check program.gbl
```

Then review the block and authorize precisely that digest:

```console
goblin++ program.gbl --compile \
  --allow-inline-rust FULL_64_CHARACTER_SHA256
```

Changing one byte changes the digest and returns the policy to refusal. Inline blocks execute in source order and may read the generated `goblin_env`; the alpha contract does not let them mutate sealed Goblin++ variables. Interpreter mode always refuses inline Rust.

## Custody workflow

```console
goblin++ freeze experiment.gbl
goblin++ experiment.gbl
goblin++ verify experiment-runs/RUN_DIRECTORY

goblin++ revise experiment.gbl experiment_R1.gbl \
  --reason "document the scientific reason"
```

Frozen source changes fail before evaluation. A notation-only edit is still a protocol violation because freezing commits exact bytes, but its unchanged canonical meaning is reported separately. Refusals are preserved as verifiable runs.

Useful inspection commands are `check`, `fits-info`, `status`, `lineage`, `audit-ledger`, `doctor`, `diff`, and `capabilities`.

## Release status

This is an alpha foundation, not a declaration that the migration is complete. See [PORTING_MATRIX.md](docs/PORTING_MATRIX.md) and [SECURITY.md](docs/SECURITY.md) before scientific production use.

## Licensing

Goblin++ software, examples, test fixtures, and extension assets are licensed
under [MIT](LICENSE). Original tutorial/documentation prose and diagrams are
licensed under [CC BY 4.0](LICENSE-DOCS.md). Code examples inside the
documentation remain MIT-licensed. The documentation license file gives the
exact file scope; third-party dependencies keep their own licenses. The exact
locked macOS arm64 binary dependency inventory, upstream license files, and
Rust standard-library notices are preserved in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The
[project logo](assets/goblinpp-logo.png) is separate branding, described in
[BRANDING.md](BRANDING.md), and is not covered by either project license.
