# Trust Layer architecture

```text
caller hashes payload -> derives PDA [receipt, hash] -> signed issue instruction
 -> Receipt {hash, issuer, slot, memo, authority, is_voided, bump}
query hash + PDA -> Anchor account/seed validation -> verification result
issuer-signed void -> has_one authority / close -> account removed, lamports refunded
```

The full [program](programs/trust_layer/src/lib.rs) is small: issue initializes a PDA/account, bounds memo to 64 bytes and writes issuer/slot; verify reads a typed account and compares stored hash/nonvoid status; void has an empty handler because Anchor's close/account constraints implement the destructive operation. No handler changes `is_voided` to true before closure. Client [demo.mjs](ts-client/demo.mjs) builds an Anchor provider/new wallet and sends RPC instructions; IDL describes the same entrypoints.

Tests use the compiled SBF fixture, which is not by itself evidence its bytes match a newly rebuilt source tree. [DEPLOY-LIMITATION.md](DEPLOY-LIMITATION.md) records a validator/sBPF feature mismatch; historical test/deploy prose is not current cluster verification.

## Risk / failure boundaries

- Hash commitment does not establish claim truth, issuer trust, legal delivery, escrow correctness or privacy. Caller-side serialization/hash conventions and expected issuer checks are separate obligations.
- Hash-global PDA namespace means different issuers contend for the same hash; issuer is not part of identity. Closing permits absence rather than a durable revocation record, so consumers must not infer permanent uniqueness/history from live account state.
- Account absence/seeds errors can happen before structured verification. Do not treat an RPC exception as a `matches:false` response with complete issuer/slot fields, or assume all failures mean tampering rather than transport/deployment.
- Signing/funding/deployment/voiding has spend/public/irreversible effects and requires the appropriate approval. Demo/test names are not authority to execute them against a cluster.

## Docs verification

Complete Rust source, actual client/IDL and limitation read; JS syntax and fixture/IDL presence checked. No Rust/validator tests, account deletion, credentials, chain network, wallet or deployment exercised. No current on-chain/security acceptance; runtime unchanged, revert docs for rollback.
