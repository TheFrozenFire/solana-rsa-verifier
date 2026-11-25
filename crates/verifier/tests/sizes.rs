use solana_rsa_verifier::bench::{
    self, modexp_128, modexp_1024, modexp_2048, modexp_3072, modexp_4096,
};
use solana_rsa_verifier::pow3;

#[test]
fn sized_cube_matches_bit_remainder() {
    let x = [6u64, 0];
    let n = [7u64, 0];
    assert_eq!(modexp_128(&x, &n, 3)[0], 6);
    let x = [10u64, 0];
    assert_eq!(modexp_128(&x, &n, 3)[0], 6);
    assert_eq!(modexp_128(&[0u64, 0], &n, 65537), [0, 0]);
    let got = modexp_128(&x, &n, 65537);
    assert_eq!(got, reference(&x, &n, 65537));
}

#[test]
fn wider_sizes_match_the_two_limb_result() {
    let expect3 = modexp_128(&[10, 0], &[7, 0], 3);
    let expect_e = modexp_128(&[10, 0], &[7, 0], 65537);
    assert_eq!(low2(&modexp_1024(&pad(10), &pad(7), 3)), expect3);
    assert_eq!(low2(&modexp_1024(&pad(10), &pad(7), 65537)), expect_e);
    assert_eq!(low2(&modexp_2048(&pad(10), &pad(7), 3)), expect3);
    assert_eq!(low2(&modexp_2048(&pad(10), &pad(7), 65537)), expect_e);
    assert_eq!(low2(&modexp_3072(&pad(10), &pad(7), 3)), expect3);
    assert_eq!(low2(&modexp_3072(&pad(10), &pad(7), 65537)), expect_e);
    assert_eq!(low2(&modexp_4096(&pad(10), &pad(7), 3)), expect3);
    assert_eq!(low2(&modexp_4096(&pad(10), &pad(7), 65537)), expect_e);
}

#[test]
fn sized_2048_cube_matches_pow3() {
    let (base, modulus) = bench::bench_2048();
    assert_eq!(modexp_2048(&base, &modulus, 3), pow3(&base, &modulus));
}

fn pad<const L: usize>(limb: u64) -> [u64; L] {
    let mut out = [0u64; L];
    out[0] = limb;
    out
}

fn low2(value: &[u64]) -> [u64; 2] {
    [value[0], value[1]]
}

fn reference(x: &[u64; 2], n: &[u64; 2], exponent: u32) -> [u64; 2] {
    let mut acc = [1u64, 0];
    let mut base = *x;
    let mut exp = exponent;
    while exp > 0 {
        if exp & 1 == 1 {
            acc = mul_mod(&acc, &base, n);
        }
        base = mul_mod(&base, &base, n);
        exp >>= 1;
    }
    acc
}

fn mul_mod(a: &[u64; 2], b: &[u64; 2], n: &[u64; 2]) -> [u64; 2] {
    let mut wide = [0u64; 4];
    for i in 0..2 {
        let mut carry = 0u128;
        for j in 0..2 {
            let product = u128::from(a[i]) * u128::from(b[j]) + u128::from(wide[i + j]) + carry;
            wide[i + j] = product as u64;
            carry = product >> 64;
        }
        wide[i + 2] = carry as u64;
    }
    let mut acc = [0u64; 3];
    for bit in (0..128).rev() {
        let mut carry = 0u64;
        for limb in acc.iter_mut() {
            let next = *limb >> 63;
            *limb = (*limb << 1) | carry;
            carry = next;
        }
        if (wide[bit / 64] >> (bit % 64)) & 1 == 1 {
            acc[0] |= 1;
        }
        if acc[2] != 0 || acc[1] > n[1] || (acc[1] == n[1] && acc[0] >= n[0]) {
            let (d0, b0) = acc[0].overflowing_sub(n[0]);
            let (d1, b1) = acc[1].overflowing_sub(n[1] + u64::from(b0));
            acc[0] = d0;
            acc[1] = d1;
            acc[2] = acc[2].wrapping_sub(u64::from(b1));
        }
    }
    [acc[0], acc[1]]
}
