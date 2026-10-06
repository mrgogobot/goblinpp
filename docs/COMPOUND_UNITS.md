# Compound-unit input — alpha.25

Numeric literals can carry a compound suffix made from registered units:

```goblin
acceleration = 1.13e-10 m/s^2
area = 3 m^2
energy = 3 kg*m^2/s^2
pressure = 7 kg/(m*s^2)
frequency_squared = 4 s^-2
speed = 3 km/s
print("acceleration = {acceleration}; speed = {speed}")
seal acceleration
```

The coefficient belongs to the whole unit suffix. `3 m^2` means **three square
metres**, not nine. Unit scale factors are raised to their powers: `3 km^2`
is 3,000,000 square metres. Signs and scientific notation apply to the
coefficient normally. ASCII `^2` and the existing Unicode `²` notation have
the same canonical meaning; `³` is also retained. Whitespace around unit
operators is insignificant. No units, constants or conversion factors were added.

## Powers, grouping and ordinary arithmetic

```goblin
area = 3 m^2          # 3 m^2
whole = (3 m)^2      # 9 m^2: square the entire quantity
scaled = 3 km^2      # 3000000 m^2
product = 2 m * 3    # 6 m: the numeric operand ends the suffix
ratio = 2 m / 4      # 0.5 m
legacy_form = 1.13e-10 m / (1 s)^2
```

Bare units inside a numeric suffix are registry entries, not variable lookups.
They are not newly exposed as global values: `x = m/s^2` still refuses unless
those names are explicitly declared variables. Use `1 m/s^2` for a unit value.
Ordinary variable multiplication/division, constant resolution, function calls,
and `2(1+2)` keep their existing behaviour. Parenthesise a completed quantity
before further arithmetic involving bare unit factors.

Use a **single slash per unit group**, and parenthesise multiple denominator
factors. `7 kg/(m*s^2)` is accepted. `7 kg/m*s^2` and `7 kg/m/s^2` refuse with
`AMBIGUOUS COMPOUND UNIT`; Goblin++ does not guess which factors you intended
to put below the fraction bar. Nested denominator groups and a single grouped
unit, e.g. `5 m/(s)`, are allowed. A reciprocal-only suffix can be written
`2 1/s`, matching displayed inverse units. Several denominator factors are
now printed with parentheses, e.g. `kg/(m*s^2)`, in both engines.

Suffix powers are signed decimal integers between -32 and 32. Decimal,
scientific-notation, variable and chained exponents refuse. Combined dimension
exponents are bounded to [-256, 256]; unit groups may nest at most 32 levels.
A suffix is limited to 256 tokens to bound parsing and validation work.
Overflow, nonfinite unit scales and a unit scale that underflows to zero refuse.
Existing quantity arithmetic and dimensional checks still apply afterwards.
Use `(3 m)^exponent` when exponentiation of the whole quantity is intended.

## Important migration from alpha.24 and earlier

Earlier parsers interpreted `3 m^2` as `(3 m)^2`, and could not resolve the
bare denominator in `1.13e-10 m/s^2`. Alpha.25 deliberately corrects the suffix
meaning. **Review existing quantity powers before running unfrozen scripts.**
Write `(3 m)^2` to preserve the old whole-quantity interpretation.

New run, freeze, check and compilation evidence declares
`parser_policy = goblin.compound-unit-literals.v1`. Compound suffixes lower
to ordinary unit-quantity arithmetic in the AST; simple literals such as
`1 kg` and the reference energy program retain their historical canonical
hashes. This is syntax canonicalisation, not an algebraic equivalence solver;
the old workaround and the compact suffix can have different canonical hashes
even when their values and dimensions agree exactly.

Historical run and freeze evidence without this field is verified with the
**legacy parser**, including preserved imported libraries. It is not rewritten
or evaluated again under the new interpretation. Missing policy in alpha.25
or newer evidence, and unknown declared policies, fail verification.

An old freeze remains verifiable; `status` and `lineage` label it
`FROZEN_PARSER_MIGRATION_REQUIRED`. Executing or compiling it with alpha.25
refuses as `PARSER_POLICY_CHANGED_AFTER_FREEZE`. This applies even if its source
does not use compound units: a freeze must not silently adopt a new parser
policy. A refused run is preserved and independently verifiable. The explicit
migration path is:

```sh
goblin++ revise experiment.gbl experiment_R1.gbl --reason "reviewed alpha.25 unit-power semantics"
# Review/edit the child; preserve whole-quantity powers with parentheses.
goblin++ freeze experiment_R1.gbl
goblin++ experiment_R1.gbl
```

`diff` reports `PARSER_POLICY_SAME` and classifies policy differences as
`PARSER_POLICY_CHANGE`, separately from exact output equality. Checksums are
integrity evidence, not cryptographic proof of authorship. No authoritative
seals are rounded, and this release makes no universal deterministic-math claim.

## Run the example

```sh
goblin++ examples/compound_units.gbl
goblin++ examples/compound_units.gbl --compile
```

The engine remains built and tested with **Rust 1.92.0**. GitHub checks pin that
same compiler instead of following changing stable releases. A compiler upgrade
is deferred to preparation for the first non-alpha release. The minimum Rust
requirement remains 1.92; installed prebuilt interpreters do not require Rust.
Native compilation and inline Rust still require the local Rust compiler.

Seeded RNG is the next stage, followed by bootstrap and covariance sampling.
Historical Simplified Chinese documentation and community-review proposals
are retained byte-for-byte; this new guide is English and not represented as
a reviewed Mandarin translation.
