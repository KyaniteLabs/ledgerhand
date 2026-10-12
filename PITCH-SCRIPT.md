# PITCH SCRIPT — "Ledgerhand — The Trust Layer"
**Length:** 2-3 min | **Record:** 15:00-16:00 PT Oct 12 | **Read from:** CWF-PM-DESK, README Disclosure, approved on-camera sentence

Every claim below is checkable against an artifact. Do not ad-lib beyond the script without verifying the artifact exists.

---

## 0:00–0:15 — Hook

"Anyone can claim they delivered something. On Solana, anyone can **prove** it.

This is Ledgerhand — The Trust Layer: a program that turns any claim, delivery, or promise into a verifiable on-chain receipt."

---

## 0:15–0:40 — The problem

"As AI agents increasingly deliver code, content, and decisions, there's no trustless way to ask the simplest question: did this exact thing actually happen, from this party, without being tampered with?

You can verify a payment happened on-chain. But you can't verify a *delivery* happened — not a file, not a contract, not a promise — without trusting someone's word."

---

## 0:40–1:10 — How it works (keep it concrete)

"We built a Solana Anchor program — three instructions.

**Issue.** Anyone mints a receipt for a 32-byte hash of their payload. The signer becomes the issuer. One receipt per payload — no duplication.

**Verify.** Anyone checks: does this receipt exist, does the hash match, who issued it? One command. Trustless.

**Tamper-detect.** Change one byte of the payload — the hash changes — the receipt doesn't match — verification fails loudly. That's the moment blockchain earns its place."

[SCREEN: show the test suite — 5/5 passing, real compute-unit consumption, real Anchor error on tamper]

---

## 1:10–1:35 — Prior work and what this adds

"Three days ago, we published a working demo of Ledgerhand — a governed-spending quorum where multiple AI agents hold one spending key. That prototype ran off-chain. Its own description said the on-chain verification leg was 'one funded testnet key away.'

This submission is that leg. The repository behind the prototype was local-only — never pushed to an org remote — and was lost from the working machine before this repository existed. That loss is documented in our repo-sync receipt dated September 20.

This repo — trust_layer — is new code, built entirely October 11th through 12th. The prior demo's receipt book and the on-chain program are designed to compose: the video's governed-spending receipts are exactly what this program verifies on-chain."

---

## 1:35–2:00 — Why it matters / composability

"A receipt is a primitive, not a platform. Any Solana program can issue or consume receipts without permission.

Marketplace delivery proofs — a seller mints a receipt, the buyer verifies.
Escrow release conditions — pay out only when a verified receipt exists.
Oracle attestations — any off-chain event gets a tamper-evident on-chain record.

Every AI-agent economy needs this layer. We're building it as Apache-2.0 — the standard itself is the prize."

---

## 2:00–2:20 — Honest scope

"Today: a local validator run — 5/5 tests green against the compiled binary, with real instruction execution, real compute-unit consumption, and real Anchor errors. A real transaction signature from the same chain is in our receipt as chain-liveness proof.

The deploy to devnet is blocked by an SBPF version mismatch on this validator — that limitation is documented honestly in the repo.

We are not claiming what we can't show."

---

## 2:20–2:40 — Close

"Repo link, three-command quickstart, full disclosure, and the approved on-camera sentence — all in the README.

Verify it yourself: `cargo test` against the compiled binary. Five tests. Zero dependencies beyond Rust.

Ledgerhand — The Trust Layer."

---

## ARTIFACT REFERENCES (do not claim without these)

| Claim | Artifact |
|---|---|
| 5/5 tests green | `G2-RECEIPT-local-validator.txt` (lines 22-28) |
| Real TX signature | `G2-RECEIPT-local-validator.txt` (lines 13-16, signature `4eu4sbHx...`) |
| Real compute-unit consumption | `G2-RECEIPT-local-validator.txt` (lines 30-55) |
| SBPF deploy limitation | `DEPLOY-LIMITATION.md` in repo |
| Prior demo video (Oct 9) | https://www.youtube.com/watch?v=TdNA-H70FD8 (2:02) |
| Repo-sync loss receipt | Referenced in README Disclosure section |
| Program ID | `AYqDidei67kRrTtJ4jqpG9iFJ9UfUg16ZN5mBEZj3sne` |
| Approved on-camera sentence | LIAM, 2026-10-12 (CWF-PM-DESK.md §APPROVED ON-CAMERA SENTENCE) |