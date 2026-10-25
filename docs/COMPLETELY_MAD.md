# COMPLETELY_MAD — bounded final-state evidence, stage 1

Alpha.39 introduces an opt-in profile that implies `GO_PARANOID` and captures
final user-global values in a separate, hashed `final-state.json`. It does not
insert lines into the source or change the ordinary `seal` snapshot semantics.
This is the first evidence-profile stage, not every proposed strict-policy gate.

```goblin
COMPLETELY_MAD

mass = 2 kg
seal mass
mass = 3 kg
energy = mass * c^2
print("energy = {energy}")
```

`mass.json` retains the explicit 2 kg snapshot. `final-state.json` contains final
mass 3 kg and energy, with SI dimensions preserved. The receipt's `final_state`
section records the policy, inventory, byte count, SHA-256, completeness and
execution origin. Explicit artifacts remain in `sealed_artifacts` unchanged.

## Run and verify

```sh
goblin++ run examples/completely_mad.gbl
goblin++ verify <RUN_DIR printed by that run>
goblin++ run examples/completely_mad.gbl --compile
goblin++ verify <RUN_DIR printed by that compiled run>
```

Use the actual directory printed by each run, without literal angle brackets.
Each execution creates new evidence. `verify` checks saved evidence without
rerunning the scientific program. `check --json` reports a preview only; it
does not create a receipt or certify custody.

## Precisely what is captured

- Only names successfully assigned by user code in the global execution context,
  including reached branches and loops. The final value is captured once.
- Numbers with dimensions, text, Booleans and flat arrays, including empty arrays.
- Updates through indexed assignment appear in the final array.
- No function-local bindings, function definitions, unused constants, implicit
  `argc`, unexecuted assignments or loop-induction-only names. A loop counter
  explicitly assigned as a global is still a global and its last value is captured.
- No assignment-by-assignment history or reconstruction of released batches.
  Closed resource-handle text is only text, not a saved external resource.

The directive must occur once as the first statement (comments/blank lines are
allowed before it), before imports and function definitions. It is not allowed
inside a function, branch, loop or library. Parentheses are not accepted.
Inline Rust is refused, even with an allowed hash, before user effects.

## Bounds and failures

At most **256 distinct global bindings** and **8 MiB of serialized final-state
JSON** are admitted. Exceeding either refuses capture and a successful run claim;
there is no truncation or silent exclusion. These are evidence bounds, not a
process-memory quota. Existing array, text and loop limits also apply.

Successful runs record `CAPTURED` / `COMPLETE`. Failed runs with capturable state
record `CAPTURED` / `PARTIAL`. Before evaluation begins, `NOT_STARTED` is possible;
failed capture is `REFUSED`, with a reason. Original source, explicit snapshots
and failure evidence are preserved. No final capture is promised after a process
kill, power loss, panic or unrecoverable storage failure.

For compiled runs the launcher first computes an interpreter reference, then
checks native final values bit-for-bit, including dimensions and inventory, as
well as existing stdout/data/resource parity. Final native manifest keys use a
separate `final:` namespace, not user seal names. A failed compiled attempt labels
captured state `INTERPRETER_REFERENCE` / `PARTIAL`, never completed native state.
Standalone native execution without the audited launcher is refused for this
profile; use `run --compile` so custody checks and receipts are available.
The standalone guard is a launcher-integration check, not authentication against
an operator who forges environment variables or changes the executable.

## Verification and trust limits

Both interpreted and compiled runs identify the selected profile as
`GOBLIN++ COMPLETELY_MAD MODE`, with `EXECUTION_PROFILE=COMPLETELY_MAD` and
`COMPLETELY_MAD_POSTFLIGHT=PASS` on successful postflight. `verify` likewise uses
`COMPLETELY_MAD_` labels for the inherited integrity checks. Ordinary
`GO_PARANOID` runs retain their existing banner and postflight labels.

New receipts and `check --json` explicitly record `execution_profile` as
`COMPLETELY_MAD`, `GO_PARANOID` or `EVERYDAY`. Verification compares it to the
preserved source. The legacy receipt fields `paranoid_mode` and
`paranoid_postflight` remain for compatibility: they record the shared checks,
not the selected profile. Older receipts without `execution_profile` remain
verifiable without modification.

Offline verification binds the directive to the recorded policy, checks the
captured inventory and file hash/size, validates source-scope/completeness
structure, and for successful compiled runs checks final values against the
preserved native manifest. It does **not** replay arbitrary branches or prove
the scientific calculation. A failed run can have intact, verifiable evidence.

Final values can contain sensitive information. The file is created private
(0600 on Unix), but it is plaintext: **sealing is not encryption**. Do not put
passwords, private keys or other secrets in automatically captured variables.
Hashes are not signatures; an independently trusted digest or future signature
is needed to detect a fully rewritten bundle. This is not an OS sandbox,
physical blinding system, higher-precision arithmetic or cross-platform
bitwise-mathematics guarantee. Numerical-audit requests remain explicit.

Ordinary programs and `GO_PARANOID` alone do not gain automatic seals.
Historical receipts keep their original parser interpretation. Older freezes
remain verifiable but require an explicit revision/new freeze before executing
under the new parser policy; do not rewrite their receipts or source snapshots.

Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).
