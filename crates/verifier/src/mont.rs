//! Montgomery cubes of a 2048-bit integer.
//!
//! `n` is odd. `rr` is `2^4096 mod n` and `n0` is `-n[0]^{-1} mod 2^64`, both
//! depending only on `n`. Each cube is `x^3 mod n`.

use crate::schoolbook::{self, LIMBS, PRODUCT_LIMBS};

/// `-n0^{-1} mod 2^64`.
pub const fn n_prime(n0: u64) -> u64 {
    let mut inv = 1u64;
    let mut round = 0;
    while round < 6 {
        inv = inv.wrapping_mul(2u64.wrapping_sub(n0.wrapping_mul(inv)));
        round += 1;
    }
    inv.wrapping_neg()
}

/// `2^4096 mod n` by doubling. `n` is nonzero.
pub fn r_squared(n: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut acc = [0u64; LIMBS];
    acc[0] = 1;
    let mut step = 0;
    while step < LIMBS * 128 {
        acc = double_mod(&acc, n);
        step += 1;
    }
    acc
}

/// Schoolbook limb scan in Montgomery form. `a` and `b` are less than `n`.
pub fn cube_cios(
    out: &mut [u64; LIMBS],
    x: &[u64; LIMBS],
    n: &[u64; LIMBS],
    rr: &[u64; LIMBS],
    n0: u64,
) {
    let xm = enter(x, n, rr, n0);
    let mut x2 = [0u64; LIMBS];
    cios(&mut x2, &xm, &xm, n, n0);
    let mut x3 = [0u64; LIMBS];
    cios(&mut x3, &x2, &xm, n, n0);
    *out = leave(&x3, n, n0);
}

/// Two-level Karatsuba product, then Montgomery reduction.
pub fn cube_karatsuba(
    out: &mut [u64; LIMBS],
    x: &[u64; LIMBS],
    n: &[u64; LIMBS],
    rr: &[u64; LIMBS],
    n0: u64,
) {
    let xm = enter_karatsuba(x, n, rr, n0);
    let mut wide = [0u64; PRODUCT_LIMBS];
    karatsuba(&mut wide, &xm, &xm);
    let x2 = reduce(&wide, n, n0);
    karatsuba(&mut wide, &x2, &xm);
    let x3 = reduce(&wide, n, n0);
    *out = leave(&x3, n, n0);
}

/// Montgomery exponentiation whose squaring computes each cross term once.
pub fn cube_square(
    out: &mut [u64; LIMBS],
    x: &[u64; LIMBS],
    n: &[u64; LIMBS],
    rr: &[u64; LIMBS],
    n0: u64,
) {
    let xm = enter(x, n, rr, n0);
    let mut wide = [0u64; PRODUCT_LIMBS];
    square(&mut wide, &xm);
    let x2 = reduce(&wide, n, n0);
    let mut x3 = [0u64; LIMBS];
    cios(&mut x3, &x2, &xm, n, n0);
    *out = leave(&x3, n, n0);
}

/// One Karatsuba split. Each half is the unrolled 32-bit product.
pub fn karatsuba(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    let mut low = [0u64; 32];
    let mut high = [0u64; 32];
    schoolbook::mul16_mac(&mut low, &a[..16], &b[..16]);
    schoolbook::mul16_mac(&mut high, &a[16..], &b[16..]);
    let (a_sum, a_carry) = add_from(&a[..16], &a[16..]);
    let (b_sum, b_carry) = add_from(&b[..16], &b[16..]);
    let mut mid = [0u64; 34];
    let mut mid_low = [0u64; 32];
    schoolbook::mul16_mac(&mut mid_low, &a_sum, &b_sum);
    mid[..32].copy_from_slice(&mid_low);
    if a_carry == 1 {
        add_at(&mut mid, &b_sum, 16);
    }
    if b_carry == 1 {
        add_at(&mut mid, &a_sum, 16);
    }
    if a_carry == 1 && b_carry == 1 {
        add_at(&mut mid, &[1], 32);
    }
    sub_from(&mut mid, &low);
    sub_from(&mut mid, &high);
    out[..32].copy_from_slice(&low);
    out[32..].copy_from_slice(&high);
    add_at(out, &mid, 16);
}

/// Square. Off-diagonal products are computed once and doubled.
pub fn square(out: &mut [u64; PRODUCT_LIMBS], a: &[u64; LIMBS]) {
    *out = [0; PRODUCT_LIMBS];
    for i in 0..LIMBS {
        let mut carry = 0u128;
        for j in (i + 1)..LIMBS {
            let product = u128::from(a[i]) * u128::from(a[j]);
            let lo = product as u64;
            let hi = (product >> 64) as u64;
            let (sum_lo, carry_bit) = lo.overflowing_add(out[i + j]);
            let (sum_lo, carry_bit_2) = sum_lo.overflowing_add(carry as u64);
            let (sum_hi, hi_bit) =
                hi.overflowing_add(u64::from(carry_bit) + u64::from(carry_bit_2));
            out[i + j] = sum_lo;
            carry = u128::from(sum_hi) + (u128::from(hi_bit) << 64) + (carry >> 64);
        }
        add_u128(out, i + LIMBS, carry);
    }
    let mut doubled = 0u64;
    for limb in out.iter_mut() {
        let next = *limb >> 63;
        *limb = (*limb << 1) | doubled;
        doubled = next;
    }
    for i in 0..LIMBS {
        add_u128(out, 2 * i, u128::from(a[i]) * u128::from(a[i]));
    }
}

fn enter(x: &[u64; LIMBS], n: &[u64; LIMBS], rr: &[u64; LIMBS], n0: u64) -> [u64; LIMBS] {
    let mut wide = [0u64; PRODUCT_LIMBS];
    schoolbook::mul32_mac(&mut wide, x, rr);
    reduce(&wide, n, n0)
}

fn enter_karatsuba(x: &[u64; LIMBS], n: &[u64; LIMBS], rr: &[u64; LIMBS], n0: u64) -> [u64; LIMBS] {
    let mut wide = [0u64; PRODUCT_LIMBS];
    karatsuba(&mut wide, x, rr);
    reduce(&wide, n, n0)
}

fn leave(value: &[u64; LIMBS], n: &[u64; LIMBS], n0: u64) -> [u64; LIMBS] {
    let mut wide = [0u64; PRODUCT_LIMBS];
    wide[..LIMBS].copy_from_slice(value);
    reduce(&wide, n, n0)
}

fn reduce(product: &[u64; PRODUCT_LIMBS], n: &[u64; LIMBS], n0: u64) -> [u64; LIMBS] {
    let mut t = [0u64; PRODUCT_LIMBS + 2];
    t[..PRODUCT_LIMBS].copy_from_slice(product);
    for i in 0..LIMBS {
        let m = t[i].wrapping_mul(n0);
        let mut carry = 0u128;
        for j in 0..LIMBS {
            let (lo, hi) = schoolbook::widening_mul(m, n[j]);
            let (sum_lo, carry_bit) = lo.overflowing_add(t[i + j]);
            let (sum_lo, carry_bit_2) = sum_lo.overflowing_add(carry as u64);
            let (sum_hi, hi_bit) =
                hi.overflowing_add(u64::from(carry_bit) + u64::from(carry_bit_2));
            t[i + j] = sum_lo;
            carry = u128::from(sum_hi) + (u128::from(hi_bit) << 64) + (carry >> 64);
        }
        let mut k = i + LIMBS;
        while carry != 0 {
            let sum = u128::from(t[k]) + carry;
            t[k] = sum as u64;
            carry = sum >> 64;
            k += 1;
        }
    }
    let mut out = [0u64; LIMBS];
    out.copy_from_slice(&t[LIMBS..PRODUCT_LIMBS]);
    let mut hi = t[PRODUCT_LIMBS];
    if hi != 0 || at_least(&out, n) {
        if sub(&mut out, n) {
            hi -= 1;
        }
    }
    if hi != 0 || at_least(&out, n) {
        sub(&mut out, n);
    }
    out
}

fn cios(out: &mut [u64; LIMBS], a: &[u64; LIMBS], b: &[u64; LIMBS], n: &[u64; LIMBS], n0: u64) {
    let mut t = [0u64; LIMBS + 2];
    for i in 0..LIMBS {
        let mut carry = 0u128;
        for j in 0..LIMBS {
            carry += u128::from(t[j]) + u128::from(a[j]) * u128::from(b[i]);
            t[j] = carry as u64;
            carry >>= 64;
        }
        carry += u128::from(t[LIMBS]);
        t[LIMBS] = carry as u64;
        t[LIMBS + 1] = (carry >> 64) as u64;

        let m = t[0].wrapping_mul(n0);
        carry = u128::from(t[0]) + u128::from(m) * u128::from(n[0]);
        carry >>= 64;
        for j in 1..LIMBS {
            carry += u128::from(t[j]) + u128::from(m) * u128::from(n[j]);
            t[j - 1] = carry as u64;
            carry >>= 64;
        }
        carry += u128::from(t[LIMBS]);
        t[LIMBS - 1] = carry as u64;
        carry >>= 64;
        carry += u128::from(t[LIMBS + 1]);
        t[LIMBS] = carry as u64;
        t[LIMBS + 1] = (carry >> 64) as u64;
    }
    out.copy_from_slice(&t[..LIMBS]);
    let mut hi = t[LIMBS];
    if hi != 0 || at_least(out, n) {
        if sub(out, n) {
            hi -= 1;
        }
    }
    if hi != 0 || at_least(out, n) {
        sub(out, n);
    }
}

fn add_from(a: &[u64], b: &[u64]) -> ([u64; 16], u64) {
    let mut out = [0u64; 16];
    let mut carry = 0u64;
    for i in 0..a.len() {
        let (sum, c1) = a[i].overflowing_add(b[i]);
        let (sum, c2) = sum.overflowing_add(carry);
        out[i] = sum;
        carry = u64::from(c1) + u64::from(c2);
    }
    (out, carry)
}

fn add_at(acc: &mut [u64], src: &[u64], offset: usize) {
    let mut carry = 0u64;
    for i in 0..src.len() {
        let (sum, c1) = acc[offset + i].overflowing_add(src[i]);
        let (sum, c2) = sum.overflowing_add(carry);
        acc[offset + i] = sum;
        carry = u64::from(c1) + u64::from(c2);
    }
    let mut index = offset + src.len();
    while carry != 0 {
        let (sum, overflow) = acc[index].overflowing_add(carry);
        acc[index] = sum;
        carry = u64::from(overflow);
        index += 1;
    }
}

fn sub_from(acc: &mut [u64], rhs: &[u64]) {
    let mut borrow = false;
    for i in 0..rhs.len() {
        let (diff, b1) = acc[i].overflowing_sub(rhs[i]);
        let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
        acc[i] = diff;
        borrow = b1 || b2;
    }
    let mut index = rhs.len();
    while borrow {
        let (diff, b) = acc[index].overflowing_sub(1);
        acc[index] = diff;
        borrow = b;
        index += 1;
    }
}

fn add_u128(out: &mut [u64], index: usize, mut extra: u128) {
    let mut index = index;
    while extra != 0 {
        let (sum, overflow) = out[index].overflowing_add(extra as u64);
        out[index] = sum;
        extra = (extra >> 64) + u128::from(overflow);
        index += 1;
    }
}

fn double_mod(a: &[u64; LIMBS], n: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut out = [0u64; LIMBS];
    let mut carry = 0u64;
    for i in 0..LIMBS {
        let next = a[i] >> 63;
        out[i] = (a[i] << 1) | carry;
        carry = next;
    }
    if carry == 1 || at_least(&out, n) {
        sub(&mut out, n);
    }
    out
}

fn at_least(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> bool {
    for i in (0..LIMBS).rev() {
        if a[i] != b[i] {
            return a[i] > b[i];
        }
    }
    true
}

fn sub(a: &mut [u64; LIMBS], b: &[u64; LIMBS]) -> bool {
    let mut borrow = false;
    for i in 0..LIMBS {
        let (diff, b1) = a[i].overflowing_sub(b[i]);
        let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
        a[i] = diff;
        borrow = b1 || b2;
    }
    borrow
}
