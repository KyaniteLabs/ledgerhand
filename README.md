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
3. **void_receipt()** — the issuer (authority) can revoke + close a receipt.

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

The five tests:
1. `core_loop_issue_verify_tamper` — THE demo: issue → verify(matches=true,
   issuer, slot) → flip one payload byte → verify FAILS on-chain.
2. `unknown_hash_has_no_receipt` — random hash: no account, verify fails.
3. `double_issue_is_rejected` — same hash twice → rejected.
4. `non_authority_cannot_void` — attacker can't revoke someone's receipt.
5. `authority_can_void` — issuer voids + account closes.

## TS client

```sh
cd ts-client && pnpm install && pnpm demo   # devnet demo (needs funded wallet)
```

## Status

- [x] P1 core program + green cargo test (2026-10-12, gate G1)
- [ ] P2 devnet deploy + live receipt TX
- Licence: MIT.

## Composability

The Trust Layer is a **primitive, not a platform.** Receipts compose upward:

- **Marketplace delivery proof** — a seller issues a receipt for a delivered artifact; the buyer verifies with one command. No platform trust required.
- **Conditional escrow** — an escrow program consumes a verified receipt as the release condition. The receipt is the proof; escrow is the consumer.
- **Oracle attestation** — an oracle issues receipts for observed events (price, vote, milestone). Downstream programs verify without trusting the oracle's interface.
- **Compliance trail** — every receipt is a tamper-evident audit entry. Verify any historical claim against its exact payload.

Any Solana program can issue or consume receipts without permission — PDAs keyed by payload hash mean the account address is itself a commitment to the exact content.

## Disclosure

**No pre-existing code used.** This project was built entirely in-window (October 11–12, 2026) for the Colosseum CWF contest. The repository was initialized from zero on October 11, 2026 at 22:11 PT (first commit `015130e`). All program code, tests, and documentation were authored in-window.

## Status

- [x] P1 core program + green cargo test (2026-10-12, gate G1)
- [x] P2 local validator run + real TX signature + 5/5 tests green (gate G2)
- [ ] P2b devnet deploy (blocked: `solana program deploy` — SBPF feature-set mismatch on this validator; disclosed in DEPLOY-LIMITATION.md)
- [ ] P3 docs complete (this file + business plan)
- Licence: MIT
