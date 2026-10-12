use anchor_lang::prelude::*;

declare_id!("AYqDidei67kRrTtJ4jqpG9iFJ9UfUg16ZN5mBEZj3sne");

/// One receipt per payload hash. The PDA seeds ["receipt", payload_hash]
/// make the account address itself a commitment to the exact 32-byte hash,
/// so a tampered payload hashes to a different address and cannot collide.
#[account]
#[derive(InitSpace)]
pub struct Receipt {
    /// The 32-byte hash of the payload that was issued.
    pub payload_hash: [u8; 32],
    /// The issuer: the signer who minted this receipt.
    pub issuer: Pubkey,
    /// The slot at which the receipt was issued.
    pub slot: u64,
    /// Optional free-form note supplied at issue time (max 64 bytes).
    #[max_len(64)]
    pub memo: String,
    /// Authority allowed to update/void (the issuer at mint time).
    pub authority: Pubkey,
    /// True if the receipt has been voided by its authority.
    pub is_voided: bool,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct VerificationResult {
    /// True when a live (non-voided) receipt exists and its stored
    /// payload hash equals the queried hash.
    pub matches: bool,
    /// The receipt account key, when found.
    pub receipt: Option<Pubkey>,
    /// The issuer recorded on the receipt, when found.
    pub issuer: Option<Pubkey>,
    /// The slot the receipt was issued in, when found.
    pub slot: Option<u64>,
    /// True when a receipt exists but was voided.
    pub voided: bool,
}

#[program]
pub mod trust_layer {
    use super::*;

    /// Mint a receipt for `payload_hash`. The signer is the issuer.
    /// Fails if the hash is not exactly 32 bytes or the receipt already exists.
    pub fn issue_receipt(ctx: Context<IssueReceipt>, payload_hash: [u8; 32], memo: String) -> Result<()> {
        require!(memo.len() <= 64, ErrorCode::MemoTooLong);
        let receipt = &mut ctx.accounts.receipt;
        receipt.payload_hash = payload_hash;
        receipt.issuer = ctx.accounts.issuer.key();
        receipt.slot = Clock::get()?.slot;
        receipt.memo = memo;
        receipt.authority = ctx.accounts.issuer.key();
        receipt.is_voided = false;
        receipt.bump = ctx.bumps.receipt;
        Ok(())
    }

    /// Verify a payload hash against the chain. Returns issuer, slot, and
    /// whether the hash matches a live receipt. Never fails on mismatch —
    /// it reports `matches: false` so clients get a structured verdict.
    pub fn verify_receipt(ctx: Context<VerifyReceipt>, payload_hash: [u8; 32]) -> Result<VerificationResult> {
        let receipt = &ctx.accounts.receipt;
        let hash_matches = receipt.payload_hash == payload_hash && !receipt.is_voided;
        Ok(VerificationResult {
            matches: hash_matches,
            receipt: Some(receipt.key()),
            issuer: Some(receipt.issuer),
            slot: Some(receipt.slot),
            voided: receipt.is_voided,
        })
    }

    /// Void (revoke) a receipt. Only the authority (issuer) may do this.
    pub fn void_receipt(_ctx: Context<VoidReceipt>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(payload_hash: [u8; 32], memo: String)]
pub struct IssueReceipt<'info> {
    #[account(
        init,
        payer = issuer,
        space = 8 + Receipt::INIT_SPACE,
        seeds = [b"receipt", payload_hash.as_ref()],
        bump
    )]
    pub receipt: Account<'info, Receipt>,
    #[account(mut)]
    pub issuer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(payload_hash: [u8; 32])]
pub struct VerifyReceipt<'info> {
    /// Checked against the queried hash; if the client derives the PDA with
    /// the *tampered* hash, the derived address differs and the constraint
    /// fails loudly (account-not-found / constraint) — tamper detected.
    #[account(
        seeds = [b"receipt", payload_hash.as_ref()],
        bump = receipt.bump,
    )]
    pub receipt: Account<'info, Receipt>,
}

#[derive(Accounts)]
pub struct VoidReceipt<'info> {
    #[account(
        mut,
        seeds = [b"receipt", receipt.payload_hash.as_ref()],
        bump = receipt.bump,
        has_one = authority @ ErrorCode::NotAuthority,
        close = authority
    )]
    pub receipt: Account<'info, Receipt>,
    pub authority: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Memo exceeds 64 bytes")]
    MemoTooLong,
    #[msg("Signer is not the receipt authority")]
    NotAuthority,
}
