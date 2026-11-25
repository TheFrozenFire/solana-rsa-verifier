//! Times [`solana_rsa_verifier::pow3`] inside the Solana VM.
//!
//! Return data is two little-endian `u64`s: compute units charged for the
//! exponentiation, then the checksum of the limbs.
#![no_std]

use solana_program::{
    account_info::AccountInfo, compute_units::sol_remaining_compute_units, entrypoint,
    entrypoint::ProgramResult, program::set_return_data, pubkey::Pubkey,
};
use solana_rsa_verifier::{LIMBS, pow3};

entrypoint!(process_instruction);

const BENCH_N: [u64; LIMBS] = bench_modulus();
const BENCH_X: [u64; LIMBS] = bench_base();

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let before = sol_remaining_compute_units();
    let out = pow3(&BENCH_X, &BENCH_N);
    core::hint::black_box(&out);
    let after = sol_remaining_compute_units();
    let digest = out
        .iter()
        .enumerate()
        .fold(0, |acc, (index, limb)| acc ^ limb.rotate_left(index as u32));
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&before.saturating_sub(after).to_le_bytes());
    data[8..].copy_from_slice(&digest.to_le_bytes());
    set_return_data(&data);
    Ok(())
}

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
