# PCG reference attribution

PCG32 XSH-RR setseq arithmetic and initialization in `src/random.rs` are
adapted from the PCG minimal C reference by M. E. O'Neill.

Copyright 2014 M. E. O'Neill <oneill@pcg-random.org>.
Upstream: https://www.pcg-random.org/download.html and
https://www.pcg-random.org/using-pcg-c-basic.html.

The upstream minimal implementation is offered under Apache License 2.0;
its license is preserved as LICENSE-APACHE in this directory. Goblin++'s
Rust adaptation adds checked explicit seeds/streams, specified uniform and
integer mappings, bounded transactional operations and replayable receipts.
It is scientific randomness, not a cryptographic generator. No PCG crate
or additional Cargo dependency is bundled.
