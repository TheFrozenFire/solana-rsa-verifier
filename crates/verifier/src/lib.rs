//! RSA signature verification for Solana programs.
//!
//! The public exponent is 3. A modulus is 256, 384, or 512 bytes (2048, 3072,
//! or 4096 bits) with its top bit set. Signatures are big-endian, as in
//! PKCS#1. [`pow3`] takes 2048-bit integers as little-endian `u64` limbs.
#![no_std]

#[doc(hidden)]
pub mod bench;
mod mul;
mod pkcs1;
mod pow;
mod pss;
mod sha256;

pub const LIMBS: usize = 32;
pub const MODULUS_BYTES: usize = 256;
pub const MODULUS_3072_BYTES: usize = 384;
pub const MODULUS_4096_BYTES: usize = 512;

/// SHA-256 DigestInfo prefix from RFC 8017, including the `NULL` parameter.
pub const SHA256_DIGESTINFO: [u8; 19] = [
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05,
    0x00, 0x04, 0x20,
];

/// SHA-384 DigestInfo prefix from RFC 8017, including the `NULL` parameter.
pub const SHA384_DIGESTINFO: [u8; 19] = [
    0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02, 0x05,
    0x00, 0x04, 0x30,
];

/// SHA-512 DigestInfo prefix from RFC 8017, including the `NULL` parameter.
pub const SHA512_DIGESTINFO: [u8; 19] = [
    0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03, 0x05,
    0x00, 0x04, 0x40,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The modulus is even, does not fill its width, or is not 2048, 3072, or 4096 bits.
    Modulus,
    /// The signature is zero, has a different length, or is not strictly less than the modulus.
    Representative,
    /// The encoded message is not the requested PKCS#1 v1.5 or PSS encoding.
    Padding,
}

/// `base.pow(3) % modulus`.
///
/// Limbs are little-endian. `modulus` is nonzero. A zero modulus yields zero.
pub fn pow3(base: &[u64; LIMBS], modulus: &[u64; LIMBS]) -> [u64; LIMBS] {
    pow::pow3(base, modulus)
}

/// Verify a 2048-bit PKCS#1 v1.5 SHA-256 signature with public exponent 3.
///
/// `sha256` is the 32-byte hash of the message. On Solana, hash with the
/// `sol_sha256` syscall.
pub fn verify(
    modulus: &[u8; MODULUS_BYTES],
    signature: &[u8; MODULUS_BYTES],
    sha256: &[u8; 32],
) -> Result<(), Error> {
    verify_pkcs1_sha256(modulus, signature, sha256)
}

/// Verify a PKCS#1 v1.5 SHA-256 signature with public exponent 3.
///
/// `modulus` and `signature` are 256, 384, or 512 bytes.
pub fn verify_pkcs1_sha256(
    modulus: &[u8],
    signature: &[u8],
    sha256: &[u8; 32],
) -> Result<(), Error> {
    verify_pkcs1(modulus, signature, pkcs1::sha256, sha256)
}

/// Verify a PKCS#1 v1.5 SHA-384 signature with public exponent 3.
///
/// `modulus` and `signature` are 256, 384, or 512 bytes.
pub fn verify_pkcs1_sha384(
    modulus: &[u8],
    signature: &[u8],
    sha384: &[u8; 48],
) -> Result<(), Error> {
    verify_pkcs1(modulus, signature, pkcs1::sha384, sha384)
}

/// Verify a PKCS#1 v1.5 SHA-512 signature with public exponent 3.
///
/// `modulus` and `signature` are 256, 384, or 512 bytes.
pub fn verify_pkcs1_sha512(
    modulus: &[u8],
    signature: &[u8],
    sha512: &[u8; 64],
) -> Result<(), Error> {
    verify_pkcs1(modulus, signature, pkcs1::sha512, sha512)
}

/// Verify an RSASSA-PSS SHA-256 signature with public exponent 3.
///
/// The salt is 32 bytes, the mask is MGF1 with SHA-256, and the trailer byte
/// is `0xbc`. `sha256` is the 32-byte hash of the message. `modulus` and
/// `signature` are 256, 384, or 512 bytes.
pub fn verify_pss(modulus: &[u8], signature: &[u8], sha256: &[u8; 32]) -> Result<(), Error> {
    let mut encoded = [0u8; MODULUS_4096_BYTES];
    let encoded = exponentiate(modulus, signature, &mut encoded)?;
    if pss::sha256_salt32(encoded, sha256) {
        Ok(())
    } else {
        Err(Error::Padding)
    }
}

fn verify_pkcs1<const N: usize>(
    modulus: &[u8],
    signature: &[u8],
    accept: fn(&[u8], &[u8; N]) -> bool,
    digest: &[u8; N],
) -> Result<(), Error> {
    let mut encoded = [0u8; MODULUS_4096_BYTES];
    let encoded = exponentiate(modulus, signature, &mut encoded)?;
    if accept(encoded, digest) {
        Ok(())
    } else {
        Err(Error::Padding)
    }
}

fn exponentiate<'a>(
    modulus: &[u8],
    signature: &[u8],
    encoded: &'a mut [u8; MODULUS_4096_BYTES],
) -> Result<&'a [u8], Error> {
    if !supported_modulus(modulus) {
        return Err(Error::Modulus);
    }
    if signature.len() != modulus.len()
        || signature.iter().all(|byte| *byte == 0)
        || signature >= modulus
    {
        return Err(Error::Representative);
    }
    let width = modulus.len();
    match width {
        MODULUS_BYTES => {
            let mut base = [0u64; LIMBS];
            let mut n = [0u64; LIMBS];
            be_to_limbs(signature, &mut base);
            be_to_limbs(modulus, &mut n);
            limbs_to_be(&pow::pow3(&base, &n), &mut encoded[..width]);
        }
        MODULUS_3072_BYTES => {
            let mut base = [0u64; 48];
            let mut n = [0u64; 48];
            be_to_limbs(signature, &mut base);
            be_to_limbs(modulus, &mut n);
            limbs_to_be(&bench::modexp_3072(&base, &n, 3), &mut encoded[..width]);
        }
        MODULUS_4096_BYTES => {
            let mut base = [0u64; 64];
            let mut n = [0u64; 64];
            be_to_limbs(signature, &mut base);
            be_to_limbs(modulus, &mut n);
            limbs_to_be(&bench::modexp_4096(&base, &n, 3), &mut encoded[..width]);
        }
        _ => return Err(Error::Modulus),
    }
    Ok(&encoded[..width])
}

fn supported_modulus(modulus: &[u8]) -> bool {
    matches!(
        modulus.len(),
        MODULUS_BYTES | MODULUS_3072_BYTES | MODULUS_4096_BYTES
    ) && modulus[0] & 0x80 != 0
        && modulus[modulus.len() - 1] & 1 != 0
}

fn be_to_limbs(bytes: &[u8], limbs: &mut [u64]) {
    for (i, limb) in limbs.iter_mut().enumerate() {
        let start = bytes.len() - (i + 1) * 8;
        let mut word = [0u8; 8];
        word.copy_from_slice(&bytes[start..start + 8]);
        *limb = u64::from_be_bytes(word);
    }
}

fn limbs_to_be(limbs: &[u64], bytes: &mut [u8]) {
    for (i, limb) in limbs.iter().enumerate() {
        let start = bytes.len() - (i + 1) * 8;
        bytes[start..start + 8].copy_from_slice(&limb.to_be_bytes());
    }
}

pub fn from_be_bytes(bytes: &[u8; MODULUS_BYTES]) -> [u64; LIMBS] {
    let mut limbs = [0u64; LIMBS];
    be_to_limbs(bytes, &mut limbs);
    limbs
}

pub fn to_be_bytes(limbs: &[u64; LIMBS]) -> [u8; MODULUS_BYTES] {
    let mut bytes = [0u8; MODULUS_BYTES];
    limbs_to_be(limbs, &mut bytes);
    bytes
}
