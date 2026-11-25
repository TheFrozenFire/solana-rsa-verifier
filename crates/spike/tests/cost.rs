use solana_instruction::Instruction;
use solana_program_test::{ProgramTest, tokio};
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use solana_transaction::Transaction;
use verifier::{
    BENCH_A, BENCH_B, BENCH_N, BENCH_X, CANDIDATES, LIMBS, PRODUCT_LIMBS, checksum, modular, noop,
    schoolbook,
};

#[tokio::test]
async fn measures_each_candidate() {
    let program_id = Pubkey::new_unique();
    let mut test = ProgramTest::default();
    test.prefer_bpf(true);
    test.add_program("spike", program_id, None);
    test.set_compute_max_units(1_400_000);
    let context = test.start_with_context().await;

    let mut expected = [0u64; PRODUCT_LIMBS];
    schoolbook::mul(&mut expected, &BENCH_A, &BENCH_B);
    let schoolbook_digest = checksum(&expected);
    let mut empty = [0u64; PRODUCT_LIMBS];
    noop(&mut empty, &BENCH_A, &BENCH_B);
    let mut cube_limbs = [0u64; LIMBS];
    modular::cube(&mut cube_limbs, &BENCH_X, &BENCH_N);
    let mut cube_padded = [0u64; PRODUCT_LIMBS];
    cube_padded[..LIMBS].copy_from_slice(&cube_limbs);

    let mut charged = [0u64; 11];
    for candidate in CANDIDATES {
        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![],
                data: vec![candidate.id],
            }],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        let result = context
            .banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap_or_else(|error| panic!("{} failed to run: {error}", candidate.name));
        assert!(
            result.result.is_ok(),
            "{name} rejected: {result:?}",
            name = candidate.name
        );
        let metadata = result.metadata.expect("transaction metadata");
        let return_data = metadata.return_data.expect("return data").data;
        assert_eq!(return_data.len(), 16, "{name}", name = candidate.name);
        let cu = u64::from_le_bytes(return_data[..8].try_into().unwrap());
        let digest = u64::from_le_bytes(return_data[8..].try_into().unwrap());
        let want = match candidate.id {
            0 | 6 | 8 | 9 => schoolbook_digest,
            1 => checksum(&empty),
            _ => checksum(&cube_padded),
        };
        assert_eq!(digest, want, "{name}", name = candidate.name);
        charged[candidate.id as usize] = cu;
        eprintln!(
            "{name}: {cu} compute units (transaction consumed {total})",
            name = candidate.name,
            total = metadata.compute_units_consumed
        );
    }

    assert!(
        charged[0] > charged[1],
        "schoolbook ({}) should cost more than the empty candidate ({})",
        charged[0],
        charged[1]
    );
    assert!(
        charged[2] > charged[0],
        "cube ({}) should cost more than schoolbook multiplication ({})",
        charged[2],
        charged[0]
    );
}
