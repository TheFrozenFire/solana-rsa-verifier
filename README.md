# solana-rsa-verifier

RSA-2048 signature verification for Solana programs. The public exponent is 3. The crate is `no_std` and has no dependencies, so a program can import it without pulling in another SDK.

Verification is two 2048-bit multiplications and two remainders. On sBPF v3 that is the fastest shape measured in this repository: an unrolled 32-bit product, and a remainder whose quotient digits come from `div64` and `mod64`.

## Quick start

```toml
[dependencies]
solana-rsa-verifier = { git = "https://github.com/TheFrozenFire/solana-rsa-verifier" }
```

```rust
use solana_rsa_verifier::verify;

verify(&modulus, &signature, &sha256)?;
```

`modulus` and `signature` are 256-byte big-endian integers. `sha256` is the 32-byte hash of the message. Hash with the `sol_sha256` syscall, then call `verify`.

`pow3` is the same exponentiation on little-endian `u64` limbs, for a caller that already holds the integers that way.

## Benchmark

`pow3` of a fixed 2048-bit base and an odd 2048-bit modulus, timed between two `sol_remaining_compute_units` reads in the SVM:

| Operation | Compute units | Median priority fee | 95th percentile |
| --- | ---: | ---: | ---: |
| `pow3` | 285,781 | $0.000032 | $0.26 |

The dollar figures use 1,672 micro-lamports per compute unit at the median and 13,500,000 at the 95th percentile, with SOL near $67 on 2026-11-25. The per-signature fee is separate. A full `verify` adds the hash syscall and a comparison of the encoded message. Those are small next to `pow3`.

Reproduce the measurement:

```
cargo test-sbf -p spike --arch v3 -- --nocapture
```

`cargo test-sbf` keeps stderr, which is where the test prints the count.

## How it works

Exponent 3 is a square and one extra multiply. Each product unpacks 64-bit limbs into 32-bit digits so that one `mul64` is a full digit product, and the inner row is unrolled by four. sBPF v3 has no instruction that returns the high half of a 64-by-64 multiply, and a `u128` product compiles to a called helper. The 32-bit product avoids that call.

Each remainder is schoolbook division. The quotient digit is a 128-by-64 division built from `div64` and `mod64`, then multiplied by the modulus and subtracted. The modulus stays in 64-bit limbs. Splitting the remainder itself into 32-bit digits made the cube slower, because the extra digits cost more than the shorter multiply saved.

The implementation is in `crates/verifier/src/mul.rs` and `crates/verifier/src/pow.rs`.

## PKCS#1 v1.5

`verify` accepts a signature when `signature^3 mod modulus` is exactly

```
00 01 FF FF … FF 00 || SHA-256 DigestInfo || hash
```

for a 256-byte modulus. The `FF` run is 202 bytes. `SHA256_DIGESTINFO` is the 19-byte prefix from RFC 8017. Anything else, including a short pad or a trailing byte, is `Error::Padding`. The signature must be nonzero and strictly less than the modulus.

## Security

The arithmetic is variable-time. That is appropriate for signature verification, where the inputs are public. It is not appropriate for a secret exponent.

The exponent is fixed at 3, and the modulus and signature are fixed at 2048 bits. PSS is not implemented. Exponent 3 is safe to verify only when the encoded message is checked as strictly as above. A product of individually valid signatures is not a proof that each one is valid, and a random combination of them costs more modular multiplications than verifying each signature on its own.

## Tests

```
cargo test -p solana-rsa-verifier
cargo test-sbf -p spike --arch v3 -- --nocapture
```

`pow3` is checked against a bit-by-bit remainder. `verify` is checked against an RSA-2048 signature with exponent 3 over the message `solana-rsa-verifier`, including a wrong hash, an even modulus, and a signature that is not below the modulus.
