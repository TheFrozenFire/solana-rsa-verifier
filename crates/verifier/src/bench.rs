//! Same product and remainder as [`crate::pow3`], for other limb counts.
//!
//! Digits are read from the `u64` limbs in place. The 2048-bit exponent-3
//! path used by [`crate::pow3`] unpacks those digits first, which needs more
//! stack than a 4096-bit operand can spare.
//!
//! Each width is a concrete copy. Array lengths such as `L * 2` are not
//! allowed on a const generic.

use crate::LIMBS;

macro_rules! define_width {
    ($name:ident, $limbs:literal) => {
        pub mod $name {
            pub fn modexp(
                base: &[u64; $limbs],
                modulus: &[u64; $limbs],
                exponent: u32,
            ) -> [u64; $limbs] {
                match exponent {
                    3 => cube(base, modulus),
                    65537 => exp_65537(base, modulus),
                    _ => [0; $limbs],
                }
            }

            fn cube(base: &[u64; $limbs], modulus: &[u64; $limbs]) -> [u64; $limbs] {
                let mut wide = [0u64; $limbs * 2];
                product(&mut wide, base, base);
                let square = remainder(&wide, modulus);
                product(&mut wide, &square, base);
                remainder(&wide, modulus)
            }

            fn exp_65537(base: &[u64; $limbs], modulus: &[u64; $limbs]) -> [u64; $limbs] {
                let mut acc = *base;
                let mut wide = [0u64; $limbs * 2];
                let mut i = 0;
                while i < 16 {
                    product(&mut wide, &acc, &acc);
                    acc = remainder(&wide, modulus);
                    i += 1;
                }
                product(&mut wide, &acc, base);
                remainder(&wide, modulus)
            }

            fn product(out: &mut [u64; $limbs * 2], a: &[u64; $limbs], b: &[u64; $limbs]) {
                let digits = $limbs * 2;
                let mut digit = [0u64; $limbs * 4];
                for i in 0..digits {
                    let ai = u32_at(a, i);
                    let mut carry = 0u64;
                    let mut j = 0;
                    while j < digits {
                        carry = mac(&mut digit, i + j, ai, u32_at(b, j), carry);
                        carry = mac(&mut digit, i + j + 1, ai, u32_at(b, j + 1), carry);
                        carry = mac(&mut digit, i + j + 2, ai, u32_at(b, j + 2), carry);
                        carry = mac(&mut digit, i + j + 3, ai, u32_at(b, j + 3), carry);
                        j += 4;
                    }
                    digit[i + digits] = carry;
                }
                for i in 0..$limbs * 2 {
                    out[i] = digit[2 * i] | (digit[2 * i + 1] << 32);
                }
            }

            fn u32_at(words: &[u64; $limbs], index: usize) -> u64 {
                let word = words[index / 2];
                if index % 2 == 0 {
                    word & 0xffff_ffff
                } else {
                    word >> 32
                }
            }

            #[inline(always)]
            fn mac(digit: &mut [u64], index: usize, ai: u64, bj: u64, carry: u64) -> u64 {
                let sum = digit[index] + ai * bj + carry;
                digit[index] = sum & 0xffff_ffff;
                sum >> 32
            }

            fn remainder(product: &[u64; $limbs * 2], modulus: &[u64; $limbs]) -> [u64; $limbs] {
                let n = match super::significant_limbs(modulus) {
                    0 => return [0; $limbs],
                    n => n,
                };
                let shift = modulus[n - 1].leading_zeros();
                let mut v = [0u64; $limbs];
                v[..n].copy_from_slice(&modulus[..n]);
                let mut u = [0u64; $limbs * 2 + 1];
                u[..$limbs * 2].copy_from_slice(product);
                super::shl(&mut v[..n], shift);
                super::shl(&mut u, shift);

                if n == 1 {
                    let mut out = [0u64; $limbs];
                    super::rem_limb(&u, v[0], shift, &mut out);
                    return out;
                }

                let steps = u.len() - n;
                for j in (0..steps).rev() {
                    let top = u[j + n];
                    let hi = u[j + n - 1];
                    let (mut qhat, mut rhat) = if top == v[n - 1] {
                        (u64::MAX, u128::from(hi) + u128::from(v[n - 1]))
                    } else {
                        let (q, r) = super::div_rem_128_64(top, hi, v[n - 1]);
                        (q, u128::from(r))
                    };
                    while rhat < (1u128 << 64)
                        && u128::from(qhat) * u128::from(v[n - 2])
                            > (rhat << 64) | u128::from(u[j + n - 2])
                    {
                        qhat -= 1;
                        rhat += u128::from(v[n - 1]);
                    }
                    let window = &mut u[j..j + n + 1];
                    if super::mul_sub(window, &v[..n], qhat) {
                        super::add_back(window, &v[..n]);
                    }
                }

                let mut out = [0u64; $limbs];
                out[..n].copy_from_slice(&u[..n]);
                super::shr(&mut out, shift);
                out
            }
        }
    };
}

define_width!(w128, 2);
define_width!(w1024, 16);
define_width!(w2048, 32);
define_width!(w3072, 48);
define_width!(w4096, 64);

pub use w128::modexp as modexp_128;
pub use w1024::modexp as modexp_1024;
pub use w2048::modexp as modexp_2048;
pub use w3072::modexp as modexp_3072;
pub use w4096::modexp as modexp_4096;

fn div_rem_128_64(hi: u64, lo: u64, d: u64) -> (u64, u64) {
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
    let (q1, step) = quot_digit(u_hi, u_lo >> 32, vn1, vn0, v);
    let (q0, rem) = quot_digit(step, u_lo & 0xffff_ffff, vn1, vn0, v);
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

fn rem_limb(u: &[u64], divisor: u64, shift: u32, out: &mut [u64]) {
    let mut acc = 0u64;
    for limb in u.iter().rev() {
        acc = div_rem_128_64(acc, *limb, divisor).1;
    }
    out.fill(0);
    out[0] = acc;
    shr(out, shift);
}

fn significant_limbs(value: &[u64]) -> usize {
    value
        .iter()
        .rposition(|limb| *limb != 0)
        .map(|index| index + 1)
        .unwrap_or(0)
}

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
    u[v.len()] = u[v.len()].wrapping_add(carry);
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

pub fn modulus<const L: usize>() -> [u64; L] {
    let mut n = [0u64; L];
    let mut i = 0;
    while i < L {
        n[i] = 0xF0F0_F0F0_F0F0_F0F0 ^ ((i as u64) << 32) ^ (i as u64);
        i += 1;
    }
    n[0] |= 1;
    n[L - 1] |= 1 << 63;
    n
}

pub fn base<const L: usize>() -> [u64; L] {
    let n = modulus::<L>();
    let mut x = [0u64; L];
    let mut i = 0;
    while i < L {
        x[i] = 0x1234_5678_9ABC_DEF0 ^ n[i].rotate_left(7);
        i += 1;
    }
    x[L - 1] = n[L - 1];
    x[0] = n[0] - 1;
    x
}

pub fn checksum(limbs: &[u64]) -> u64 {
    limbs
        .iter()
        .enumerate()
        .fold(0, |acc, (index, limb)| acc ^ limb.rotate_left(index as u32))
}

/// 2048-bit inputs used by [`crate::pow3`]'s published measurement.
pub fn bench_2048() -> ([u64; LIMBS], [u64; LIMBS]) {
    (base::<LIMBS>(), modulus::<LIMBS>())
}
