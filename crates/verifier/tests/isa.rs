use verifier::{BENCH_A, BENCH_B, BENCH_N, BENCH_X, LIMBS, PRODUCT_LIMBS, modular, schoolbook};

#[test]
fn inlined_product_matches_schoolbook() {
    let mut got = [0u64; PRODUCT_LIMBS];
    let mut want = [0u64; PRODUCT_LIMBS];
    schoolbook::mul_wide(&mut got, &BENCH_A, &BENCH_B);
    schoolbook::mul(&mut want, &BENCH_A, &BENCH_B);
    assert_eq!(got, want);
    let mut a = [0u64; LIMBS];
    let mut b = [0u64; LIMBS];
    a[0] = 6;
    b[0] = 7;
    schoolbook::mul_wide(&mut got, &a, &b);
    schoolbook::mul(&mut want, &a, &b);
    assert_eq!(got, want);
    a[0] = u64::MAX;
    b[0] = u64::MAX;
    schoolbook::mul_wide(&mut got, &a, &b);
    schoolbook::mul(&mut want, &a, &b);
    assert_eq!(got, want);
}

#[test]
fn unrolled_mac_matches_schoolbook() {
    let mut got = [0u64; PRODUCT_LIMBS];
    let mut want = [0u64; PRODUCT_LIMBS];
    for (a, b) in [
        ([0; LIMBS], [0; LIMBS]),
        (BENCH_A, BENCH_B),
        (BENCH_X, BENCH_N),
    ] {
        schoolbook::mul32_mac(&mut got, &a, &b);
        schoolbook::mul(&mut want, &a, &b);
        assert_eq!(got, want);
    }
}

#[test]
fn remainder32_cube_matches_schoolbook_cube() {
    let mut got = [0u64; LIMBS];
    let mut want = [0u64; LIMBS];
    for (x0, n0) in [(6u64, 7u64), (10, 7), (100, 7), (0, 9), (4, 5)] {
        let mut x = [0u64; LIMBS];
        let mut n = [0u64; LIMBS];
        x[0] = x0;
        n[0] = n0;
        modular::cube_rem32(&mut got, &x, &n);
        modular::cube(&mut want, &x, &n);
        assert_eq!(got, want, "{x0} {n0}");
    }
    modular::cube_rem32(&mut got, &BENCH_X, &BENCH_N);
    modular::cube(&mut want, &BENCH_X, &BENCH_N);
    assert_eq!(got, want);
}

#[test]
fn limb32_product_matches_schoolbook() {
    let mut got = [0u64; PRODUCT_LIMBS];
    let mut want = [0u64; PRODUCT_LIMBS];
    for (a, b) in [
        ([0; LIMBS], [0; LIMBS]),
        (BENCH_A, BENCH_B),
        (BENCH_X, BENCH_N),
    ] {
        schoolbook::mul32(&mut got, &a, &b);
        schoolbook::mul(&mut want, &a, &b);
        assert_eq!(got, want);
    }
}

#[test]
fn div_rem_matches_u128_division() {
    let values = [
        0u64,
        1,
        2,
        3,
        7,
        0xffff_ffff,
        0x1_0000_0000,
        0x8000_0000_0000_0000,
        u64::MAX - 1,
        u64::MAX,
    ];
    for hi in values {
        for lo in values {
            for d in values {
                if d == 0 || hi >= d {
                    continue;
                }
                let numerator = (u128::from(hi) << 64) | u128::from(lo);
                let (q, r) = modular::div_rem_128_64(hi, lo, d);
                assert_eq!(u128::from(q), numerator / u128::from(d), "{hi} {lo} {d}");
                assert_eq!(u128::from(r), numerator % u128::from(d), "{hi} {lo} {d}");
            }
        }
    }
}

#[test]
fn quotient_cube_matches_schoolbook_cube() {
    let mut got = [0u64; LIMBS];
    let mut want = [0u64; LIMBS];
    for (x0, n0) in [(6u64, 7u64), (10, 7), (100, 7), (0, 9)] {
        let mut x = [0u64; LIMBS];
        let mut n = [0u64; LIMBS];
        x[0] = x0;
        n[0] = n0;
        modular::cube_quot(&mut got, &x, &n);
        modular::cube(&mut want, &x, &n);
        assert_eq!(got, want, "{x0} {n0}");
    }
    modular::cube_quot(&mut got, &BENCH_X, &BENCH_N);
    modular::cube(&mut want, &BENCH_X, &BENCH_N);
    assert_eq!(got, want);
}
