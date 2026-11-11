# ledgerhand — The Trust Layer

CWF contest entry (Colosseum v4) — ARENA PM lane owns the product build.
Infra lane: Platform (repo, CI, toolchain, devnet, deploy path, deadman coverage).

## What this is

A Solana (Anchor) program that turns any claim, delivery, or promise into a
**verifiable receipt**:

1. **issue_receipt(payload_hash, memo)** — anyone mints a receipt PDA for a
   32-byte payload hash. The signer is the issuer. One receipt per payload
   hash (re-issue is rejected).
2. **verify_receipt(payload_hash)** — returns issuer + slot + match boolean.
   Never silently fails on mismatch: it reports a structured verdict, and a
   tampered payload hash derives a *different* PDA, so verification fails
   loudly (no account / seeds constraint).
3. **void_receipt()** — the issuer (authority) can revoke a receipt. The
   account is **marked** (`is_voided = true`), never closed, so revocation
   is durable: the PDA stays initialized and the same payload hash can never
   be re-issued by anyone. `verify_receipt` reports `voided: true` for it.
   The voided account keeps its rent-exempt lamports — that rent is the
   price of a durable revocation record and is not refunded.

Program ID: `AYqDidei67kRrTtJ4jqpG9iFJ9UfUg16ZN5mBEZj3sne` (devnet, pending deploy).

PDA scheme: `["receipt", payload_hash]` — the account address is itself a
commitment to the exact 32-byte hash, so a one-byte payload change cannot
collide with an existing receipt.

## Repo layout

- `programs/trust_layer/` — the Anchor program (anchor-lang 0.31).
- `tests/` — Rust integration tests (solana-program-test, SBF mode: they run
  the *real compiled `trust_layer.so`*, the same artifact that deploys).
- `ts-client/` — minimal TypeScript client (web3.js + IDL).
- `tests/fixtures/trust_layer.so` — the SBF build artifact used by tests.

## Quickstart (no Solana CLI needed)

```sh
./run_tests.sh
cargo test --manifest-path tests/Cargo.toml    # issue -> verify OK -> tamper -> FAIL
```

`.cargo/config.toml` sets `SBF_OUT_DIR` to `tests/fixtures` for every cargo
command in this repo, so the suite finds the committed `trust_layer.so` and the
bare command above works from any subdirectory. `./run_tests.sh` sets the same
variable explicitly and forwards extra arguments to cargo.

The six tests:
1. `core_loop_issue_verify_tamper` — THE demo: issue → verify(matches=true,
   issuer, slot) → flip one payload byte → verify FAILS on-chain.
2. `unknown_hash_has_no_receipt` — random hash: no account, verify fails.
3. `double_issue_is_rejected` — same hash twice → rejected.
4. `non_authority_cannot_void` — attacker can't revoke someone's receipt.
5. `authority_can_void` — issuer voids: the receipt is marked
   (`is_voided = true`), the account survives, and verify reports
   `voided: true` with `matches: false`.
6. `voided_receipt_cannot_be_reissued` — after a void, the same payload hash
   cannot be re-issued by the original issuer *or* by a different signer.

Host-side tests in `tests/src/receipt_status.rs` pin the receipt lifecycle
(`ReceiptStatus`: `Live -> Voided`, `Voided` terminal) against the program's
Rust source: the status/`is_voided` table, every transition, the verify
verdict table, the account byte layout, and a proptest model test over random
operation sequences. They run in the same `cargo test` command.

Regenerate the committed IDL from the program after changing accounts,
instructions or their docs (never hand-edit it):

```sh
./scripts/gen-idl.sh
```

The script sets `ANCHOR_IDL_BUILD_RESOLUTION=TRUE`, which anchor-syn requires
at proc-macro time to emit `pda` seeds, account `address` entries and
`relations` in the IDL (without it the IDL is silently degraded). It writes
to a temp file and only replaces the committed IDL on success, so a failed
run cannot leave a truncated file.

## TS client

```sh
cd ts-client && pnpm install && pnpm demo   # devnet demo (needs funded wallet)
```

## Composability

The Trust Layer is a **primitive, not a platform.** Receipts compose upward:

- **Marketplace delivery proof** — a seller issues a receipt for a delivered artifact; the buyer verifies with one command. No platform trust required.
- **Conditional escrow** — an escrow program consumes a verified receipt as the release condition. The receipt is the proof; escrow is the consumer.
- **Oracle attestation** — an oracle issues receipts for observed events (price, vote, milestone). Downstream programs verify without trusting the oracle's interface.
- **Compliance trail** — every receipt is a tamper-evident audit entry. Verify any historical claim against its exact payload.

Any Solana program can issue or consume receipts without permission — PDAs keyed by payload hash mean the account address is itself a commitment to the exact content.

## Disclosure

Prior in-window work exists and is disclosed here in full:

- **Prior public demo, 2026-10-09:** ["Ledgerhand — governed spending for agent fleets (demo)"](https://www.youtube.com/watch?v=TdNA-H70FD8) — a working prototype of a quorum ledger: a checkable receipts book, chain-verified facts, quorum voting with signed ballots, and an honest refusal to anchor on-chain without the executive key. It was recorded three days before this repository existed.
- **The prototype's repository was local-only and never pushed to an org remote** — the repo-sync receipt of 2026-09-20 records `ledgerhand [main]: NOT-BACKED-UP: no org remote configured — canonical missing`. It was lost from the working machine before 2026-10-11; searches of this machine, the org Forgejo, and the public GitHub org found no surviving copy.
- **This repository is new code, built in-window** (2026-10-11–12; first commit `015130e`, 22:11 PT). It is the on-chain receipt leg the prior demo described as "one funded testnet key away." No source files from the prototype were carried into it — every commit in this history is in-window and verifiable above.

## Status

- [x] P1 core program + green cargo test (2026-10-12, gate G1)
- [x] P2 local validator run + real TX signature + 5/5 tests green (gate G2)
- [ ] P2b devnet deploy (blocked: `solana program deploy` — SBPF feature-set mismatch on this validator; disclosed in DEPLOY-LIMITATION.md)
- [ ] P3 docs complete (this file + business plan)
- Licence: Apache-2.0 (see LICENSE; Security ruling on file)
