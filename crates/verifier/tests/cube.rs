use verifier::{LIMBS, modular};

#[test]
fn cube_matches_known_residues() {
    expect(6, 7, 6);
    expect(10, 7, 6);
    expect(2, 5, 3);
    expect(4, 5, 4);
    expect(0, 7, 0);
}

#[test]
fn cube_matches_binary_reduction() {
    let mut x = [0u64; LIMBS];
    let mut n = [0u64; LIMBS];
    x[1] = 1 << 36;
    n[1] = 1 << 16;
    n[0] = 13;
    expect_limbs(&x, &n);

    let mut step = 0x9E37_79B9_7F4A_7C15u64;
    for limb in 0..LIMBS {
        step = step.wrapping_mul(0x6C62_272E_07BB_0142).wrapping_add(1);
        x[limb] = step;
        n[limb] = step.rotate_left(17) | 1;
    }
    n[0] |= 1;
    n[LIMBS - 1] |= 1 << 63;
    x[LIMBS - 1] = n[LIMBS - 1];
    x[0] = n[0] - 1;
    expect_limbs(&x, &n);
    expect_limbs(&verifier::BENCH_X, &verifier::BENCH_N);
}

fn expect(x: u64, n: u64, residue: u64) {
    let mut x_limbs = [0u64; LIMBS];
    let mut n_limbs = [0u64; LIMBS];
    x_limbs[0] = x;
    n_limbs[0] = n;
    let mut got = [0u64; LIMBS];
    modular::cube(&mut got, &x_limbs, &n_limbs);
    assert_eq!(got[0], residue);
    assert!(got[1..].iter().all(|limb| *limb == 0));
}

fn expect_limbs(x: &[u64; LIMBS], n: &[u64; LIMBS]) {
    let mut got = [0u64; LIMBS];
    modular::cube(&mut got, x, n);
    assert_eq!(got, binary_cube(x, n));
    assert!(below(got, *n));
}

fn binary_cube(x: &[u64; LIMBS], n: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut wide = [0u64; LIMBS * 2];
    verifier::schoolbook::mul(&mut wide, x, x);
    let square = binary_rem(&wide, n);
    verifier::schoolbook::mul(&mut wide, &square, x);
    binary_rem(&wide, n)
}

/// Remainder of a 4096-bit product by shifting one bit at a time.
fn binary_rem(product: &[u64; LIMBS * 2], n: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut acc = [0u64; LIMBS + 1];
    for bit in (0..LIMBS * 128).rev() {
        shl1(&mut acc);
        if (product[bit / 64] >> (bit % 64)) & 1 == 1 {
            acc[0] |= 1;
        }
        if acc_at_least_n(&acc, n) {
            sub_n(&mut acc, n);
        }
    }
    let mut out = [0u64; LIMBS];
    out.copy_from_slice(&acc[..LIMBS]);
    assert_eq!(acc[LIMBS], 0);
    out
}

fn shl1(acc: &mut [u64; LIMBS + 1]) {
    let mut carry = 0u64;
    for limb in acc.iter_mut() {
        let next = *limb >> 63;
        *limb = (*limb << 1) | carry;
        carry = next;
    }
}

fn acc_at_least_n(acc: &[u64; LIMBS + 1], n: &[u64; LIMBS]) -> bool {
    if acc[LIMBS] != 0 {
        return true;
    }
    for limb in (0..LIMBS).rev() {
        if acc[limb] != n[limb] {
            return acc[limb] > n[limb];
        }
    }
    true
}

fn sub_n(acc: &mut [u64; LIMBS + 1], n: &[u64; LIMBS]) {
    let mut borrow = false;
    for limb in 0..LIMBS {
        let (diff, b1) = acc[limb].overflowing_sub(n[limb]);
        let (diff, b2) = diff.overflowing_sub(u64::from(borrow));
        acc[limb] = diff;
        borrow = b1 || b2;
    }
    let (diff, b) = acc[LIMBS].overflowing_sub(u64::from(borrow));
    acc[LIMBS] = diff;
    assert!(!b);
}

fn below(value: [u64; LIMBS], n: [u64; LIMBS]) -> bool {
    for limb in (0..LIMBS).rev() {
        if value[limb] != n[limb] {
            return value[limb] < n[limb];
        }
    }
    false
}
