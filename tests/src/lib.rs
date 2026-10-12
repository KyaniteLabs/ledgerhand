//! Integration tests for the trust_layer program using solana-program-test.
//! Proves the core loop: issue -> verify OK -> tamper -> verify FAIL,
//! plus negative cases (unknown hash, double-issue, non-authority void,
//! authority void happy path).
//!
//! Instructions are encoded the same way a real client does: 8-byte Anchor
//! sighash discriminator + borsh-serialized args.
//!
//! Run from repo root: `cargo test --manifest-path tests/Cargo.toml`
//! (no validator, no Solana CLI — pure cargo.)

use anchor_lang::AccountDeserialize;
use borsh::BorshDeserialize;
use sha2::{Digest, Sha256};
use solana_program_test::*;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use trust_layer::{Receipt, ID as PROGRAM_ID};

/// Payload used across the demo beats: a small "delivery manifest".
const FILE_A: &[u8] = b"{\"deliverable\":\"q3-report.pdf\",\"from\":\"acme\",\"to\":\"globex\",\"amount\":1200}";

fn hash_of(bytes: &[u8]) -> [u8; 32] {
    let h = Sha256::digest(bytes);
    h.into()
}

fn receipt_pda(payload_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"receipt", payload_hash.as_ref()], &PROGRAM_ID)
}

/// Anchor global-namespace sighash: first 8 bytes of sha256("namespace:name").
fn sighash(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    let digest = Sha256::digest(preimage.as_bytes());
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

/// Encode issue_receipt(payload_hash: [u8;32], memo: String) exactly as the
/// Anchor TS client would.
fn encode_issue(payload_hash: &[u8; 32], memo: &str) -> Vec<u8> {
    let mut data = sighash("issue_receipt").to_vec();
    data.extend_from_slice(payload_hash);
    // borsh string: little-endian u32 length + utf8 bytes
    data.extend_from_slice(&(memo.len() as u32).to_le_bytes());
    data.extend_from_slice(memo.as_bytes());
    data
}

/// Encode verify_receipt(payload_hash: [u8;32]).
fn encode_verify(payload_hash: &[u8; 32]) -> Vec<u8> {
    let mut data = sighash("verify_receipt").to_vec();
    data.extend_from_slice(payload_hash);
    data
}

/// Encode void_receipt().
fn encode_void() -> Vec<u8> {
    sighash("void_receipt").to_vec()
}

async fn setup() -> ProgramTestContext {
    // SBF mode: load the real compiled trust_layer.so (same artifact that
    // deploys to devnet) from tests/fixtures/, found via SBF_OUT_DIR.
    let pt = ProgramTest::new("trust_layer", PROGRAM_ID, None);
    pt.start_with_context().await
}

/// Fund `to` from the test payer so it can pay for account creation.
async fn fund(ctx: &ProgramTestContext, payer: &Keypair, to: &Keypair) {
    let ix = solana_sdk::system_instruction::transfer(&payer.pubkey(), &to.pubkey(), 5_000_000_000);
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&payer.pubkey()), &[payer], ctx.last_blockhash);
    ctx.banks_client.process_transaction(tx).await.expect("fund tx");
}

fn issue_ix(issuer: &Pubkey, payload_hash: &[u8; 32], memo: &str) -> Instruction {
    let (pda, _) = receipt_pda(payload_hash);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(pda, false),
            AccountMeta::new(*issuer, true),
            AccountMeta::new_readonly(solana_sdk::system_program::ID, false),
        ],
        data: encode_issue(payload_hash, memo),
    }
}

/// Build the REAL on-chain verify instruction for a queried hash.
fn verify_ix(payload_hash: &[u8; 32]) -> Instruction {
    let (pda, _) = receipt_pda(payload_hash);
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(pda, false)],
        data: encode_verify(payload_hash),
    }
}

/// Mirror of the program's VerificationResult for decoding return data.
/// Anchor return data = borsh payload WITHOUT discriminator.
#[derive(BorshDeserialize, Debug)]
struct Verification {
    matches: bool,
    receipt: Option<Pubkey>,
    issuer: Option<Pubkey>,
    slot: Option<u64>,
    voided: bool,
}

/// Run verify_receipt on-chain (simulated so we can read return data) and
/// decode the returned issuer + slot + match boolean.
async fn verify_on_chain(ctx: &mut ProgramTestContext, payload_hash: &[u8; 32]) -> Verification {
    let tx = Transaction::new_signed_with_payer(
        &[verify_ix(payload_hash)],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    let sim = ctx.banks_client.simulate_transaction(tx).await.expect("simulate verify");
    let details = sim.simulation_details.as_ref().expect("simulation details");
    assert!(
        sim.result.is_none() || sim.result.as_ref().unwrap().is_ok(),
        "verify tx must not itself fail: result={:?} logs={:?}",
        sim.result,
        details.logs
    );
    let rd = details.return_data.as_ref().expect("verify must return data");
    assert_eq!(rd.program_id, PROGRAM_ID);
    Verification::try_from_slice(&rd.data).expect("decode VerificationResult")
}

#[tokio::test]
async fn core_loop_issue_verify_tamper() {
    let mut ctx = setup().await;
    let issuer = Keypair::new();
    fund(&ctx, &ctx.payer.insecure_clone(), &issuer).await;

    // ---- BEAT 1: issue a receipt for the untouched payload ----
    let hash_a = hash_of(FILE_A);
    let tx = Transaction::new_signed_with_payer(
        &[issue_ix(&issuer.pubkey(), &hash_a, "delivery receipt demo")],
        Some(&issuer.pubkey()),
        &[&issuer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.expect("issue_receipt tx");

    let (pda, _) = receipt_pda(&hash_a);
    let account = ctx.banks_client.get_account(pda).await.unwrap().expect("receipt account exists");
    let receipt = Receipt::try_deserialize(&mut account.data.as_slice()).unwrap();
    assert_eq!(receipt.payload_hash, hash_a, "stored hash must equal issued hash");
    assert_eq!(receipt.issuer, issuer.pubkey(), "issuer must be the signer");
    assert!(receipt.slot > 0, "slot must be recorded");
    assert_eq!(receipt.memo, "delivery receipt demo");
    assert!(!receipt.is_voided);

    // ---- BEAT 2: verify the untouched payload -> MATCH, with issuer + slot ----
    let vr = verify_on_chain(&mut ctx, &hash_a).await;
    assert!(vr.matches, "untampered payload must verify (matches=true)");
    assert_eq!(vr.issuer.expect("issuer returned"), issuer.pubkey());
    assert!(vr.slot.expect("slot returned") > 0);
    assert_eq!(vr.receipt.expect("receipt returned"), pda);

    // ---- BEAT 3: tamper (flip ONE byte) -> on-chain verify FAILS loudly ----
    let mut tampered = FILE_A.to_vec();
    tampered[10] ^= 0x01;
    assert_ne!(&tampered, FILE_A, "sanity: payload actually changed");
    let hash_t = hash_of(&tampered);
    assert_ne!(hash_t, hash_a, "sanity: hash changed after one-byte flip");

    // The tampered hash derives a DIFFERENT PDA with no account behind it...
    let (t_pda, _) = receipt_pda(&hash_t);
    assert_ne!(t_pda, pda);
    let missing = ctx.banks_client.get_account(t_pda).await.unwrap();
    assert!(missing.is_none(), "no receipt may exist for a tampered payload hash");

    // ...so the real verify instruction FAILS (seeds constraint: no account).
    let tx = Transaction::new_signed_with_payer(
        &[verify_ix(&hash_t)],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    let err = ctx.banks_client.process_transaction(tx).await;
    assert!(err.is_err(), "TAMPER DETECTED: verify of tampered hash must fail on-chain");

    eprintln!("CORE LOOP GREEN: issue -> verify(true, issuer, slot) -> tamper -> verify FAIL");
}

#[tokio::test]
async fn unknown_hash_has_no_receipt() {
    let mut ctx = setup().await;
    let random_hash = hash_of(b"never-issued payload");
    let (pda, _) = receipt_pda(&random_hash);
    let acc = ctx.banks_client.get_account(pda).await.unwrap();
    assert!(acc.is_none(), "unknown hash must have no account");
    let tx = Transaction::new_signed_with_payer(
        &[verify_ix(&random_hash)],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        ctx.last_blockhash,
    );
    assert!(
        ctx.banks_client.process_transaction(tx).await.is_err(),
        "verify of unknown hash must fail"
    );
}

#[tokio::test]
async fn double_issue_is_rejected() {
    let mut ctx = setup().await;
    let issuer = Keypair::new();
    fund(&ctx, &ctx.payer.insecure_clone(), &issuer).await;

    let hash = hash_of(b"idempotency-check");
    let tx1 = Transaction::new_signed_with_payer(
        &[issue_ix(&issuer.pubkey(), &hash, "first")],
        Some(&issuer.pubkey()),
        &[&issuer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx1).await.expect("first issue ok");

    let tx2 = Transaction::new_signed_with_payer(
        &[issue_ix(&issuer.pubkey(), &hash, "second")],
        Some(&issuer.pubkey()),
        &[&issuer],
        ctx.last_blockhash,
    );
    let err = ctx.banks_client.process_transaction(tx2).await;
    assert!(err.is_err(), "re-issuing the same payload hash must be rejected");
}

#[tokio::test]
async fn non_authority_cannot_void() {
    let mut ctx = setup().await;
    let issuer = Keypair::new();
    fund(&ctx, &ctx.payer.insecure_clone(), &issuer).await;

    let hash = hash_of(b"void-auth-check");
    let tx = Transaction::new_signed_with_payer(
        &[issue_ix(&issuer.pubkey(), &hash, "to be voided")],
        Some(&issuer.pubkey()),
        &[&issuer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.expect("issue ok");
    let (pda, _) = receipt_pda(&hash);

    let attacker = Keypair::new();
    fund(&ctx, &ctx.payer.insecure_clone(), &attacker).await;

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(pda, false),
            AccountMeta::new_readonly(attacker.pubkey(), true),
        ],
        data: encode_void(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&attacker.pubkey()), &[&attacker], ctx.last_blockhash);
    let err = ctx.banks_client.process_transaction(tx).await;
    assert!(err.is_err(), "non-authority void must be rejected");
}

/// Voiding by the real authority works and closes the account.
#[tokio::test]
async fn authority_can_void() {
    let mut ctx = setup().await;
    let issuer = Keypair::new();
    fund(&ctx, &ctx.payer.insecure_clone(), &issuer).await;

    let hash = hash_of(b"void-happy-path");
    let tx = Transaction::new_signed_with_payer(
        &[issue_ix(&issuer.pubkey(), &hash, "temp")],
        Some(&issuer.pubkey()),
        &[&issuer],
        ctx.last_blockhash,
    );
    ctx.banks_client.process_transaction(tx).await.expect("issue ok");
    let (pda, _) = receipt_pda(&hash);

    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(pda, false),
            AccountMeta::new(issuer.pubkey(), true),
        ],
        data: encode_void(),
    };
    let tx = Transaction::new_signed_with_payer(&[ix], Some(&issuer.pubkey()), &[&issuer], ctx.last_blockhash);
    ctx.banks_client.process_transaction(tx).await.expect("authority void ok");
    let acc = ctx.banks_client.get_account(pda).await.unwrap();
    assert!(acc.is_none(), "voided receipt account must be closed");
}
