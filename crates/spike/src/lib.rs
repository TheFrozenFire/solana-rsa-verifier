//! Times one multiplication candidate inside the Solana VM.
//!
//! Instruction data is a single byte, the candidate id from `verifier::CANDIDATES`.
//! Return data is two little-endian `u64`s: compute units charged between the
//! two remaining-compute reads, then the xor of the product limbs.
#![no_std]

use solana_program::{
    account_info::AccountInfo, compute_units::sol_remaining_compute_units, entrypoint,
    entrypoint::ProgramResult, program::set_return_data, program_error::ProgramError,
    pubkey::Pubkey,
};
use verifier::{
    BENCH_A, BENCH_B, BENCH_N, BENCH_N0, BENCH_RR, BENCH_X, LIMBS, PRODUCT_LIMBS, modular, mont,
    noop, schoolbook,
};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let id = instruction_data
        .first()
        .copied()
        .ok_or(ProgramError::InvalidInstructionData)?;
    let (charged, digest) = match id {
        0 => time(|out| schoolbook::mul(out, &BENCH_A, &BENCH_B)),
        1 => time(|out| noop(out, &BENCH_A, &BENCH_B)),
        2 => time_cube(modular::cube),
        3 => time_cube(|out, x, n| mont::cube_cios(out, x, n, &BENCH_RR, BENCH_N0)),
        4 => time_cube(|out, x, n| mont::cube_karatsuba(out, x, n, &BENCH_RR, BENCH_N0)),
        5 => time_cube(|out, x, n| mont::cube_square(out, x, n, &BENCH_RR, BENCH_N0)),
        6 => time(|out| schoolbook::mul_wide(out, &BENCH_A, &BENCH_B)),
        7 => time_cube(modular::cube_quot),
        8 => time(|out| schoolbook::mul32(out, &BENCH_A, &BENCH_B)),
        9 => time(|out| schoolbook::mul32_mac(out, &BENCH_A, &BENCH_B)),
        10 => time_cube(modular::cube_rem32),
        _ => return Err(ProgramError::InvalidInstructionData),
    };
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&charged.to_le_bytes());
    data[8..].copy_from_slice(&digest.to_le_bytes());
    set_return_data(&data);
    Ok(())
}

fn time_cube(op: impl FnOnce(&mut [u64; LIMBS], &[u64; LIMBS], &[u64; LIMBS])) -> (u64, u64) {
    time(|out| {
        let mut reduced = [0u64; LIMBS];
        op(&mut reduced, &BENCH_X, &BENCH_N);
        out[..LIMBS].copy_from_slice(&reduced);
    })
}

fn time(op: impl FnOnce(&mut [u64; PRODUCT_LIMBS])) -> (u64, u64) {
    let mut out = [0u64; PRODUCT_LIMBS];
    let before = sol_remaining_compute_units();
    op(&mut out);
    core::hint::black_box(&out);
    let after = sol_remaining_compute_units();
    let digest = verifier::checksum(&out);
    (before.saturating_sub(after), digest)
}
