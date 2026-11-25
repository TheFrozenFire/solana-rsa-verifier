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

/// Same cube, but each quotient digit is a 128÷64 division built from `u64` division.
pub fn cube_quot(out: &mut [u64; LIMBS], x: &[u64; LIMBS], n: &[u64; LIMBS]) {
    let mut wide = [0u64; PRODUCT_LIMBS];
    schoolbook::mul(&mut wide, x, x);
    let square = remainder_quot(&wide, n);
    schoolbook::mul(&mut wide, &square, x);
    *out = remainder_quot(&wide, n);
}

/// Divide `hi * 2^64 + lo` by `d`. `hi < d` and `d != 0`, so the quotient fits in `u64`.
pub fn div_rem_128_64(hi: u64, lo: u64, d: u64) -> (u64, u64) {
    if hi == 0 {
        return (lo / d, lo % d);
    }
    let shift = d.leading_zeros();
    let v = d << shift;
    let vn1 = v >> 32;
    let vn0 = v & 0xffff_ffff;
    let u_hi = if shift == 0 {
        hi
    } else {
        (hi << shift) | (lo >> (64 - shift))
    };
    let u_lo = lo << shift;
    let u_mid = u_lo >> 32;
    let u_bot = u_lo & 0xffff_ffff;
    let (q1, step) = quot_digit(u_hi, u_mid, vn1, vn0, v);
    let (q0, rem) = quot_digit(step, u_bot, vn1, vn0, v);
    ((q1 << 32) | q0, rem >> shift)
}

fn quot_digit(top: u64, low: u64, vn1: u64, vn0: u64, v: u64) -> (u64, u64) {
    let base = 1u64 << 32;
    let q_raw = top / vn1;
    let r_raw = top % vn1;
    let (mut q, mut rhat) = if q_raw >= base {
        let adjusted = u128::from(top) - u128::from(base - 1) * u128::from(vn1);
        (base - 1, adjusted as u64)
    } else {
        (q_raw, r_raw)
    };
    if rhat < base {
        while u128::from(q) * u128::from(vn0) > (u128::from(rhat) << 32) + u128::from(low) {
            q -= 1;
            rhat += vn1;
            if rhat >= base {
                break;
            }
        }
    }
    let value = (u128::from(top) << 32) | u128::from(low);
    let mut prod = u128::from(q) * u128::from(v);
    if prod > value {
        q -= 1;
        prod -= u128::from(v);
    }
    if prod > value {
        q -= 1;
        prod -= u128::from(v);
    }
    (q, (value - prod) as u64)
}

fn remainder_quot(product: &[u64; PRODUCT_LIMBS], modulus: &[u64; LIMBS]) -> [u64; LIMBS] {
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
        return rem_limb_quot(&u, v[0], shift);
    }

    let steps = u.len() - n;
    for j in (0..steps).rev() {
        let top = u[j + n];
        let hi = u[j + n - 1];
        let (mut qhat, mut rhat) = if top == v[n - 1] {
            (u64::MAX, u128::from(hi) + u128::from(v[n - 1]))
        } else {
            let (q, r) = div_rem_128_64(top, hi, v[n - 1]);
            (q, u128::from(r))
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

fn rem_limb_quot(u: &[u64], divisor: u64, shift: u32) -> [u64; LIMBS] {
    let mut acc = 0u64;
    for limb in u.iter().rev() {
        acc = div_rem_128_64(acc, *limb, divisor).1;
    }
    let mut out = [0u64; LIMBS];
    out[0] = acc;
    shr(&mut out, shift);
    out
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

const DIGIT: u64 = 0xffff_ffff;

/// Schoolbook products, with the remainder's `q * n` multiply done in 32-bit limbs.
pub fn cube_rem32(out: &mut [u64; LIMBS], x: &[u64; LIMBS], n: &[u64; LIMBS]) {
    let mut wide = [0u64; PRODUCT_LIMBS];
    schoolbook::mul(&mut wide, x, x);
    let square = remainder32(&wide, n);
    schoolbook::mul(&mut wide, &square, x);
    *out = remainder32(&wide, n);
}

fn remainder32(product: &[u64; PRODUCT_LIMBS], modulus: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut u = [0u64; 129];
    let mut v = [0u64; 64];
    for i in 0..PRODUCT_LIMBS {
        u[2 * i] = product[i] & DIGIT;
        u[2 * i + 1] = product[i] >> 32;
    }
    for i in 0..LIMBS {
        v[2 * i] = modulus[i] & DIGIT;
        v[2 * i + 1] = modulus[i] >> 32;
    }
    let nlen = match v.iter().rposition(|digit| *digit != 0) {
        Some(index) => index + 1,
        None => return [0; LIMBS],
    };
    let shift = (v[nlen - 1] as u32).leading_zeros();
    shl32(&mut v[..nlen], shift);
    shl32(&mut u, shift);

    if nlen == 1 {
        let mut acc = 0u64;
        for digit in u.iter().rev() {
            acc = ((acc << 32) | *digit) % v[0];
        }
        let mut out = [0u64; LIMBS];
        out[0] = acc >> shift;
        return out;
    }

    let steps = u.len() - nlen;
    for j in (0..steps).rev() {
        let top = u[j + nlen];
        let hi = u[j + nlen - 1];
        let numer = (top << 32) | hi;
        let vtop = v[nlen - 1];
        let mut qhat = numer / vtop;
        let mut rhat = u128::from(numer % vtop);
        if qhat > DIGIT {
            qhat = DIGIT;
            rhat = u128::from(numer) - u128::from(qhat) * u128::from(vtop);
        }
        for _ in 0..2 {
            if rhat >= (1u128 << 32) {
                break;
            }
            let left = u128::from(qhat) * u128::from(v[nlen - 2]);
            let right = (rhat << 32) + u128::from(u[j + nlen - 2]);
            if left <= right {
                break;
            }
            qhat -= 1;
            rhat += u128::from(vtop);
        }
        let window = &mut u[j..j + nlen + 1];
        if mul_sub32(window, &v[..nlen], qhat) {
            add_back32(window, &v[..nlen]);
        }
    }

    shr32(&mut u[..nlen], shift);
    let mut out = [0u64; LIMBS];
    for i in 0..LIMBS {
        out[i] = u[2 * i] | (u[2 * i + 1] << 32);
    }
    out
}

fn mul_sub32(u: &mut [u64], v: &[u64], q: u64) -> bool {
    let mut carry = 0u64;
    for i in 0..v.len() {
        let prod = q * v[i] + carry;
        let lo = prod & DIGIT;
        carry = prod >> 32;
        if u[i] >= lo {
            u[i] -= lo;
        } else {
            u[i] = (u[i] + (1 << 32)) - lo;
            carry += 1;
        }
    }
    let last = v.len();
    if u[last] >= carry {
        u[last] -= carry;
        false
    } else {
        u[last] = (u[last] + (1 << 32)) - carry;
        true
    }
}

fn add_back32(u: &mut [u64], v: &[u64]) {
    let mut carry = 0u64;
    for i in 0..v.len() {
        let sum = u[i] + v[i] + carry;
        u[i] = sum & DIGIT;
        carry = sum >> 32;
    }
    let last = v.len();
    let sum = u[last] + carry;
    u[last] = sum & DIGIT;
}

fn shl32(words: &mut [u64], bits: u32) {
    if bits == 0 {
        return;
    }
    let mut carry = 0u64;
    for word in words.iter_mut() {
        let next = *word >> (32 - bits);
        *word = ((*word << bits) | carry) & DIGIT;
        carry = next;
    }
}

fn shr32(words: &mut [u64], bits: u32) {
    if bits == 0 {
        return;
    }
    let mut carry = 0u64;
    for word in words.iter_mut().rev() {
        let next = (*word << (32 - bits)) & DIGIT;
        *word = (*word >> bits) | carry;
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
