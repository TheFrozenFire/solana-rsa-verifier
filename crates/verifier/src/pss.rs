//! RSASSA-PSS with SHA-256, MGF1-SHA-256, a 32-byte salt, and trailer `0xbc`.
//!
//! The modulus has its top bit set, so the encoded message is one bit shorter
//! than the modulus and the top bit of the representative is zero.

use crate::sha256::sha256;

const DIGEST_LEN: usize = 32;
const SALT_LEN: usize = 32;

pub(crate) fn sha256_salt32(encoded: &[u8], digest: &[u8; DIGEST_LEN]) -> bool {
    if encoded.len() < DIGEST_LEN + SALT_LEN + 2 || encoded[encoded.len() - 1] != 0xbc {
        return false;
    }
    // emBits = modulus bit length - 1, which is one bit short of a full byte.
    if encoded[0] & 0x80 != 0 {
        return false;
    }
    let masked_len = encoded.len() - DIGEST_LEN - 1;
    let masked_db = &encoded[..masked_len];
    let Ok(hashed) = <[u8; DIGEST_LEN]>::try_from(&encoded[masked_len..masked_len + DIGEST_LEN])
    else {
        return false;
    };
    if masked_db[0] & 0x80 != 0 {
        return false;
    }

    let mut db = [0u8; 512];
    let db = &mut db[..masked_len];
    mgf1_xor(&hashed, masked_db, db);
    db[0] &= 0x7f;

    let pad_len = encoded.len() - DIGEST_LEN - SALT_LEN - 2;
    if db[..pad_len].iter().any(|byte| *byte != 0) || db[pad_len] != 0x01 {
        return false;
    }
    let salt = &db[pad_len + 1..];
    let mut block = [0u8; 8 + DIGEST_LEN + SALT_LEN];
    block[8..8 + DIGEST_LEN].copy_from_slice(digest);
    block[8 + DIGEST_LEN..].copy_from_slice(salt);
    sha256(&block) == hashed
}

fn mgf1_xor(seed: &[u8], masked: &[u8], out: &mut [u8]) {
    let mut counter = 0u32;
    let mut offset = 0;
    while offset < out.len() {
        let mut input = [0u8; DIGEST_LEN + 4];
        input[..DIGEST_LEN].copy_from_slice(seed);
        input[DIGEST_LEN..].copy_from_slice(&counter.to_be_bytes());
        let digest = sha256(&input);
        let take = (out.len() - offset).min(DIGEST_LEN);
        for i in 0..take {
            out[offset + i] = masked[offset + i] ^ digest[i];
        }
        offset += take;
        counter += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::sha256_salt32;
    use crate::sha256::sha256;

    #[test]
    fn accepts_an_encoding_this_crate_produced() {
        let digest = sha256(b"solana-rsa-verifier");
        let salt = [0x5a; 32];
        let encoded = encode(&digest, &salt);
        assert!(sha256_salt32(&encoded, &digest));
        let mut flipped = encoded;
        flipped[0] |= 0x80;
        assert!(!sha256_salt32(&flipped, &digest));
    }

    #[test]
    fn rejects_a_different_salt_length() {
        let digest = sha256(b"solana-rsa-verifier");
        let encoded = encode(&digest, &[]);
        assert!(!sha256_salt32(&encoded, &digest));
    }

    fn encode(digest: &[u8; 32], salt: &[u8]) -> [u8; 256] {
        let mut db = [0u8; 256 - 32 - 1];
        let pad_len = db.len() - salt.len() - 1;
        db[pad_len] = 0x01;
        db[pad_len + 1..].copy_from_slice(salt);
        let mut block = [0u8; 72];
        block[8..40].copy_from_slice(digest);
        block[40..40 + salt.len()].copy_from_slice(salt);
        let hashed = sha256(&block[..8 + 32 + salt.len()]);
        let mut masked = db;
        xor_mask(&hashed, &mut masked);
        masked[0] &= 0x7f;
        let mut encoded = [0u8; 256];
        encoded[..masked.len()].copy_from_slice(&masked);
        encoded[masked.len()..masked.len() + 32].copy_from_slice(&hashed);
        encoded[255] = 0xbc;
        encoded
    }

    fn xor_mask(seed: &[u8], out: &mut [u8]) {
        let mut counter = 0u32;
        let mut offset = 0;
        while offset < out.len() {
            let mut input = [0u8; 36];
            input[..32].copy_from_slice(seed);
            input[32..].copy_from_slice(&counter.to_be_bytes());
            let digest = sha256(&input);
            let take = (out.len() - offset).min(32);
            for i in 0..take {
                out[offset + i] ^= digest[i];
            }
            offset += take;
            counter += 1;
        }
    }
}
