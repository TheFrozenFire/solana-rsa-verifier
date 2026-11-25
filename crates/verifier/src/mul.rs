//! 2048-bit product. Limbs are unpacked once, and each inner row is unrolled.

use crate::LIMBS;

const PRODUCT_LIMBS: usize = LIMBS * 2;

pub fn product(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    let mut left = [0u64; 64];
    let mut right = [0u64; 64];
    for i in 0..LIMBS {
        left[2 * i] = a[i] & 0xffff_ffff;
        left[2 * i + 1] = a[i] >> 32;
        right[2 * i] = b[i] & 0xffff_ffff;
        right[2 * i + 1] = b[i] >> 32;
    }
    let mut digit = [0u64; 128];
    for i in 0..64 {
        let ai = left[i];
        let mut carry = 0u64;
        let mut j = 0;
        while j < 64 {
            carry = mac(&mut digit, i + j, ai, right[j], carry);
            carry = mac(&mut digit, i + j + 1, ai, right[j + 1], carry);
            carry = mac(&mut digit, i + j + 2, ai, right[j + 2], carry);
            carry = mac(&mut digit, i + j + 3, ai, right[j + 3], carry);
            j += 4;
        }
        digit[i + 64] = carry;
    }
    for i in 0..PRODUCT_LIMBS {
        out[i] = digit[2 * i] | (digit[2 * i + 1] << 32);
    }
}

#[inline(always)]
fn mac(digit: &mut [u64], index: usize, ai: u64, bj: u64, carry: u64) -> u64 {
    let sum = digit[index] + ai * bj + carry;
    digit[index] = sum & 0xffff_ffff;
    sum >> 32
}
