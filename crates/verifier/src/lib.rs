//! On-chain RSA signature verification for Solana programs.
//!
//! Multiplication candidates share one shape so the spike can time them
//! against the same 2048-bit inputs. Limbs are little-endian `u64` words.
#![no_std]

pub mod schoolbook;

pub use schoolbook::{LIMBS, PRODUCT_LIMBS};

/// One timed multiplication. `id` is the spike instruction's first byte.
pub struct Candidate {
    pub name: &'static str,
    pub id: u8,
}

pub const CANDIDATES: &[Candidate] = &[
    Candidate {
        name: "schoolbook",
        id: 0,
    },
    Candidate {
        name: "noop",
        id: 1,
    },
];

/// 2048-bit all-ones operands. The product is fixed, so each candidate is timed
/// on the same work.
pub const BENCH_A: [u64; LIMBS] = [u64::MAX; LIMBS];
pub const BENCH_B: [u64; LIMBS] = [u64::MAX; LIMBS];

pub fn noop(out: &mut [u64; PRODUCT_LIMBS], _a: &[u64; LIMBS], _b: &[u64; LIMBS]) {
    *out = [0; PRODUCT_LIMBS];
}

pub fn checksum(limbs: &[u64]) -> u64 {
    limbs
        .iter()
        .enumerate()
        .fold(0, |acc, (index, limb)| acc ^ limb.rotate_left(index as u32))
}
