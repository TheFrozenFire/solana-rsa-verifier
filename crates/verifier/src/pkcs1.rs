//! PKCS#1 v1.5 signature encoding, checked as an exact byte string.
//!
//! The `FF` run is at least 8 octets. The DigestInfo prefix includes the
//! `NULL` parameter from RFC 8017.

use crate::{SHA256_DIGESTINFO, SHA384_DIGESTINFO, SHA512_DIGESTINFO};

pub(crate) fn sha256(encoded: &[u8], digest: &[u8; 32]) -> bool {
    matches(encoded, &SHA256_DIGESTINFO, digest)
}

pub(crate) fn sha384(encoded: &[u8], digest: &[u8; 48]) -> bool {
    matches(encoded, &SHA384_DIGESTINFO, digest)
}

pub(crate) fn sha512(encoded: &[u8], digest: &[u8; 64]) -> bool {
    matches(encoded, &SHA512_DIGESTINFO, digest)
}

fn matches(encoded: &[u8], digest_info: &[u8], digest: &[u8]) -> bool {
    let body = digest_info.len() + digest.len();
    // 00 01 || PS || 00 || DigestInfo || digest, and PS is at least 8 octets.
    if encoded.len() < body + 11 {
        return false;
    }
    let ff_len = encoded.len() - body - 3;
    let separator = 2 + ff_len;
    encoded[0] == 0x00
        && encoded[1] == 0x01
        && encoded[2..separator].iter().all(|byte| *byte == 0xff)
        && encoded[separator] == 0x00
        && encoded[separator + 1..separator + 1 + digest_info.len()] == *digest_info
        && encoded[encoded.len() - digest.len()..] == *digest
}

#[cfg(test)]
mod tests {
    use super::matches;
    use crate::{SHA256_DIGESTINFO, SHA384_DIGESTINFO, SHA512_DIGESTINFO};

    #[test]
    fn each_hash_has_its_own_pad_length() {
        let sha256 = em(256, &SHA256_DIGESTINFO, &[0x11; 32]);
        let sha384 = em(256, &SHA384_DIGESTINFO, &[0x22; 48]);
        let sha512 = em(256, &SHA512_DIGESTINFO, &[0x33; 64]);
        assert!(matches(&sha256[..256], &SHA256_DIGESTINFO, &[0x11; 32]));
        assert!(matches(&sha384[..256], &SHA384_DIGESTINFO, &[0x22; 48]));
        assert!(matches(&sha512[..256], &SHA512_DIGESTINFO, &[0x33; 64]));
        let wide = em(384, &SHA256_DIGESTINFO, &[0x11; 32]);
        assert!(matches(&wide[..384], &SHA256_DIGESTINFO, &[0x11; 32]));
        let wider = em(512, &SHA512_DIGESTINFO, &[0x33; 64]);
        assert!(matches(&wider[..512], &SHA512_DIGESTINFO, &[0x33; 64]));
    }

    #[test]
    fn a_sha256_layout_is_not_a_sha512_signature() {
        let encoded = em(256, &SHA256_DIGESTINFO, &[0x11; 32]);
        assert!(!matches(&encoded[..256], &SHA512_DIGESTINFO, &[0x11; 64]));
    }

    #[test]
    fn a_short_ff_run_is_rejected() {
        let mut encoded = em(256, &SHA256_DIGESTINFO, &[0x11; 32]);
        encoded[10] = 0x00;
        assert!(!matches(&encoded[..256], &SHA256_DIGESTINFO, &[0x11; 32]));
    }

    #[test]
    fn an_omitted_null_parameter_is_rejected() {
        // SHA-256 DigestInfo without the 05 00 NULL parameter.
        let info: [u8; 17] = [
            0x30, 0x2f, 0x30, 0x0b, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02,
            0x01, 0x04, 0x20,
        ];
        let digest = [0x11u8; 32];
        let encoded = em_with(256, &info, &digest);
        assert!(!matches(&encoded[..256], &SHA256_DIGESTINFO, &digest));
    }

    fn em(k: usize, info: &[u8], digest: &[u8]) -> [u8; 512] {
        em_with(k, info, digest)
    }

    fn em_with(k: usize, info: &[u8], digest: &[u8]) -> [u8; 512] {
        let mut encoded = [0u8; 512];
        let body = info.len() + digest.len();
        let ff_len = k - body - 3;
        encoded[1] = 0x01;
        for byte in &mut encoded[2..2 + ff_len] {
            *byte = 0xff;
        }
        encoded[2 + ff_len + 1..2 + ff_len + 1 + info.len()].copy_from_slice(info);
        encoded[k - digest.len()..k].copy_from_slice(digest);
        encoded
    }
}
