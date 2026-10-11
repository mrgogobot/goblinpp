# Captured date and time (alpha.29)

Goblin++ supports date/time text in everyday programs and audited science.

```goblin
da_time = time_date()          # current system local date/time
utc_time = time_date("utc")    # current UTC date/time
local_time = time_date("local")
print("Captured UTC = {utc_time}")
write_text("run_time.txt", "Captured UTC: {utc_time}")
seal utc_time
```

Each call returns ordinary captured text in **DD-MM-YYYY:HH:MM:SS**
format, for example `11-10-2026:14:05:09`.
Assignment, printing, file writing and sealing do not refresh a stored timestamp.
No `GO_PARANOID` or `seal` is required to use the function.
Use UTC for shared scientific reports; the returned string alone does not
identify a time zone. Label exported fields explicitly as UTC or local.

## Audit contract

New run receipts contain `clock_policy` and `clock.readings` (empty if unused).
Each reading records call index, requested mode, UTC instant with nanoseconds,
captured UTC offset in seconds, time-zone label and exact returned text.
Local means the host system's local zone at that instant, not a guessed location:
the label is `system-local`, with its fixed offset recorded. UTC has zero offset.
Daylight-saving transitions may repeat local text; the preserved UTC instants
and offsets distinguish the calls. Display text has second resolution.

Successful audited compiled runs replay the interpreter reference's clock
readings in call order, so crossing a second boundary cannot create a false
native/reference mismatch. The private `clock-replay.json` is hashed, and native
clock evidence must match the reference. Standalone compiled programs read their
own system clock and preserve readings in their native-data manifest; that is
not a complete audited run receipt. `goblin++ check` reports `clock_preview`;
its preview readings are not the later run's readings.

Independent verification checks internal consistency and evidence hashes:

```sh
goblin++ examples/clock_time.gbl
# Use the actual RUN_DIR printed above:
goblin++ verify ./examples/runs/<actual-run-directory>
```

For example, from within an examples directory, a previously printed run path
can be verified as `goblin++ verify ./runs/20261010T212059.403367Z_cf5e6c73fe40`.
Do not copy that old path for a new run; use its new `RUN_DIR`.

Different clock readings are different inputs: exact diffs label an otherwise
unchanged program `CLOCK_INPUT_CHANGE`. Exact seals are never rounded or stripped
of timestamps to conceal those differences. A timestamp helps label results;
hashes, receipts and custody evidence provide their integrity linkage.

## Boundaries and refusal

- System time can be wrong, adjusted or forged. This is **not certified time**,
  a cryptographic timestamp service, elapsed-time measurement or a monotonic timer.
- Only zero arguments or one text mode (`"local"` or `"utc"`, lower case) are
  accepted. There are no custom format or time-zone arguments in this stage.
- At most 16,384 readings per execution; four-digit years only. Excess calls,
  malformed replay evidence and incomplete/out-of-order replay refuse explicitly.
- Discarding a result, such as a bare `time_date()` statement, refuses. User
  functions/parameters cannot shadow this builtin during execution; historical
  source remains parseable for verification.
- New freezes pin the clock policy. An older freeze without that policy can
  still execute a non-clock program; adopting clock calls requires an explicit
  revision and child freeze. Historical receipts remain independently verifiable.
- Clock evidence records system metadata, not a promise of physical blinding,
  sandboxing or resistance to an attacker controlling the machine.

Rust remains pinned to 1.92.0. No new dependencies, threads, encryption or
`COMPLETELY_MAD` implementation are introduced.

Clock-using native compilations use the existing locked Cargo helper runtime.
Have Cargo/Rust and the locked dependency cache available, as for native FITS,
file-output and other shared scientific helpers.
