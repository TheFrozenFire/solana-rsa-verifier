use verifier::{BENCH_N, BENCH_X, LIMBS, PRODUCT_LIMBS, modular, mont, schoolbook};

#[test]
fn n_prime_is_the_negated_inverse() {
    for n0 in [1u64, 3, 7, 9, u64::MAX, BENCH_N[0]] {
        assert_eq!(n0.wrapping_mul(mont::n_prime(n0)).wrapping_add(1), 0);
    }
}

#[test]
fn karatsuba_matches_schoolbook() {
    check_product(&[0; LIMBS], &[0; LIMBS]);
    check_product(&[1; LIMBS], &[2; LIMBS]);
    check_product(&[u64::MAX; LIMBS], &[u64::MAX; LIMBS]);
    check_product(&BENCH_X, &BENCH_N);
    let mut a = [0u64; LIMBS];
    let mut b = [0u64; LIMBS];
    for i in 0..LIMBS {
        a[i] = u64::MAX - i as u64;
        b[i] = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    }
    check_product(&a, &b);
}

#[test]
fn square_matches_schoolbook() {
    check_square(&[0; LIMBS]);
    check_square(&[1; LIMBS]);
    check_square(&[u64::MAX; LIMBS]);
    check_square(&BENCH_X);
}

#[test]
fn montgomery_cubes_match_schoolbook_division() {
    let samples = [
        limbs(6, 7),
        limbs(10, 7),
        limbs(2, 5),
        limbs(0, 7),
        limbs(100, 7),
        (BENCH_X, BENCH_N),
    ];
    for (x, n) in samples {
        let rr = mont::r_squared(&n);
        let n0 = mont::n_prime(n[0]);
        let mut want = [0u64; LIMBS];
        modular::cube(&mut want, &x, &n);
        for cube in [mont::cube_cios, mont::cube_karatsuba, mont::cube_square] {
            let mut got = [0u64; LIMBS];
            cube(&mut got, &x, &n, &rr, n0);
            assert_eq!(got, want);
            assert!(below(got, n));
        }
    }
}

fn limbs(x: u64, n: u64) -> ([u64; LIMBS], [u64; LIMBS]) {
    let mut x_limbs = [0u64; LIMBS];
    let mut n_limbs = [0u64; LIMBS];
    x_limbs[0] = x;
    n_limbs[0] = n;
    (x_limbs, n_limbs)
}

fn check_product(a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    let mut got = [0u64; PRODUCT_LIMBS];
    let mut want = [0u64; PRODUCT_LIMBS];
    mont::karatsuba(&mut got, a, b);
    schoolbook::mul(&mut want, a, b);
    assert_eq!(got, want);
}

fn check_square(a: &[u64; LIMBS]) {
    let mut got = [0u64; PRODUCT_LIMBS];
    let mut want = [0u64; PRODUCT_LIMBS];
    mont::square(&mut got, a);
    schoolbook::mul(&mut want, a, a);
    assert_eq!(got, want);
}

#[test]
fn bench_rr_matches_repeated_doubling() {
    assert_eq!(mont::r_squared(&BENCH_N), verifier::BENCH_RR);
}

fn below(value: [u64; LIMBS], n: [u64; LIMBS]) -> bool {
    for limb in (0..LIMBS).rev() {
        if value[limb] != n[limb] {
            return value[limb] < n[limb];
        }
    }
    false
}
