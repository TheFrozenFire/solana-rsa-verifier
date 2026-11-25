//! On-chain RSA signature verification for Solana programs.
//!
//! Multiplication candidates share one shape so the spike can time them
//! against the same 2048-bit inputs. Limbs are little-endian `u64` words.
#![no_std]

pub mod modular;
pub mod mont;
pub mod schoolbook;

pub use schoolbook::{LIMBS, PRODUCT_LIMBS};

/// One timed operation. `id` is the spike instruction's first byte.
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
    Candidate {
        name: "cube",
        id: 2,
    },
    Candidate {
        name: "cios",
        id: 3,
    },
    Candidate {
        name: "karatsuba",
        id: 4,
    },
    Candidate {
        name: "square",
        id: 5,
    },
    Candidate {
        name: "wide",
        id: 6,
    },
    Candidate {
        name: "quot",
        id: 7,
    },
    Candidate {
        name: "limb32",
        id: 8,
    },
];

/// 2048-bit all-ones operands. The product is fixed, so each candidate is timed
/// on the same work.
pub const BENCH_A: [u64; LIMBS] = [u64::MAX; LIMBS];
pub const BENCH_B: [u64; LIMBS] = [u64::MAX; LIMBS];

/// Full-width base and modulus for `x^3 mod n`. `BENCH_N` is odd and its
/// top bit is set. `BENCH_X` keeps that top limb, so it is 2048 bits as well.
pub const BENCH_N: [u64; LIMBS] = bench_modulus();
pub const BENCH_X: [u64; LIMBS] = bench_base();
pub const BENCH_N0: u64 = mont::n_prime(BENCH_N[0]);

/// `2^4096 mod BENCH_N`.
pub const BENCH_RR: [u64; LIMBS] = [
    9080094901108508550,
    1473061867638795321,
    15318180814900901642,
    16154696778292119225,
    17537455368449395242,
    1644730675064421190,
    14376695302646295909,
    6170904032705790116,
    15609744607374706347,
    16426473061858625191,
    14481795701340567402,
    15917674933707548297,
    15178971050070957803,
    3261865218163396876,
    10204014066213191846,
    164910636092807123,
    10822884400084898774,
    12728186567416858392,
    2626220129079101011,
    5378031926482962867,
    4152132873373513487,
    16939457789885343731,
    12872186405925892223,
    16962235178237235961,
    5334813383414303154,
    17253112838718901013,
    12755711026206775927,
    17866575160127405471,
    18403508619166229035,
    8370687535180452901,
    2264716289698633902,
    8416699609164770616,
];

const fn bench_modulus() -> [u64; LIMBS] {
    let mut n = [0u64; LIMBS];
    let mut i = 0;
    while i < LIMBS {
        n[i] = 0xF0F0_F0F0_F0F0_F0F0 ^ ((i as u64) << 32) ^ (i as u64);
        i += 1;
    }
    n[0] |= 1;
    n[LIMBS - 1] |= 1 << 63;
    n
}

const fn bench_base() -> [u64; LIMBS] {
    let n = bench_modulus();
    let mut x = [0u64; LIMBS];
    let mut i = 0;
    while i < LIMBS {
        x[i] = 0x1234_5678_9ABC_DEF0 ^ n[i].rotate_left(7);
        i += 1;
    }
    x[LIMBS - 1] = n[LIMBS - 1];
    x[0] = n[0] - 1;
    x
}

pub fn noop(out: &mut [u64; PRODUCT_LIMBS], _a: &[u64; LIMBS], _b: &[u64; LIMBS]) {
    *out = [0; PRODUCT_LIMBS];
}

pub fn checksum(limbs: &[u64]) -> u64 {
    limbs
        .iter()
        .enumerate()
        .fold(0, |acc, (index, limb)| acc ^ limb.rotate_left(index as u32))
}
