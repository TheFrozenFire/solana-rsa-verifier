//! Schoolbook multiplication of two 2048-bit integers.

pub const LIMBS: usize = 32;
pub const PRODUCT_LIMBS: usize = LIMBS * 2;

pub fn mul(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    *out = [0; PRODUCT_LIMBS];
    for i in 0..LIMBS {
        let mut carry = 0u128;
        for j in 0..LIMBS {
            let product = u128::from(a[i]) * u128::from(b[j]);
            let lo = product as u64;
            let hi = (product >> 64) as u64;
            let (sum_lo, carry_bit) = lo.overflowing_add(out[i + j]);
            let (sum_lo, carry_bit_2) = sum_lo.overflowing_add(carry as u64);
            let (sum_hi, hi_bit) =
                hi.overflowing_add(u64::from(carry_bit) + u64::from(carry_bit_2));
            out[i + j] = sum_lo;
            carry = u128::from(sum_hi) + (u128::from(hi_bit) << 64) + (carry >> 64);
        }
        add_carry(out, i + LIMBS, carry);
    }
}

/// Same schoolbook product, with the 64×64→128 step written as four `u64` multiplies.
pub fn mul_wide(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    *out = [0; PRODUCT_LIMBS];
    for i in 0..LIMBS {
        let mut carry = 0u128;
        for j in 0..LIMBS {
            let (lo, hi) = widening_mul(a[i], b[j]);
            let (sum_lo, carry_bit) = lo.overflowing_add(out[i + j]);
            let (sum_lo, carry_bit_2) = sum_lo.overflowing_add(carry as u64);
            let (sum_hi, hi_bit) =
                hi.overflowing_add(u64::from(carry_bit) + u64::from(carry_bit_2));
            out[i + j] = sum_lo;
            carry = u128::from(sum_hi) + (u128::from(hi_bit) << 64) + (carry >> 64);
        }
        add_carry(out, i + LIMBS, carry);
    }
}

/// 32-bit schoolbook product. Limbs are unpacked once, and the inner row is unrolled.
pub fn mul32_mac(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
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
            carry = mac_digit(&mut digit, i + j, ai, right[j], carry);
            carry = mac_digit(&mut digit, i + j + 1, ai, right[j + 1], carry);
            carry = mac_digit(&mut digit, i + j + 2, ai, right[j + 2], carry);
            carry = mac_digit(&mut digit, i + j + 3, ai, right[j + 3], carry);
            j += 4;
        }
        digit[i + 64] = carry;
    }
    for i in 0..PRODUCT_LIMBS {
        out[i] = digit[2 * i] | (digit[2 * i + 1] << 32);
    }
}

#[inline(always)]
fn mac_digit(digit: &mut [u64; 128], index: usize, ai: u64, bj: u64, carry: u64) -> u64 {
    let sum = digit[index] + ai * bj + carry;
    digit[index] = sum & 0xffff_ffff;
    sum >> 32
}

/// Same product, one `u64` multiply per 32-bit limb.
pub fn mul32(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    let mut digit = [0u64; 128];
    for i in 0..64 {
        let ai = u32_limb(a, i);
        let mut carry = 0u64;
        for j in 0..64 {
            let sum = digit[i + j] + ai * u32_limb(b, j) + carry;
            digit[i + j] = sum & 0xffff_ffff;
            carry = sum >> 32;
        }
        digit[i + 64] = carry;
    }
    for i in 0..PRODUCT_LIMBS {
        out[i] = digit[2 * i] | (digit[2 * i + 1] << 32);
    }
}

/// Four 32-bit products. The high half never goes through a `u128` multiply.
#[inline(always)]
fn widening_mul(a: u64, b: u64) -> (u64, u64) {
    let a0 = a & 0xffff_ffff;
    let a1 = a >> 32;
    let b0 = b & 0xffff_ffff;
    let b1 = b >> 32;
    let p00 = a0 * b0;
    let p01 = a0 * b1;
    let p10 = a1 * b0;
    let p11 = a1 * b1;
    let (mid, mid_overflow) = p01.overflowing_add(p10);
    let (lo, lo_overflow) = p00.overflowing_add(mid << 32);
    let mut hi = p11 + (mid >> 32);
    if mid_overflow {
        hi += 1 << 32;
    }
    if lo_overflow {
        hi += 1;
    }
    (lo, hi)
}

fn u32_limb(words: &[u64; LIMBS], index: usize) -> u64 {
    let word = words[index / 2];
    if index % 2 == 0 {
        word & 0xffff_ffff
    } else {
        word >> 32
    }
}

fn add_carry(out: &mut [u64; PRODUCT_LIMBS], index: usize, mut extra: u128) {
    let mut index = index;
    while extra != 0 {
        let (sum, overflow) = out[index].overflowing_add(extra as u64);
        out[index] = sum;
        extra = (extra >> 64) + u128::from(overflow);
        index += 1;
    }
}
