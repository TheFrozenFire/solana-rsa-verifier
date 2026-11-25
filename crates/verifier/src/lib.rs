//! RSA-2048 signature verification for Solana programs.
//!
//! The public exponent is 3. A signature is a 256-byte integer, and the
//! modulus is a 256-byte integer. Both are big-endian, as in PKCS#1.
//! [`pow3`] takes the same integers as little-endian `u64` limbs.
#![no_std]

mod mul;
mod pow;

pub const LIMBS: usize = 32;
pub const MODULUS_BYTES: usize = 256;

/// SHA-256 DigestInfo prefix from RFC 8017.
pub const SHA256_DIGESTINFO: [u8; 19] = [
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The modulus is even or zero.
    Modulus,
    /// The signature is zero or is not strictly less than the modulus.
    Representative,
    /// The encoded message is not PKCS#1 v1.5 with SHA-256.
    Padding,
}

/// `base.pow(3) % modulus`.
///
/// Limbs are little-endian. `modulus` is nonzero. A zero modulus yields zero.
pub fn pow3(base: &[u64; LIMBS], modulus: &[u64; LIMBS]) -> [u64; LIMBS] {
    pow::pow3(base, modulus)
}

/// Verify a PKCS#1 v1.5 SHA-256 signature with public exponent 3.
///
/// `sha256` is the 32-byte hash of the message. On Solana, hash with the
/// `sol_sha256` syscall.
pub fn verify(
    modulus: &[u8; MODULUS_BYTES],
    signature: &[u8; MODULUS_BYTES],
    sha256: &[u8; 32],
) -> Result<(), Error> {
    if modulus[255] & 1 == 0 {
        return Err(Error::Modulus);
    }
    if signature == &[0; MODULUS_BYTES] || signature >= modulus {
        return Err(Error::Representative);
    }
    let encoded = pow3(&from_be_bytes(signature), &from_be_bytes(modulus));
    if pkcs1_sha256(&to_be_bytes(&encoded), sha256) {
        Ok(())
    } else {
        Err(Error::Padding)
    }
}

fn pkcs1_sha256(encoded: &[u8; MODULUS_BYTES], sha256: &[u8; 32]) -> bool {
    encoded[0] == 0x00
        && encoded[1] == 0x01
        && encoded[2..204].iter().all(|byte| *byte == 0xff)
        && encoded[204] == 0x00
        && encoded[205..224] == SHA256_DIGESTINFO
        && encoded[224..] == *sha256
}

pub fn from_be_bytes(bytes: &[u8; MODULUS_BYTES]) -> [u64; LIMBS] {
    let mut limbs = [0u64; LIMBS];
    for i in 0..LIMBS {
        let start = MODULUS_BYTES - (i + 1) * 8;
        let mut word = [0u8; 8];
        word.copy_from_slice(&bytes[start..start + 8]);
        limbs[i] = u64::from_be_bytes(word);
    }
    limbs
}

pub fn to_be_bytes(limbs: &[u64; LIMBS]) -> [u8; MODULUS_BYTES] {
    let mut bytes = [0u8; MODULUS_BYTES];
    for i in 0..LIMBS {
        let start = MODULUS_BYTES - (i + 1) * 8;
        bytes[start..start + 8].copy_from_slice(&limbs[i].to_be_bytes());
    }
    bytes
}
