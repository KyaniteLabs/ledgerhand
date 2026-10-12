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
export SBF_OUT_DIR=$PWD/tests/fixtures
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
