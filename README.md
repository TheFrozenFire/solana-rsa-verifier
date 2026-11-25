# solana-rsa-verifier

A Solana library for RSA signature verification.

`verifier` (`crates/verifier`) is `no_std` and has no dependencies. It holds the arithmetic candidates. `spike` (`crates/spike`) is a Solana program that times one candidate and returns the compute units it was charged. Dependencies are Solana crates only.

The first candidate is schoolbook multiplication of two 2048-bit all-ones integers, 32 little-endian `u64` limbs. `noop` is the same call shape with an empty product, so its cost is the measurement itself. `cube` is `x^3 mod n` for a fixed 2048-bit `x` and odd 2048-bit `n`: two schoolbook products, each reduced by schoolbook division.

```
cargo test -p verifier
cargo test-sbf -p spike --arch v3 -- --nocapture
```

The spike test writes compute units for each candidate to stderr. `cargo test-sbf` keeps stderr and drops the test harness stdout. A new candidate gets an id in `verifier::CANDIDATES` and a match arm in `spike`'s `process_instruction`.
