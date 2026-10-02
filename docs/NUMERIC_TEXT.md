# Lossless default numeric text (alpha.21)

Default numeric text uses the shortest decimal representation that round-trips
the **stored finite f64**, including `-0`. This prevents output from silently
discarding precision. It does not make floating-point arithmetic exact or promise
identical transcendental results across platforms.

```goblin
value = 0.12345678901234567
saved = to_text(value)
restored = parse_number(saved)
same = value == restored
print("{saved}; {same}")       # 0.12345678901234566; true
print("{value:.3f}")          # 0.123, explicitly rounded
```

The source decimal above rounds to the nearest representable binary floating-point
value. Output recovers that stored value, not every digit of the source spelling.
Catalogue IDs beyond the exact f64 integer range still need the future exact
integer feature; this change does not make them safe.

## Where this applies

- `print(value)`, default `{value}` placeholders, array display and `to_text`.
- Numeric cells in `write_csv` / `write_tsv` and numeric `write_text` arguments.
- Both the interpreter and generated native code, using one shared formatter.

Nonzero magnitudes below `1e-4` or at least `1e15` use scientific notation;
other values use fixed notation. Exponents have an explicit sign. Scientific
notation lets `parse_number` read large floating-point values without weakening
its refusal of potentially rounded plain integer inputs.

`-0` remains `-0`; tiny/subnormal numbers are not replaced with zero.
`parse_number(to_text(value))` round-trips finite **unitless** values bit-for-bit.
Unitful values include their SI unit label: `to_text(2 kg)` yields `2 kg`,
which `parse_number` deliberately refuses. Compound-unit input remains queued.

Explicit `.Nf`, `.Ne` and printf precision remain presentation choices and may
lose precision. Plot labels/coordinates retain their existing presentation
precision; do not use image/SVG geometry as numeric data export.

JSON exports and sealed quantities already used round-trip serialization, while
native seal manifests already stored exact bits. These mechanisms are retained
and tested rather than replaced. Nonfinite results, overflow, nonzero underflow
in numeric parsing, and unsafe plain integer text remain refused.

## Frozen projects and historical evidence

Some decimal output and therefore its SHA-256 hashes change. Alpha.21 records
`goblin.eager-text-and-roundtrip-numbers.v2` in the hashed
`language_semantics` field of new run and freeze receipts.

Historical runs and old freeze integrity still verify. Running or compiling
an alpha.20 (or older) freeze under alpha.21 is refused as
`LANGUAGE_SEMANTICS_CHANGED_AFTER_FREEZE`, preserving a protocol-violation
run when launched through the audited runner. Do not delete or rewrite the
old freeze. Adopt the change explicitly:

```console
goblin++ revise experiment.gbl experiment_R1.gbl --reason "adopt lossless numeric text"
goblin++ freeze experiment_R1.gbl
goblin++ experiment_R1.gbl
```

Review the revised source before freezing it. Cross-policy comparisons report
`LANGUAGE_SEMANTICS_CHANGE` even if a particular program's output is unchanged.
For exact old output, retain the old engine and archived evidence.

Try `examples/numeric_text.gbl`. No dependencies, new maths functions, larger
resource limits or cross-platform deterministic math are added in this stage.
