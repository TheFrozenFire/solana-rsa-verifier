use solana_instruction::Instruction;
use solana_program_test::{ProgramTest, tokio};
use solana_pubkey::Pubkey;
use solana_rsa_verifier::{LIMBS, pow3};
use solana_signer::Signer;
use solana_transaction::Transaction;

#[tokio::test]
async fn measures_pow3() {
    let program_id = Pubkey::new_unique();
    let mut test = ProgramTest::default();
    test.prefer_bpf(true);
    test.add_program("spike", program_id, None);
    test.set_compute_max_units(1_400_000);
    let context = test.start_with_context().await;

    let transaction = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id,
            accounts: vec![],
            data: vec![],
        }],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );
    let result = context
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .expect("pow3 failed to run");
    assert!(result.result.is_ok(), "{:?}", result.result);
    let metadata = result.metadata.expect("transaction metadata");
    let return_data = metadata.return_data.expect("return data").data;
    let cu = u64::from_le_bytes(return_data[..8].try_into().unwrap());
    let digest = u64::from_le_bytes(return_data[8..].try_into().unwrap());
    let expected = pow3(&bench_base(), &bench_modulus());
    let want = expected
        .iter()
        .enumerate()
        .fold(0, |acc, (index, limb)| acc ^ limb.rotate_left(index as u32));
    assert_eq!(digest, want);
    eprintln!(
        "pow3: {cu} compute units (transaction consumed {})",
        metadata.compute_units_consumed
    );
}

fn bench_modulus() -> [u64; LIMBS] {
    let mut n = [0u64; LIMBS];
    for i in 0..LIMBS {
        n[i] = 0xF0F0_F0F0_F0F0_F0F0 ^ ((i as u64) << 32) ^ (i as u64);
    }
    n[0] |= 1;
    n[LIMBS - 1] |= 1 << 63;
    n
}

fn bench_base() -> [u64; LIMBS] {
    let n = bench_modulus();
    let mut x = [0u64; LIMBS];
    for i in 0..LIMBS {
        x[i] = 0x1234_5678_9ABC_DEF0 ^ n[i].rotate_left(7);
    }
    x[LIMBS - 1] = n[LIMBS - 1];
    x[0] = n[0] - 1;
    x
}
