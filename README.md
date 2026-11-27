# solana-rsa-verifier

RSA signature verification for Solana programs. The crate is `no_std` and has no dependencies, so a program can import it without pulling in another SDK.

## Quick start

```toml
[dependencies]
solana-rsa-verifier = { git = "https://github.com/TheFrozenFire/solana-rsa-verifier" }
```

```rust
use solana_rsa_verifier::verify;

verify(&modulus, &signature, &sha256)?;
```

`verify` checks a 2048-bit PKCS#1 v1.5 SHA-256 signature. The public exponent is 3. `modulus` and `signature` are 256-byte big-endian integers. `sha256` is the 32-byte hash of the message. Hash with the `sol_sha256` syscall, then call `verify`.

`verify_pkcs1_sha256`, `verify_pkcs1_sha384`, and `verify_pkcs1_sha512` take a 256-, 384-, or 512-byte modulus and signature. `verify_pss` checks RSASSA-PSS with SHA-256, MGF1-SHA-256, and a 32-byte salt. `pow3` is the 2048-bit exponentiation on little-endian `u64` limbs.

## Benchmark

| Modulus | Exponent | Compute units | Median priority fee | 95th percentile |
| --- | ---: | ---: | ---: | ---: |
| 1024-bit | 3 | 84,181 | $0.000009 | $0.076 |
| 1024-bit | 65537 | 716,705 | $0.000080 | $0.65 |
| 2048-bit | 3 | 285,781 | $0.000032 | $0.26 |
| 2048-bit | 65537 | 2,553,580 | $0.000286 | $2.31 |
| 3072-bit | 3 | 641,648 | $0.000072 | $0.58 |
| 3072-bit | 65537 | 5,444,120 | $0.000610 | $4.92 |
| 4096-bit | 3 | 1,107,794 | $0.000124 | $1.00 |
| 4096-bit | 65537 | 9,418,740 | $0.001055 | $8.52 |

A full signature check with public exponent 3:

| Check | Compute units | Median priority fee | 95th percentile |
| --- | ---: | ---: | ---: |
| 2048-bit PKCS#1 v1.5 SHA-256 | 292,398 | $0.000033 | $0.26 |
| 2048-bit PSS | 362,032 | $0.000041 | $0.33 |
| 3072-bit PKCS#1 v1.5 SHA-256 | 651,455 | $0.000073 | $0.59 |
| 3072-bit PSS | 753,719 | $0.000084 | $0.68 |
| 4096-bit PKCS#1 v1.5 SHA-256 | 1,127,678 | $0.000126 | $1.02 |
| 4096-bit PSS | 1,260,417 | $0.000141 | $1.14 |

Dollar figures use 1,672 micro-lamports per compute unit at the median and 13,500,000 at the 95th percentile, with SOL near $67 on 2026-11-25. The signature fee is separate. Exponent 65537 at 2048 bits and wider is past the 1,400,000 compute-unit transaction limit. Those rows were measured with a higher budget in the test VM.

```
cargo test-sbf -p spike --arch v3 -- --nocapture
```

## How it works

The product uses 32-bit digits, so one `mul64` is a full digit product, and the inner row is unrolled by four. The remainder is schoolbook division. Each quotient digit is a 128-by-64 division built from `div64` and `mod64`.

The 2048-bit product is in `crates/verifier/src/mul.rs` and the remainder is in `crates/verifier/src/pow.rs`. Wider products are in `crates/verifier/src/bench.rs`.

## PKCS#1 v1.5

A signature is accepted when `signature^3 mod modulus` is

```
00 01 FF FF … FF 00 || DigestInfo || hash
```

The `FF` run is at least 8 bytes. `SHA256_DIGESTINFO`, `SHA384_DIGESTINFO`, and `SHA512_DIGESTINFO` are the prefixes from RFC 8017, including the `NULL` parameter. Anything else is `Error::Padding`. The modulus is odd and its top bit is set. The signature is nonzero and strictly less than the modulus.

## PSS

`verify_pss` accepts a signature when the representative is an EMSA-PSS encoding of the SHA-256 message hash. The top bit is clear, the last byte is `0xbc`, and MGF1-SHA-256 recovers a 32-byte salt.

## Security

The arithmetic is variable-time and is for public signature inputs. Exponent 3 is safe to verify when the encoded message is checked as strictly as above. A product of valid signatures is not a proof that each one is valid.

## Tests

```
cargo test -p solana-rsa-verifier
cargo test-sbf -p spike --arch v3 -- --nocapture
```
