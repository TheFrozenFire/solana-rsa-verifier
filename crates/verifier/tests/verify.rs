use solana_rsa_verifier::{Error, LIMBS, from_be_bytes, pow3, verify};

const MODULUS: &str = "a10f8e89a9fe6aca4ff491cf65577de3179ffdf9c5f2877b3d7d34d297d131037b58974ee69abf9d4f8ae2f8a88f8ff5de516258d98685d769d2365db1d5dc8c705ad0290235712e838e84ceaa9dd304a29c0d0b38883b78315562fd75e31e7572191d4850e1f5498ef27fe0f6f65133017ab8c21ea975ff89ba4902f42d090a9ff0c55f6699e8c52eac2aba1c9a634708ae934932eda69e2a3a66f2a31e1b17815d0ac64990b5493e7a966b96f0129051182ef8d1f13270be8c445e1a128989d5f33083abc4125ab7f53c4ec997d0902c1cd8e6da8e6f89d5c35e0bf518097884b8ecae88de1837acbd55a98d4887129b39a11ef1f293da1a38ce503731ff81";
const SIGNATURE: &str = "05d0c944d14c7895fbd25f92889f46e79b9f538fcae9e7a647cd398ef58a6d94d7ef59c70a7dd628cdc8bf91aa0d5733ae5b734d356dd9ceafb45fe2528a43671a5e8dce327021c4c6592b1841c859c5491cdc6f9ec6b25363b1772bd2c9c349c83035be1c4ecbc195d09a74944172c095f341e9725e79bb1461c13c5b2e6666af2af977c3876e074b6a81e9d41d824a505cd7f5333fb9706f0dce6ea951976504c35c46f2749f3364be08e740ca521f7862f05bc34bc8e7191c85140d56dabdf85659e45e5d85ed8c186df95e079dee9e5bc1041978568645595287aee56f22c1496f7b1262a86abd84bc362cab6a46eb0fbc7d69b1ce764978c26998b6bd5b";
const HASH: &str = "c690b0c615b5af8fedfb05d872b840f1b20ea63a6a6f32186d55e3c6621cba24";

#[test]
fn accepts_a_pkcs1_sha256_signature() {
    let modulus = hex(MODULUS);
    let signature = hex(SIGNATURE);
    let hash = hex32(HASH);
    assert_eq!(verify(&modulus, &signature, &hash), Ok(()));
}

#[test]
fn rejects_a_tampered_hash() {
    let modulus = hex(MODULUS);
    let signature = hex(SIGNATURE);
    let mut hash = hex32(HASH);
    hash[0] ^= 1;
    assert_eq!(verify(&modulus, &signature, &hash), Err(Error::Padding));
}

#[test]
fn rejects_a_representative_outside_the_modulus() {
    let modulus = hex(MODULUS);
    let mut signature = hex(SIGNATURE);
    signature.copy_from_slice(&modulus);
    let hash = hex32(HASH);
    assert_eq!(
        verify(&modulus, &signature, &hash),
        Err(Error::Representative)
    );
}

#[test]
fn rejects_an_even_modulus() {
    let mut modulus = hex(MODULUS);
    modulus[255] &= !1;
    let signature = hex(SIGNATURE);
    let hash = hex32(HASH);
    assert_eq!(verify(&modulus, &signature, &hash), Err(Error::Modulus));
}

#[test]
fn pow3_matches_a_bit_by_bit_remainder() {
    let mut x = [0u64; LIMBS];
    let mut n = [0u64; LIMBS];
    x[0] = 6;
    n[0] = 7;
    assert_eq!(pow3(&x, &n)[0], 6);
    x[0] = 10;
    assert_eq!(pow3(&x, &n)[0], 6);
    x[0] = 0;
    assert_eq!(pow3(&x, &n), [0; LIMBS]);

    let modulus = from_be_bytes(&hex(MODULUS));
    let signature = from_be_bytes(&hex(SIGNATURE));
    assert_eq!(
        pow3(&signature, &modulus),
        reference_pow3(&signature, &modulus)
    );
}

fn reference_pow3(x: &[u64; LIMBS], n: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut wide = [0u64; LIMBS * 2];
    school(&mut wide, x, x);
    let square = rem(&wide, n);
    school(&mut wide, &square, x);
    rem(&wide, n)
}

fn school(out: &mut [u64; LIMBS * 2], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    *out = [0; LIMBS * 2];
    for i in 0..LIMBS {
        let mut carry = 0u128;
        for j in 0..LIMBS {
            let product = u128::from(a[i]) * u128::from(b[j]) + u128::from(out[i + j]) + carry;
            out[i + j] = product as u64;
            carry = product >> 64;
        }
        let mut index = i + LIMBS;
        while carry != 0 {
            let sum = u128::from(out[index]) + carry;
            out[index] = sum as u64;
            carry = sum >> 64;
            index += 1;
        }
    }
}

fn rem(product: &[u64; LIMBS * 2], n: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut acc = [0u64; LIMBS + 1];
    for bit in (0..LIMBS * 128).rev() {
        let mut carry = 0u64;
        for limb in acc.iter_mut() {
            let next = *limb >> 63;
            *limb = (*limb << 1) | carry;
            carry = next;
        }
        if (product[bit / 64] >> (bit % 64)) & 1 == 1 {
            acc[0] |= 1;
        }
        if at_least(&acc, n) {
            let mut borrow = false;
            for limb in 0..LIMBS {
                let (diff, b1) = acc[limb].overflowing_sub(n[limb]);
                let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
                acc[limb] = diff;
                borrow = b1 || b2;
            }
            acc[LIMBS] = acc[LIMBS].wrapping_sub(u64::from(borrow));
        }
    }
    let mut out = [0u64; LIMBS];
    out.copy_from_slice(&acc[..LIMBS]);
    out
}

fn at_least(acc: &[u64; LIMBS + 1], n: &[u64; LIMBS]) -> bool {
    if acc[LIMBS] != 0 {
        return true;
    }
    for limb in (0..LIMBS).rev() {
        if acc[limb] != n[limb] {
            return acc[limb] > n[limb];
        }
    }
    true
}

fn hex(text: &str) -> [u8; 256] {
    let mut out = [0u8; 256];
    for i in 0..256 {
        out[i] = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap();
    }
    out
}

fn hex32(text: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap();
    }
    out
}
