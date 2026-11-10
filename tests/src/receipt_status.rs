//! Host-side tests for the receipt lifecycle (`trust_layer::ReceiptStatus`).
//!
//! These run the program's own Rust (not the SBF fixture) and pin:
//! - the status <-> persisted `is_voided` byte table, both ways;
//! - every legal transition (`issue`, `void`), and that `Voided` is terminal;
//! - the verify verdict table, including that `matches && voided` never occurs;
//! - the on-disk account bytes, so the layout stays compatible;
//! - a model-based property test over random operation sequences.

use anchor_lang::{AccountDeserialize, AccountSerialize};
use proptest::prelude::*;
use solana_sdk::pubkey::Pubkey;
use trust_layer::{Receipt, ReceiptStatus, VerificationResult};

const HASH: [u8; 32] = [7u8; 32];
const OTHER_HASH: [u8; 32] = [9u8; 32];

/// Exhaustive by construction: adding a `ReceiptStatus` variant without
/// deciding its persisted flag here fails to compile.
fn expected_flag(status: ReceiptStatus) -> bool {
    match status {
        ReceiptStatus::Live => false,
        ReceiptStatus::Voided => true,
    }
}

/// Expected target of `void` for each status (exhaustive, see above).
fn expected_after_void(status: ReceiptStatus) -> ReceiptStatus {
    match status {
        ReceiptStatus::Live => ReceiptStatus::Voided,
        ReceiptStatus::Voided => ReceiptStatus::Voided,
    }
}

fn blank_receipt() -> Receipt {
    Receipt {
        payload_hash: [0u8; 32],
        issuer: Pubkey::default(),
        slot: 0,
        memo: String::new(),
        authority: Pubkey::default(),
        is_voided: false,
        bump: 0,
    }
}

fn issued(issuer: Pubkey) -> Receipt {
    let mut r = blank_receipt();
    r.issue(HASH, issuer, 42, "memo".to_string(), 254);
    r
}

fn receipt_in(status: ReceiptStatus) -> Receipt {
    let mut r = issued(Pubkey::new_unique());
    if status == ReceiptStatus::Voided {
        r.void();
    }
    assert_eq!(r.status(), status);
    r
}

#[test]
fn all_lists_every_variant_once() {
    // Each variant is listed, and each appears exactly once.
    for s in ReceiptStatus::ALL {
        assert_eq!(ReceiptStatus::ALL.iter().filter(|x| **x == s).count(), 1);
        // touch the exhaustive helpers so a new variant must be handled there
        let _ = (expected_flag(s), expected_after_void(s));
    }
    assert!(ReceiptStatus::ALL.contains(&ReceiptStatus::Live));
    assert!(ReceiptStatus::ALL.contains(&ReceiptStatus::Voided));
}

#[test]
fn persisted_flag_table_round_trips() {
    for s in ReceiptStatus::ALL {
        assert_eq!(s.is_voided(), expected_flag(s), "{s:?} flag");
        assert_eq!(ReceiptStatus::from_is_voided(s.is_voided()), s, "{s:?} round trip");
    }
    // Legacy values already on chain: the only bytes ever written are 0 and 1.
    assert_eq!(ReceiptStatus::from_is_voided(false), ReceiptStatus::Live);
    assert_eq!(ReceiptStatus::from_is_voided(true), ReceiptStatus::Voided);
}

#[test]
fn issue_starts_live_and_fills_every_field() {
    let issuer = Pubkey::new_unique();
    // Start from a dirty account to prove issue writes every field,
    // including the status flag.
    let mut r = blank_receipt();
    r.is_voided = true;
    r.issue(HASH, issuer, 42, "memo".to_string(), 254);
    assert_eq!(ReceiptStatus::INITIAL, ReceiptStatus::Live);
    assert_eq!(r.status(), ReceiptStatus::Live);
    assert!(!r.is_voided);
    assert_eq!(r.payload_hash, HASH);
    assert_eq!(r.issuer, issuer);
    assert_eq!(r.authority, issuer, "authority is the issuer at mint time");
    assert_eq!(r.slot, 42);
    assert_eq!(r.memo, "memo");
    assert_eq!(r.bump, 254);
}

#[test]
fn void_transition_table() {
    for s in ReceiptStatus::ALL {
        assert_eq!(s.void(), expected_after_void(s), "pure void from {s:?}");
        let mut r = receipt_in(s);
        let before = r.clone();
        r.void();
        assert_eq!(r.status(), expected_after_void(s), "Receipt::void from {s:?}");
        // void touches the status only
        assert_eq!(r.payload_hash, before.payload_hash);
        assert_eq!(r.issuer, before.issuer);
        assert_eq!(r.authority, before.authority);
        assert_eq!(r.slot, before.slot);
        assert_eq!(r.memo, before.memo);
        assert_eq!(r.bump, before.bump);
    }
}

#[test]
fn voided_is_terminal() {
    // No transition out of Voided leads anywhere but Voided.
    let mut r = receipt_in(ReceiptStatus::Voided);
    for _ in 0..3 {
        r.void();
        assert_eq!(r.status(), ReceiptStatus::Voided);
    }
}

#[test]
fn verdict_table() {
    for s in ReceiptStatus::ALL {
        for (queried, hash_equal) in [(HASH, true), (OTHER_HASH, false)] {
            let r = receipt_in(s);
            let key = Pubkey::new_unique();
            let v = VerificationResult::for_receipt(key, &r, &queried);
            let expected_matches = match s {
                ReceiptStatus::Live => hash_equal,
                ReceiptStatus::Voided => false,
            };
            assert_eq!(v.matches, expected_matches, "{s:?} hash_equal={hash_equal}");
            assert_eq!(v.voided, expected_flag(s), "{s:?}");
            assert!(!(v.matches && v.voided), "matches and voided together for {s:?}");
            assert_eq!(v.receipt, Some(key));
            assert_eq!(v.issuer, Some(r.issuer));
            assert_eq!(v.slot, Some(r.slot));
        }
    }
}

/// The account bytes are exactly what the pre-refactor program wrote:
/// discriminator, hash, issuer, slot, borsh memo, authority, is_voided, bump.
#[test]
fn account_layout_is_unchanged() {
    for s in ReceiptStatus::ALL {
        let r = receipt_in(s);
        let mut bytes = Vec::new();
        r.try_serialize(&mut bytes).unwrap();

        let mut expected = Vec::new();
        expected.extend_from_slice(<Receipt as anchor_lang::Discriminator>::DISCRIMINATOR);
        expected.extend_from_slice(&r.payload_hash);
        expected.extend_from_slice(r.issuer.as_ref());
        expected.extend_from_slice(&r.slot.to_le_bytes());
        expected.extend_from_slice(&(r.memo.len() as u32).to_le_bytes());
        expected.extend_from_slice(r.memo.as_bytes());
        expected.extend_from_slice(r.authority.as_ref());
        expected.push(expected_flag(s) as u8);
        expected.push(r.bump);
        assert_eq!(bytes, expected, "{s:?} account bytes");

        let back = Receipt::try_deserialize(&mut bytes.as_slice()).unwrap();
        assert_eq!(back.status(), s);
    }
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Void,
    Verify { same_hash: bool },
    Persist,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::Void),
        any::<bool>().prop_map(|same_hash| Op::Verify { same_hash }),
        Just(Op::Persist),
    ]
}

proptest! {
    /// Model test: drive random sequences of operations against a real
    /// `Receipt` and a two-line model, checking the invariants after every step.
    #[test]
    fn random_sequences_keep_invariants(ops in proptest::collection::vec(op(), 0..40)) {
        let issuer = Pubkey::new_unique();
        let mut r = issued(issuer);
        let mut model = ReceiptStatus::INITIAL;
        let mut ever_voided = false;

        for op in ops {
            match op {
                Op::Void => {
                    r.void();
                    model = ReceiptStatus::Voided;
                    ever_voided = true;
                }
                Op::Verify { same_hash } => {
                    let q = if same_hash { HASH } else { OTHER_HASH };
                    let v = VerificationResult::for_receipt(Pubkey::new_unique(), &r, &q);
                    prop_assert_eq!(v.matches, same_hash && model == ReceiptStatus::Live);
                    prop_assert_eq!(v.voided, model == ReceiptStatus::Voided);
                    prop_assert!(!(v.matches && v.voided));
                }
                Op::Persist => {
                    let mut bytes = Vec::new();
                    r.try_serialize(&mut bytes).unwrap();
                    r = Receipt::try_deserialize(&mut bytes.as_slice()).unwrap();
                }
            }
            // invariants after every step
            prop_assert_eq!(r.status(), model);
            prop_assert_eq!(r.is_voided, model.is_voided());
            prop_assert!(!(ever_voided && r.status() == ReceiptStatus::Live), "left Voided");
            prop_assert_eq!(r.issuer, issuer);
            prop_assert_eq!(r.authority, issuer);
            prop_assert_eq!(r.payload_hash, HASH);
        }
    }
}
