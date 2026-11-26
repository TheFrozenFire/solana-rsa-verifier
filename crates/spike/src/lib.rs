//! Times RSA exponentiation inside the Solana VM.
//!
//! A two-byte instruction is a size code (0 = 1024 bits, 1 = 2048, 2 = 3072,
//! 3 = 4096) and an exponent code (0 = 3, 1 = 65537). Return data is the
//! charged compute units and the checksum of the result.
//!
//! A longer instruction times a full signature check. Its first byte is
//! `0x10` for PKCS#1 v1.5 SHA-256 or `0x11` for PSS, followed by the modulus,
//! the signature, and the 32-byte message hash.
#![no_std]

use solana_program::{
    account_info::AccountInfo, compute_units::sol_remaining_compute_units, entrypoint,
    entrypoint::ProgramResult, program::set_return_data, program_error::ProgramError,
    pubkey::Pubkey,
};
use solana_rsa_verifier::bench::{self, checksum};
use solana_rsa_verifier::{pow3, verify_pkcs1_sha256, verify_pss};

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Some(result) = time_encoding(instruction_data) {
        return result;
    }
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

/// Instruction layout: one opcode (`0x10` PKCS#1 SHA-256, `0x11` PSS), then
/// the modulus, the signature, and the 32-byte SHA-256. The modulus and the
/// signature are the same length, 256, 384, or 512 bytes.
fn time_encoding(instruction_data: &[u8]) -> Option<ProgramResult> {
    let (opcode, modulus, signature, digest) = split_encoding(instruction_data)?;
    let (charged, ok) = match opcode {
        0x10 => time_pkcs1(modulus, signature, digest),
        0x11 => time_pss(modulus, signature, digest),
        _ => return Some(Err(ProgramError::InvalidInstructionData)),
    };
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&charged.to_le_bytes());
    data[8..].copy_from_slice(&ok.to_le_bytes());
    set_return_data(&data);
    Some(Ok(()))
}

fn split_encoding(data: &[u8]) -> Option<(u8, &[u8], &[u8], &[u8; 32])> {
    if data.len() < 33 {
        return None;
    }
    let digest: &[u8; 32] = data[data.len() - 32..].try_into().ok()?;
    let body = &data[1..data.len() - 32];
    if body.len() % 2 != 0 {
        return None;
    }
    let half = body.len() / 2;
    if !matches!(half, 256 | 384 | 512) {
        return None;
    }
    Some((data[0], &body[..half], &body[half..], digest))
}

fn time_pkcs1(modulus: &[u8], signature: &[u8], digest: &[u8; 32]) -> (u64, u64) {
    let before = sol_remaining_compute_units();
    let result = verify_pkcs1_sha256(modulus, signature, digest);
    core::hint::black_box(&result);
    let after = sol_remaining_compute_units();
    (before.saturating_sub(after), u64::from(result.is_ok()))
}

fn time_pss(modulus: &[u8], signature: &[u8], digest: &[u8; 32]) -> (u64, u64) {
    let before = sol_remaining_compute_units();
    let result = verify_pss(modulus, signature, digest);
    core::hint::black_box(&result);
    let after = sol_remaining_compute_units();
    (before.saturating_sub(after), u64::from(result.is_ok()))
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
