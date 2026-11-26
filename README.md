# solana-rsa-verifier

RSA signature verification for Solana programs. The public exponent is 3. A modulus is 2048, 3072, or 4096 bits, and its top bit is set. The crate is `no_std` and has no dependencies, so a program can import it without pulling in another SDK.

Verification is a square and one extra multiply, modulo the public key. On sBPF v3 the 2048-bit product is an unrolled 32-bit multiply, and each remainder takes its quotient digits from `div64` and `mod64`.

## Quick start

```toml
[dependencies]
solana-rsa-verifier = { git = "https://github.com/TheFrozenFire/solana-rsa-verifier" }
```

```rust
use solana_rsa_verifier::verify;

verify(&modulus, &signature, &sha256)?;
```

`verify` is the 2048-bit PKCS#1 v1.5 SHA-256 entry point. `modulus` and `signature` are 256-byte big-endian integers. `sha256` is the 32-byte hash of the message. Hash with the `sol_sha256` syscall, then call `verify`.

`verify_pkcs1_sha256`, `verify_pkcs1_sha384`, and `verify_pkcs1_sha512` take a modulus and signature of 256, 384, or 512 bytes. `verify_pss` checks RSASSA-PSS with SHA-256, MGF1-SHA-256, a 32-byte salt, and trailer byte `0xbc`.

`pow3` is the 2048-bit exponentiation on little-endian `u64` limbs, for a caller that already holds the integers that way.

## Benchmark

Each row is a fixed base and an odd modulus of that width, timed between two `sol_remaining_compute_units` reads in the SVM. The 2048-bit exponent-3 row is `pow3`. The other rows use the same remainder and a 32-bit product that reads digits from the `u64` limbs in place. Exponent 65537 is sixteen modular squares and one multiply.

| Modulus | Exponent | Compute units | Median priority fee | 95th percentile |
| --- | ---: | ---: | ---: | ---: |
| 1024-bit | 3 | 84,181 | $0.000009 | $0.076 |
| 1024-bit | 65537 | 716,705 | $0.000080 | $0.65 |
| 2048-bit | 3 | 285,781 | $0.000032 | $0.26 |
| 2048-bit | 65537 | exceeds 1,400,000 | | |
| 3072-bit | 3 | 641,648 | $0.000072 | $0.58 |
| 3072-bit | 65537 | exceeds 1,400,000 | | |
| 4096-bit | 3 | 1,107,794 | $0.000124 | $1.00 |
| 4096-bit | 65537 | exceeds 1,400,000 | | |

A full check of a real exponent-3 signature over the message `solana-rsa-verifier`:

| Check | Compute units | Median priority fee | 95th percentile |
| --- | ---: | ---: | ---: |
| 2048-bit PKCS#1 v1.5 SHA-256 | 292,398 | $0.000033 | $0.26 |
| 2048-bit PSS | 362,032 | $0.000041 | $0.33 |
| 3072-bit PKCS#1 v1.5 SHA-256 | 651,455 | $0.000073 | $0.59 |
| 3072-bit PSS | 753,719 | $0.000084 | $0.68 |
| 4096-bit PKCS#1 v1.5 SHA-256 | 1,127,678 | $0.000126 | $1.02 |
| 4096-bit PSS | 1,260,417 | $0.000141 | $1.14 |

These are real signatures, so the remainder takes a different path from the patterned rows above. SHA-384 and SHA-512 compare a different digest after that same exponentiation. PSS also runs SHA-256 inside the crate, once for the salt block and once per MGF1 block. The caller's message hash is not included.

The dollar figures use 1,672 micro-lamports per compute unit at the median and 13,500,000 at the 95th percentile, with SOL near $67 on 2026-11-25. The per-signature fee is separate. A row that exceeds 1,400,000 compute units stops at that transaction limit, so it has no completed fee.

Reproduce the measurement:

```
cargo test-sbf -p spike --arch v3 -- --nocapture
```

`cargo test-sbf` keeps stderr, which is where the test prints the count.

## How it works

Exponent 3 is a square and one extra multiply. Each product unpacks 64-bit limbs into 32-bit digits so that one `mul64` is a full digit product, and the inner row is unrolled by four. sBPF v3 has no instruction that returns the high half of a 64-by-64 multiply, and a `u128` product compiles to a called helper. The 32-bit product avoids that call.

Each remainder is schoolbook division. The quotient digit is a 128-by-64 division built from `div64` and `mod64`, then multiplied by the modulus and subtracted. The modulus stays in 64-bit limbs. Splitting the remainder itself into 32-bit digits made the cube slower, because the extra digits cost more than the shorter multiply saved.

The 2048-bit product is in `crates/verifier/src/mul.rs` and the remainder is in `crates/verifier/src/pow.rs`. A 3072-bit or 4096-bit product reads each 32-bit digit from the limb in place, in `crates/verifier/src/bench.rs`, so the 4096-bit product fits in one stack frame.

## PKCS#1 v1.5

`verify` accepts a signature when `signature^3 mod modulus` is exactly

```
00 01 FF FF … FF 00 || DigestInfo || hash
```

The `FF` run is `k - 3 - len(DigestInfo) - len(hash)` bytes, and it is at least 8. For a 256-byte modulus that is 202 bytes for SHA-256, 186 for SHA-384, and 170 for SHA-512. `SHA256_DIGESTINFO`, `SHA384_DIGESTINFO`, and `SHA512_DIGESTINFO` are the 19-byte prefixes from RFC 8017, including the `NULL` parameter. A missing `NULL`, a short `FF` run, or any other byte is `Error::Padding`. The modulus is 256, 384, or 512 bytes, odd, and its top bit is set. The signature is nonzero and strictly less than the modulus.

## PSS

`verify_pss` accepts a signature when the representative is an EMSA-PSS encoding of the SHA-256 message hash. The top bit is zero. The last byte is `0xbc`. Unmasking the block before the hash with MGF1-SHA-256 yields `00 … 00 01` and a 32-byte salt. SHA-256 of eight zero bytes, the message hash, and that salt is the hash stored in the encoding.

## Security

The arithmetic is variable-time. That is appropriate for signature verification, where the inputs are public. It is not appropriate for a secret exponent.

The exponent is 3. The modulus is 2048, 3072, or 4096 bits. The function the caller picks fixes the encoding and the hash. Exponent 3 is safe to verify when the encoded message is checked as strictly as above. A product of individually valid signatures is not a proof that each one is valid, and a random combination of them costs more modular multiplications than verifying each signature on its own.

## Tests

```
cargo test -p solana-rsa-verifier
cargo test-sbf -p spike --arch v3 -- --nocapture
```

`pow3` is checked against a bit-by-bit remainder. `verify`, the other PKCS#1 v1.5 hashes, and `verify_pss` are checked against exponent-3 signatures over the message `solana-rsa-verifier` at 2048, 3072, and 4096 bits, including a wrong hash, a short `FF` run, a zero-length PSS salt, an even modulus, a modulus whose top bit is clear, and a signature that is not below the modulus.
