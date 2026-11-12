# Ledgerhand — Trust Layer

An Anchor/Solana receipt primitive for associating an issuer and slot with an exact 32-byte payload hash. The intended loop is issue a hash-bound receipt, look it up/verify it, and reject a changed payload. It proves a recorded commitment, not that the payload's claim or delivery is true.

## Existing parts

- Rust program with `issue_receipt`, `verify_receipt` and authority-only `void_receipt`; PDA seeds are `receipt` plus payload hash, not issuer.
- Rust SBF integration tests/compiled fixture and a minimal JavaScript/Anchor client with IDL. The demo generates a new wallet and defaults to devnet, so it is not an offline/read-only smoke command.
- Deploy/limitation notes and contest/pitch/disclosure docs. This is new receipt code, not recovered source of the earlier lost governed-spending prototype.

One account can exist per payload hash at a time, regardless of issuer. Void closes the account/refunds authority; it does not retain a persistent `is_voided=true` tombstone, and the live-account uniqueness rule does not establish permanent non-reissuance. Missing/tampered-hash accounts fail account/seed checks rather than always returning a structured `matches:false` result.

## Proof/status

Read the actual complete Rust program, client and deployment note; client syntax passed, IDL exposes the three instructions, compiled fixture exists. No validator, wallet funding/signing, receipt issue/void, chain query, destructive account test or deploy ran. Source/test-fixture presence is not on-chain acceptance.

README marks devnet deployment blocked/pending while the limitation note suggests the real cluster supports required features; current deployment is unverified. Preserve that uncertainty. No financial/escrow/compliance guarantee is claimed. See [ARCHITECTURE.md](ARCHITECTURE.md).
