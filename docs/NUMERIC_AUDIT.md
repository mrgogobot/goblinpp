# Named numerical consistency audit — alpha.37

Local implementation, not a publication announcement.

`GO_NUMERIC_AUDIT` requests an explicit consistency check between the interpreter
reference and the compiled execution **on the same machine**. It compares named
scalar seals without rounding away disagreements. It complements `GO_PARANOID`;
neither directive proves that a scientific model or answer is correct.

## Start here

From the package root, run the supplied example:

```sh
goblin++ run examples/numeric_audit.gbl --compile
goblin++ verify <RUN_DIR printed by that command>
```

Replace the entire angle-bracket placeholder with the actual run directory.
The first command prints `area = 9 m^2; diagonal = 5 m`, plus run evidence and
`NUMERIC_AUDIT_STATUS=PASS`. The second independently reconstructs the saved
comparison; `VERIFICATION_STATUS=PASS` is evidence consistency, not science validation.

For your own program, put a JSON configuration alongside the source:

```goblin
GO_PARANOID
GO_NUMERIC_AUDIT "numeric_audit.json"

side = 3 m
area = side * side
seal area
```

```json
{
  "schema": "goblin.numeric-audit-config.v1",
  "comparison": "interpreter-native-same-bits",
  "quantities": ["area"]
}
```

The directive is allowed once, at main-source top level, not in an imported
library, loop, branch or function. `GO_NUMERIC_AUDIT = 2` remains an ordinary
identifier assignment for historical compatibility; use the quoted configuration
form to request an audit. Editor highlighting is a hint, not the parser authority.

## What is actually checked?

Policy: `goblin.named-scalar-native-consistency.v1`.

- Explicitly selected **finite f64 numeric scalar** seals; no array reduction,
  implicit text conversion or exact-integer promotion.
- Exact 64-bit value patterns, including the sign of zero, and all six SI
  dimension exponents: mass, length, time, temperature, amount, current.
- Both executions use the existing argument/input, clock replay, seeded RNG,
  imported-data and resource evidence contracts. Existing all-seal native parity
  and output checks remain enforced; selecting fewer seals does not weaken them.
- The exact configuration bytes are copied to `RUN_DIR/numeric-audit-config.json`
  and hashed. The report is the receipt's `numeric_audit` object, containing
  configuration identity, selected names, SI values, bit strings, dimensions,
  individual statuses and the overall outcome. Existing machine/compiler/math
  metadata remain in the receipt.

There is no tolerance in this first mode. Unknown fields, including tolerance
fields, and unknown modes are refused. Existing explicit tolerance comparison
functions remain separate; authoritative seals are not rounded for agreement.

## Outcomes

| Audit status | Meaning |
| --- | --- |
| `PASS` | Every selected scalar's bits and dimensions agree, and the run succeeded. |
| `FAIL` | At least one comparable selected scalar disagrees. |
| `INCONCLUSIVE` | A selected seal is missing or not a numeric scalar, with no comparable mismatch. |
| `ERROR` | Audit could not complete, or another run failure invalidated success. |
| `NOT_REQUESTED` | No directive; ordinary behavior is retained. |

`comparison_completed` records that comparison rows were assessed; it is **not**
an assertion that every requested scalar existed. Missing quantities remain
`INCONCLUSIVE`. `FAIL`, `INCONCLUSIVE` and `ERROR` cannot yield `RUN_STATUS=PASS`;
they produce preserved failed runs, normally `MACHINERY_FAIL`. Frozen-policy
violations retain their separate `PROTOCOL_VIOLATION` run status. Inspect the
receipt and saved error, not the audit status alone.

The explicit `--compile` is required: compilation is never silently enabled.
An interpreter-only invocation or missing compiler preserves `ERROR`, not an
unchecked success. Inline Rust is refused in this audit mode. Standalone
`goblin++ compile` refuses this directive because its executable alone would not
perform the launcher-owned reference comparison. `goblin++ check` validates
configuration and performs its existing preview, but labels the audit
`NOT_PERFORMED_PREVIEW_ONLY`; it does not create a certified run. Direct use of
the Rust evaluator/compiler library is not an audit and issues no audit receipt.

## Freeze, verification and boundaries

`freeze` also pins the exact configuration bytes. A changed, missing or invalid
configuration after freeze refuses execution with
`NUMERIC_AUDIT_CONFIG_CHANGED_AFTER_FREEZE`; create an explicit revision for an
intentional policy change. Snapshotted configuration bytes are what the run
uses; there is no claim of continuous filesystem monitoring.

`verify` uses preserved source, configuration, scalar artifacts and native
manifest to reconstruct the full report offline without rerunning the program.
Changed comparison rows, status, mode, configuration or missing new-policy
evidence are rejected even if only the receipt core is rehashed. Old non-audit
receipts/freezes remain supported; absence and verified `NOT_REQUESTED` compare
equivalently. `diff` reports `NUMERIC_AUDIT_CHANGE` when audit evidence differs.

Configuration: at most 16 KiB, 1–16 unique ASCII seal identifiers (1–64 bytes).
Use a nonhidden ASCII `.json` basename (at most 128 bytes), not a path. Regular
files only; final-component symlinks are refused. Audit native manifests and
selected seal files are each limited to 1 MiB. These bounds are not a process
RAM sandbox or a defense against an attacker replacing every trusted file.

**Shared algorithms can share bugs.** Both paths still use ordinary binary64
and the existing platform math backend. This is not an independent MPFR auditor,
arbitrary precision, interval enclosure, cancellation diagnostic, convergence
test, cross-machine bitwise guarantee, authenticity signature or scientific
reliability certificate. Machine differences are preserved, not disguised.
An internally rewritten bundle still needs an independently trusted original
digest or future signature to distinguish it from the authentic evidence.

Alpha.39 implements the first [COMPLETELY_MAD](COMPLETELY_MAD.md) stage: bounded
automatic final-state evidence. This does not automatically request a numerical
audit, higher precision or every proposed strict-policy requirement. Broader
gates require separate specified, tested implementations; a dramatic name cannot
substitute for them. Encryption/key management remain separate roadmap items.

Optional [Goblin++ merchandise](https://h4k3rl1f3.myspreadshop.co.uk).
