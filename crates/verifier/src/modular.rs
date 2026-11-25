//! `x^3 mod n` for 2048-bit integers.
//!
//! `n` is nonzero. Each product is a schoolbook multiply, and the remainder is
//! schoolbook division.

use crate::schoolbook::{self, LIMBS, PRODUCT_LIMBS};

pub fn cube(out: &mut [u64; LIMBS], x: &[u64; LIMBS], n: &[u64; LIMBS]) {
    let mut wide = [0u64; PRODUCT_LIMBS];
    schoolbook::mul(&mut wide, x, x);
    let square = remainder(&wide, n);
    schoolbook::mul(&mut wide, &square, x);
    *out = remainder(&wide, n);
}

fn remainder(product: &[u64; PRODUCT_LIMBS], modulus: &[u64; LIMBS]) -> [u64; LIMBS] {
    let n = match significant_limbs(modulus) {
        0 => return [0; LIMBS],
        n => n,
    };
    let shift = modulus[n - 1].leading_zeros();
    let mut v = [0u64; LIMBS];
    v[..n].copy_from_slice(&modulus[..n]);
    let mut u = [0u64; PRODUCT_LIMBS + 1];
    u[..PRODUCT_LIMBS].copy_from_slice(product);
    shl(&mut v[..n], shift);
    shl(&mut u, shift);

    if n == 1 {
        return rem_limb(&u, v[0], shift);
    }

    let steps = u.len() - n;
    for j in (0..steps).rev() {
        let top = u[j + n];
        let hi = u[j + n - 1];
        let (mut qhat, mut rhat) = if top == v[n - 1] {
            (u64::MAX, u128::from(hi) + u128::from(v[n - 1]))
        } else {
            let numerator = (u128::from(top) << 64) | u128::from(hi);
            let divisor = u128::from(v[n - 1]);
            ((numerator / divisor) as u64, numerator % divisor)
        };
        while rhat < (1u128 << 64)
            && u128::from(qhat) * u128::from(v[n - 2]) > (rhat << 64) | u128::from(u[j + n - 2])
        {
            qhat -= 1;
            rhat += u128::from(v[n - 1]);
        }
        let window = &mut u[j..j + n + 1];
        if mul_sub(window, &v[..n], qhat) {
            add_back(window, &v[..n]);
        }
    }

    let mut out = [0u64; LIMBS];
    out[..n].copy_from_slice(&u[..n]);
    shr(&mut out, shift);
    out
}

fn significant_limbs(value: &[u64; LIMBS]) -> usize {
    value
        .iter()
        .rposition(|limb| *limb != 0)
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn rem_limb(u: &[u64], divisor: u64, shift: u32) -> [u64; LIMBS] {
    let mut acc = 0u128;
    for limb in u.iter().rev() {
        acc = ((acc << 64) | u128::from(*limb)) % u128::from(divisor);
    }
    let mut out = [0u64; LIMBS];
    out[0] = acc as u64;
    shr(&mut out, shift);
    out
}

/// Subtract `q * v` from `u`. `u` is one limb longer than `v`.
/// Returns whether the subtraction borrowed past the last limb.
fn mul_sub(u: &mut [u64], v: &[u64], q: u64) -> bool {
    let n = v.len();
    let mut carry = 0u128;
    for i in 0..n {
        let prod = u128::from(q) * u128::from(v[i]) + carry;
        carry = prod >> 64;
        let (diff, borrowed) = u[i].overflowing_sub(prod as u64);
        u[i] = diff;
        if borrowed {
            carry += 1;
        }
    }
    let top = u128::from(u[n]);
    if carry <= top {
        u[n] = (top - carry) as u64;
        false
    } else {
        u[n] = (top + (1u128 << 64) - carry) as u64;
        true
    }
}

fn add_back(u: &mut [u64], v: &[u64]) {
    let mut carry = 0u64;
    for i in 0..v.len() {
        let (sum, c1) = u[i].overflowing_add(v[i]);
        let (sum, c2) = sum.overflowing_add(carry);
        u[i] = sum;
        carry = u64::from(c1) + u64::from(c2);
    }
    let last = v.len();
    u[last] = u[last].wrapping_add(carry);
}

fn shl(words: &mut [u64], bits: u32) {
    if bits == 0 {
        return;
    }
    let mut carry = 0u64;
    for word in words.iter_mut() {
        let next = *word >> (64 - bits);
        *word = (*word << bits) | carry;
        carry = next;
    }
}

fn shr(words: &mut [u64], bits: u32) {
    if bits == 0 {
        return;
    }
    let mut carry = 0u64;
    for word in words.iter_mut().rev() {
        let next = *word << (64 - bits);
        *word = (*word >> bits) | carry;
        carry = next;
    }
}
