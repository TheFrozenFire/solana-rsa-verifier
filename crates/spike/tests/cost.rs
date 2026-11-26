#[path = "data/samples.rs"]
mod samples;

use solana_instruction::Instruction;
use solana_program_test::{ProgramTest, tokio};
use solana_pubkey::Pubkey;
use solana_rsa_verifier::bench::{
    self, checksum, modexp_1024, modexp_2048, modexp_3072, modexp_4096,
};
use solana_rsa_verifier::pow3;
use solana_signer::Signer;
use solana_transaction::Transaction;

#[tokio::test]
async fn measures_exponent_65537_without_the_transaction_cap() {
    let program_id = Pubkey::new_unique();
    let mut test = ProgramTest::default();
    test.prefer_bpf(true);
    test.add_program("spike", program_id, None);
    // The on-chain transaction limit is 1_400_000. This budget exists only so
    // the test can read the full cost of an operation that does not fit.
    test.set_compute_max_units(20_000_000);
    let context = test.start_with_context().await;

    for (size, bits) in [(1u8, 2048u32), (2, 3072), (3, 4096)] {
        let label = format!("{bits}-bit e=65537");
        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![],
                data: vec![size, 1],
            }],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        let result = context
            .banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap_or_else(|error| panic!("{label} failed to run: {error}"));
        let metadata = result.metadata.expect("metadata");
        result
            .result
            .unwrap_or_else(|error| panic!("{label} failed: {error:?}"));
        let return_data = metadata.return_data.expect("return data").data;
        let cu = u64::from_le_bytes(return_data[..8].try_into().unwrap());
        let digest = u64::from_le_bytes(return_data[8..].try_into().unwrap());
        assert_eq!(digest, host_digest(bits, 65537), "{label}");
        eprintln!(
            "{label}: {cu} compute units (transaction consumed {})",
            metadata.compute_units_consumed
        );
    }
}

#[tokio::test]
async fn measures_key_sizes_and_exponents() {
    let program_id = Pubkey::new_unique();
    let mut test = ProgramTest::default();
    test.prefer_bpf(true);
    test.add_program("spike", program_id, None);
    test.set_compute_max_units(1_400_000);
    let context = test.start_with_context().await;

    for (size, bits, exponent_code, exponent) in [
        (0u8, 1024, 0u8, 3u32),
        (0, 1024, 1, 65537),
        (1, 2048, 0, 3),
        (1, 2048, 1, 65537),
        (2, 3072, 0, 3),
        (2, 3072, 1, 65537),
        (3, 4096, 0, 3),
        (3, 4096, 1, 65537),
    ] {
        let label = format!("{bits}-bit e={exponent}");
        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![],
                data: vec![size, exponent_code],
            }],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        let result = context
            .banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap_or_else(|error| panic!("{label} failed to run: {error}"));
        let Some(metadata) = result.metadata else {
            panic!("{label} has no metadata");
        };
        if let Err(error) = result.result {
            eprintln!(
                "{label}: did not finish ({error:?}, transaction consumed {})",
                metadata.compute_units_consumed
            );
            continue;
        }
        let return_data = metadata.return_data.expect("return data").data;
        let cu = u64::from_le_bytes(return_data[..8].try_into().unwrap());
        let digest = u64::from_le_bytes(return_data[8..].try_into().unwrap());
        let want = host_digest(bits, exponent);
        assert_eq!(digest, want, "{label}");
        eprintln!(
            "{label}: {cu} compute units (transaction consumed {})",
            metadata.compute_units_consumed
        );
    }

    for (label, opcode, modulus, signature, digest) in [
        (
            "2048-bit PKCS#1 v1.5 SHA-256 verify",
            0x10,
            samples::MODULUS_2048,
            samples::N2048_SHA256_SIGNATURE,
            samples::N2048_SHA256_DIGEST,
        ),
        (
            "2048-bit PSS verify",
            0x11,
            samples::MODULUS_2048,
            samples::PSS_2048_SIGNATURE,
            samples::PSS_2048_DIGEST,
        ),
        (
            "3072-bit PKCS#1 v1.5 SHA-256 verify",
            0x10,
            samples::MODULUS_3072,
            samples::N3072_SHA256_SIGNATURE,
            samples::N3072_SHA256_DIGEST,
        ),
        (
            "3072-bit PSS verify",
            0x11,
            samples::MODULUS_3072,
            samples::PSS_3072_SIGNATURE,
            samples::PSS_3072_DIGEST,
        ),
        (
            "4096-bit PKCS#1 v1.5 SHA-256 verify",
            0x10,
            samples::MODULUS_4096,
            samples::N4096_SHA256_SIGNATURE,
            samples::N4096_SHA256_DIGEST,
        ),
        (
            "4096-bit PSS verify",
            0x11,
            samples::MODULUS_4096,
            samples::PSS_4096_SIGNATURE,
            samples::PSS_4096_DIGEST,
        ),
    ] {
        let mut data = vec![opcode];
        data.extend(decode(modulus));
        data.extend(decode(signature));
        data.extend(decode(digest));
        let transaction = Transaction::new_signed_with_payer(
            &[Instruction {
                program_id,
                accounts: vec![],
                data,
            }],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        let result = context
            .banks_client
            .process_transaction_with_metadata(transaction)
            .await
            .unwrap_or_else(|error| panic!("{label} failed to run: {error}"));
        let metadata = result.metadata.expect("metadata");
        result
            .result
            .unwrap_or_else(|error| panic!("{label} failed: {error:?}"));
        let return_data = metadata.return_data.expect("return data").data;
        let cu = u64::from_le_bytes(return_data[..8].try_into().unwrap());
        let ok = u64::from_le_bytes(return_data[8..].try_into().unwrap());
        assert_eq!(ok, 1, "{label}");
        eprintln!(
            "{label}: {cu} compute units (transaction consumed {})",
            metadata.compute_units_consumed
        );
    }
}

fn decode(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn host_digest(bits: u32, exponent: u32) -> u64 {
    match (bits, exponent) {
        (1024, exp) => checksum(&modexp_1024(
            &bench::base::<16>(),
            &bench::modulus::<16>(),
            exp,
        )),
        (2048, 3) => {
            let (base, modulus) = bench::bench_2048();
            checksum(&pow3(&base, &modulus))
        }
        (2048, exp) => checksum(&modexp_2048(
            &bench::base::<32>(),
            &bench::modulus::<32>(),
            exp,
        )),
        (3072, exp) => checksum(&modexp_3072(
            &bench::base::<48>(),
            &bench::modulus::<48>(),
            exp,
        )),
        (4096, exp) => checksum(&modexp_4096(
            &bench::base::<64>(),
            &bench::modulus::<64>(),
            exp,
        )),
        _ => panic!("unknown size"),
    }
}
