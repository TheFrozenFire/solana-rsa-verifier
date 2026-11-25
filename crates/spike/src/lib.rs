//! Times RSA exponentiation inside the Solana VM.
//!
//! Instruction data is two bytes: a size code (0 = 1024 bits, 1 = 2048,
//! 2 = 3072, 3 = 4096) and an exponent code (0 = 3, 1 = 65537). Return data
//! is the charged compute units and the checksum of the result.
#![no_std]

use solana_program::{
    account_info::AccountInfo, compute_units::sol_remaining_compute_units, entrypoint,
    entrypoint::ProgramResult, program::set_return_data, program_error::ProgramError,
    pubkey::Pubkey,
};
use solana_rsa_verifier::bench::{self, checksum};
use solana_rsa_verifier::pow3;

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let &[size, exponent] = instruction_data else {
        return Err(ProgramError::InvalidInstructionData);
    };
    let (charged, digest) = match (size, exponent) {
        (0, 0) => time_1024(3),
        (0, 1) => time_1024(65537),
        (1, 0) => time_pow3(),
        (1, 1) => time_2048(65537),
        (2, 0) => time_3072(3),
        (2, 1) => time_3072(65537),
        (3, 0) => time_4096(3),
        (3, 1) => time_4096(65537),
        _ => return Err(ProgramError::InvalidInstructionData),
    };
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&charged.to_le_bytes());
    data[8..].copy_from_slice(&digest.to_le_bytes());
    set_return_data(&data);
    Ok(())
}

fn time_pow3() -> (u64, u64) {
    let (base, modulus) = bench::bench_2048();
    let before = sol_remaining_compute_units();
    let out = pow3(&base, &modulus);
    core::hint::black_box(&out);
    let after = sol_remaining_compute_units();
    (before.saturating_sub(after), checksum(&out))
}

macro_rules! time_width {
    ($name:ident, $limbs:literal, $exp:path) => {
        fn $name(exponent: u32) -> (u64, u64) {
            let base = bench::base::<$limbs>();
            let modulus = bench::modulus::<$limbs>();
            let before = sol_remaining_compute_units();
            let out = $exp(&base, &modulus, exponent);
            core::hint::black_box(&out);
            let after = sol_remaining_compute_units();
            (before.saturating_sub(after), checksum(&out))
        }
    };
}

time_width!(time_1024, 16, bench::modexp_1024);
time_width!(time_2048, 32, bench::modexp_2048);
time_width!(time_3072, 48, bench::modexp_3072);
time_width!(time_4096, 64, bench::modexp_4096);
