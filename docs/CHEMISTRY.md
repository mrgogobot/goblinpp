# Chemistry foundation

Goblin++ 0.1.0-alpha.16 adds a deliberately small chemistry layer for common
laboratory calculations. It is available in both the interpreter and native
compiler. The engine remains authoritative; editor suggestions are only
authoring aids.

## Registry and evidence

The element table is identified as `IUPAC-2021-ABRIDGED-COMMON-v1`. It contains
43 explicitly listed common elements and uses abridged standard atomic weights
from the [IUPAC periodic table](https://iupac.org/what-we-do/periodic-table-of-elements/).
The registry identity, contents, and SHA-256 are included in the scientific
registry and freeze evidence. A table change therefore cannot silently pass as
the same frozen scientific environment.

`chem_registry_version()` returns the exact registry identity.
`chem_atomic_number("C")` and `chem_atomic_weight("O")` require exact,
case-sensitive element symbols. The atomic weight is a dimensionless relative
value. It is not an isotope mass or a claim about a sample's isotopic
composition.

The molar gas constant `R` is derived from exact SI defining constants. The
atomic mass constant `m_u` and dalton unit `Da` use the CODATA-2022 value in
this release; their measured status is preserved in registry evidence. See the
[CODATA recommended values](https://codata.org/initiatives/data-science-and-stewardship/fundamental-physical-constants/).

## Formula and calculation helpers

```goblin
GO_PARANOID

water_molar_mass = chem_molar_mass("H2O")
water_amount = chem_moles(36.03 g, water_molar_mass)
recovered_mass = chem_mass(water_amount, water_molar_mass)

stock = chem_concentration(0.1 mol, 100 mL)
diluted = chem_dilution(stock, 10 mL, 100 mL)

print("H2O molar mass = {water_molar_mass}")
print("water amount = {water_amount}")
print("diluted concentration = {diluted}")

seal water_molar_mass
seal diluted
```

`chem_molar_mass` supports exact element symbols, positive integer counts, and
parenthesised groups, for example `H2O`, `C6H12O6`, and `Ca(OH)2`. Formulae are
bounded to 1,024 bytes, group nesting to eight levels, and each count to
1,000,000.

The dimension-checked arithmetic helpers are:

- `chem_moles(mass, molar_mass)`;
- `chem_mass(amount, molar_mass)`;
- `chem_concentration(amount, volume)`;
- `chem_dilution(concentration, initial_volume, final_volume)`, implementing
  `C2 = C1 V1 / V2` and refusing a final volume smaller than the initial one.

Useful units added with this stage are `L`, `mL`, `uL`, `µL`, `nm`, `pm`,
`angstrom`, `Å`, `Pa`, `kPa`, `bar`, `atm`, and `Da`.

## Intentional limits

This stage does **not** infer charge, isotope notation, square-bracket groups,
hydrate dots, reactions, stoichiometric balancing, pH, activity, equilibrium,
kinetics, thermodynamic state, uncertainty, or biological sequence meaning.
An unsupported formula fails instead of being guessed. Use sample-specific
atomic masses when isotopic composition matters.

The design principle is simple: useful lab arithmetic, visible assumptions,
and no synthetic chemistry oracle hiding behind a friendly function name.
