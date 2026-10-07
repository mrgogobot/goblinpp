# Seeded scientific randomness — alpha.26

Alpha.27 retains this exact primitive policy and adds explicit-stream normal
and IID bootstrap helpers: see [RESAMPLING.md](RESAMPLING.md) and
[MATRICES_COVARIANCE.md](MATRICES_COVARIANCE.md). Loop-budget and streaming
extensions are specified separately in [RESOURCES_STREAMING.md](RESOURCES_STREAMING.md).
Primitive replay does not certify cross-platform equality of transformed normals.

Goblin++ provides a reproducible **scientific**, not cryptographic, generator
in the interpreter and native compiler. Seed explicitly; there is no clock,
operating-system entropy or hidden default seed. Rust stays at 1.92.0.

## Day-one example

```goblin
GO_PARANOID
rng_seed(42)
sample = rng_uniform()        # 0 <= sample < 1
index = rng_integer(0, 10)    # one of 0 ... 9
word = rng_word()             # an exact unsigned 32-bit value
print("sample = {sample}; index = {index}; word = {word}")
seal sample
seal index
seal word
```

Run `goblin++ examples/seeded_random.gbl`, or add `--compile`.
`GO_PARANOID` and `seal` remain optional: RNG policy/evidence is preserved
even in everyday mode. Draw results must be consumed, for example by assignment.
`rng_uniform()` as a statement refuses instead of silently discarding a draw.

| Function | Meaning |
| --- | --- |
| `rng_seed(seed [, stream])` | Initialize once; returns `true`; may be a statement |
| `rng_word([stream])` | One unsigned 32-bit word, exactly representable numerically |
| `rng_uniform([stream])` | Dimensionless uniform on half-open [0, 1) |
| `rng_integer(low, high [, stream])` | Uniform numeric integer on half-open [low, high) |

Stream defaults to **0**, not to an automatically selected stream. An unseeded
draw, repeated seeding of the same stream, fractional/unit-bearing seed or
invalid bounds refuses with G204. Use another explicit stream ID or a new run.

Seeds span unsigned 64 bits; streams span unsigned 63 bits. Numeric arguments
must be dimensionless exact integers from 0 through 2^53−1. For larger values,
use canonical unsigned decimal text: `rng_seed("18446744073709551615")`.
No plus sign, whitespace or leading zeroes (except `"0"`) are accepted.
Integer bounds are numeric dimensionless integers within ±(2^53−1), with
`low < high`. This does **not** introduce a general 64-bit integer value type.

## Explicit streams and reusable functions

```goblin
rng_seed(42, 1)
rng_seed(42, 2)
first = rng_uniform(1)
other = rng_uniform(2)
next = rng_uniform(1)
g_func draw_from(stream) { return rng_uniform(stream) }
third = draw_from(1)
```

RNG state belongs to the run, not a copied array or function-local environment.
Activity on stream 2 does not advance stream 1. Use stable, explicitly assigned
stream IDs in reproducible workflows. Identical seed/stream and identical
per-stream call sequence reproduce identical outputs. Reordering calls *within*
a stream changes its outputs. Distinct streams do not constitute a proof of
statistical independence, and this is not a parallel scheduler.

## Versioned algorithm and mappings

Policy ID: **`goblin.pcg32-xsh-rr-setseq.v1`**.
Implementation: `goblin-pcg32-v1`; PCG32 XSH-RR 64-bit state / 32-bit output,
setseq variant, following M. E. O'Neill's
[PCG minimal C reference](https://www.pcg-random.org/download.html).
Preserved attribution/license: `third-party/licenses/pcg/`.

All state arithmetic wraps modulo 2^64. Multiplier = 6364136223846793005;
increment = (stream << 1) | 1. For old state s, advance with the multiplier
and increment, and emit rotate-right32(((s >> 18) XOR s) >> 27, s >> 59).
Initialization: state = 0; advance once; add seed modulo 2^64; advance once.
Initialization advances do not count as returned/raw draw words.

Uniform mapping consumes **two** successive words a, b:

```text
n = ((a >> 5) << 26) | (b >> 6)
result = n / 2^53
```

Integer mapping consumes two words as u = (a << 32) | b. For width =
high − low, threshold = (−width modulo 2^64) modulo width. Reject u below
threshold; otherwise return low + u modulo width. This avoids modulo bias.
At most 64 candidates are attempted per operation; failure is transactional.
Changing mappings, seeding or variants requires a **new policy version**.

Published reference seed 42, stream 54 gives the first six words:
`a15c02b7 7b47f409 ba1d3330 83d2f293 bfa4784b cbed606e`.
Tests use that external vector rather than deriving their expected values
from Goblin++ itself. No normal-distribution transform is supplied yet.

## Receipts, freezing and independent verification

New run receipts include `rng_policy` and `rng` evidence, even with no RNG:
`NOT_USED`, `INITIALIZED_NO_DRAWS` or `USED`. Each seeded stream records
canonical seed/ID, operation/raw-word counts, final state, SHA-256 digests,
and bounded run-length-encoded operation segments (including integer bounds).
Seeds are public provenance, **not secrets**.

Raw words hash unsigned u32 little-endian bytes. Returned words hash u32LE;
uniforms hash exact IEEE-754 f64 bits in little-endian order; integer results
hash signed i64LE. Initialization is excluded. Streams are serialized by
numeric ID; segments preserve each stream's operation order. Cross-stream
interleaving is deliberately not part of the stream-output digest.

`goblin++ verify RUN_DIR` independently replays the bounded trace and verifies
counts, final states and both digests, without executing the program or
reopening its datasets. Re-hashing an altered receipt alone does not fix
inconsistent RNG evidence. Verification establishes checksum/replay consistency,
not authenticated authorship, honest seed selection or a correct scientific
model. A fully rewritten consistent history still needs external anchoring.

New freezes pin the RNG policy. Policy drift refuses execution/compilation
and preserves protocol-violation evidence when running. Pre-alpha.26 freezes
without RNG policy retain their existing non-RNG behavior; other parser,
math and semantics guards still apply. Old receipt bytes are not modified.
`diff` compares RNG policy and evidence separately; equal seals alone cannot
hide different seeds/draw histories.

Compiled programs use the same reviewed RNG implementation via the existing
native data-support crate. This requires Cargo and cached locked dependencies,
like compiled FITS/I/O, rather than the rustc-only math compilation route.
Native compilation rejects invalid RNG arity statically, including unreachable
calls; argument evaluation stops at the first error.
Launched compiled runs compare native RNG evidence against the interpreter
reference. Standalone successful binaries preserve RNG evidence in their
native-data manifest, but are not full launcher run receipts.

## Limits and scientific scope

Per run: at most 128 streams, 1,000,000 draw operations, 8,000,000 raw words and
4096 operation segments across streams. Consecutive identical operations/bounds
coalesce. Long alternating operation kinds/bounds can reach the segment cap early.
Stable separate streams for different workflow phases can avoid that
alternation, but stream assignment is an explicit experimental choice.
The word guard conservatively reserves 128 words before each draw, so the
effective cap may be lower than 8,000,000. Existing loop/array limits
remain; configurable/streaming workloads are a separate stage.

Raw PCG words, integer mappings and 53-bit uniforms use specified arithmetic.
CI measures exact macOS/Linux and interpreter/native agreement for a fixed
fixture; it is not a universal bitwise guarantee for transcendental mathematics.
Keep exact authoritative seals and compare scientific quantities separately
with declared tolerances.

Bootstrap, covariance sampling and distribution validation follow this stage.
Do not use these functions for keys, passwords, AES-GCM nonces or any other
security randomness. Encryption and a proposed `COMPLETELY_MAD` policy profile
remain separate future features, not alpha.26 directives.

Spread the word with [Goblin Merch](https://h4k3rl1f3.myspreadshop.co.uk).
