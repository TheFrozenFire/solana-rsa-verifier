use verifier::{BENCH_A, BENCH_B, CANDIDATES, LIMBS, PRODUCT_LIMBS, checksum, schoolbook};

#[test]
fn candidate_ids_are_the_spike_instruction_bytes() {
    assert_eq!(CANDIDATES[0].name, "schoolbook");
    assert_eq!(CANDIDATES[0].id, 0);
    assert_eq!(CANDIDATES[1].name, "noop");
    assert_eq!(CANDIDATES[1].id, 1);
    assert_eq!(CANDIDATES[2].name, "cube");
    assert_eq!(CANDIDATES[2].id, 2);
    assert_eq!(CANDIDATES[3].name, "cios");
    assert_eq!(CANDIDATES[3].id, 3);
    assert_eq!(CANDIDATES[4].name, "karatsuba");
    assert_eq!(CANDIDATES[4].id, 4);
    assert_eq!(CANDIDATES[5].name, "square");
    assert_eq!(CANDIDATES[5].id, 5);
    assert_eq!(CANDIDATES[6].name, "wide");
    assert_eq!(CANDIDATES[6].id, 6);
    assert_eq!(CANDIDATES[7].name, "quot");
    assert_eq!(CANDIDATES[7].id, 7);
    assert_eq!(CANDIDATES[8].name, "limb32");
    assert_eq!(CANDIDATES[8].id, 8);
}

#[test]
fn schoolbook_matches_base_2_32_reference() {
    let mut a = [0u64; LIMBS];
    let mut b = [0u64; LIMBS];
    a[0] = 6;
    b[0] = 7;
    expect(&a, &b);

    a[0] = u64::MAX;
    b[0] = u64::MAX;
    expect(&a, &b);

    for i in 0..LIMBS {
        a[i] = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        b[i] = u64::MAX - i as u64;
    }
    expect(&a, &b);
    expect(&BENCH_A, &BENCH_B);
}

fn expect(a: &[u64; LIMBS], b: &[u64; LIMBS]) {
    let mut got = [0u64; PRODUCT_LIMBS];
    schoolbook::mul(&mut got, a, b);
    assert_eq!(got, reference(a, b), "checksum {:#x}", checksum(&got));
}

/// Independent product in base 2^32, where each partial product fits in a `u64`.
fn reference(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> [u64; PRODUCT_LIMBS] {
    let mut left = [0u32; LIMBS * 2];
    let mut right = [0u32; LIMBS * 2];
    for i in 0..LIMBS {
        left[2 * i] = a[i] as u32;
        left[2 * i + 1] = (a[i] >> 32) as u32;
        right[2 * i] = b[i] as u32;
        right[2 * i + 1] = (b[i] >> 32) as u32;
    }

    let mut digits = [0u64; LIMBS * 4 + 1];
    for i in 0..left.len() {
        let mut carry = 0u64;
        for j in 0..right.len() {
            let sum = digits[i + j] + carry + u64::from(left[i]) * u64::from(right[j]);
            digits[i + j] = sum & 0xffff_ffff;
            carry = sum >> 32;
        }
        let mut k = i + right.len();
        while carry != 0 {
            let sum = digits[k] + carry;
            digits[k] = sum & 0xffff_ffff;
            carry = sum >> 32;
            k += 1;
        }
    }

    let mut packed = [0u64; PRODUCT_LIMBS];
    for i in 0..PRODUCT_LIMBS {
        packed[i] = digits[2 * i] | (digits[2 * i + 1] << 32);
    }
    assert_eq!(digits[LIMBS * 4], 0);
    packed
}
