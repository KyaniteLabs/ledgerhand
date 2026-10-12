# Devnet deploy path — CWF/ledgerhand (Platform infra)

Devnet = free (airdrop RPC + faucet), no spend, no keys held by Platform.

## One-time setup (done once solana CLI lands from the toolchain install)
1. `solana-keygen new --no-bip39-passphrase -o ~/.config/solana/id.json` — PM lane runs this; Platform never holds the keypair.
2. `solana config set --url devnet` (https://api.devnet.solana.com — free, rate-limited; a free airdrop RPC key can be added later if rate limits bite).
3. `solana airdrop 2` (faucet; repeatable within devnet limits).

## Deploy path (PM lane executes; this is the paved road)
```
anchor build && anchor test --skip-local-validator   # devnet
anchor deploy                                        # program id from Anchor.toml
```
`Anchor.toml` [provider.cluster] = devnet. CI never needs keys: the anchor
build gate compiles only; deploy is a local PM action by design.

## Boundaries
- Keys: PM/CEO held, never Platform, never in the repo, never in CI.
- CI: build/test only (no deploy secrets in the runner).
- Anything long-running on the mini gets deadman coverage (launchd label
  registered via deadman-collect automatically — it enumerates all
  LaunchAgents, so no per-service wiring needed).
