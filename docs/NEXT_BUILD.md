# Next-build notes

## Current sequence — 2026-10-06

Alpha.25 compound-unit input is implemented; the original report below is
historical, not current behavior. Alpha.26 supplies the explicit scientific
PCG32 RNG contract; see [RANDOMNESS.md](RANDOMNESS.md). Next: declared bootstrap/
resampling contracts, then matrix/covariance validation and normal sampling.
Do not silently pick interval assumptions, seed defaults or normal transforms.

Future concept only: AES-256-GCM file protection and a separate
`COMPLETELY_MAD` strict policy profile. User-proposed commands:
`goblin++ --enc <FILENAME> --set_key <PASSWORD>` and
`goblin++ --dec <FILENAME> --key <PASSWORD>`.
These flags are not implemented. Password arguments risk shell-history/process
exposure; review hidden prompts/key handling before implementation. Scientific
PCG32 must never supply cryptographic keys/nonces. A custom IDE/workbench is
dropped to avoid upkeep; use existing IDEs/plugins.

The full prioritised work list is in
[DEVELOPMENT_BACKLOG.md](DEVELOPMENT_BACKLOG.md). This existing compound-unit
note corresponds to **GBL-013**; its detailed acceptance gates remain below.

## Compound unit input/output consistency

Reported by Malin Hess on 2026-10-02; reproduced with the local alpha.19
release executable. This is a planned fix, not an implemented feature.

```goblin
acceleration = 1.13e-10 m/s^2
```

Currently fails with `G101 UNKNOWN SYMBOL s`. The numeric-literal parser
consumes one unit spelling (`m`); `/s^2` is then ordinary expression syntax,
and bare `s` is evaluated as a name rather than as a unit factor.

The supported workaround is:

```goblin
acceleration = 1.13e-10 m / (1 s)^2
```

This passes and renders as `1.13e-10 m/s^2`. The formatter's compound-unit
notation is therefore not currently accepted as equivalent input.

### Proposed scope and acceptance gates

- Define compound-unit syntax and precedence explicitly before implementing
  it. Accept the reported acceleration form with the same value and dimension
  as the workaround; cover representative printed forms such as `kg*m^2/s`
  and `s^-1`.
- Do not silently reinterpret existing expressions, unit powers, constant
  names, variable division, or scientific-notation numbers. In particular,
  distinguish exponentiation of a whole quantity from a unit exponent.
- Test correct scale-factor application and rejection of unknown units,
  malformed unit expressions, and unsupported exponents. Dimensional checks
  must remain strict.
- Exercise interpreter/compiler parity, dimensional arithmetic, rendering,
  and preserved-run verification. Specify canonicalization deliberately;
  regression-test existing canonical hashes and frozen-source enforcement.
- Update the language reference, examples, and both editor plugins if their
  syntax handling needs to change.

Do not modify the published alpha.19 tag or assets for this fix.
