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

fn add_carry(out: &mut [u64; PRODUCT_LIMBS], index: usize, mut extra: u128) {
    let mut index = index;
    while extra != 0 {
        let (sum, overflow) = out[index].overflowing_add(extra as u64);
        out[index] = sum;
        extra = (extra >> 64) + u128::from(overflow);
        index += 1;
    }
}
