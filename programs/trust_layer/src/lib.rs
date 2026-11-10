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

/// The lifecycle of a receipt, named in one place.
///
/// ```text
///   (no account) --issue_receipt--> Live --void_receipt--> Voided --+
///                                                             ^      | void_receipt
///                                                             +------+ (idempotent no-op)
/// ```
///
/// "Not issued" is the absence of the `["receipt", payload_hash]` PDA, not a
/// variant: `issue_receipt` creates the account with Anchor `init`, which
/// refuses an address that already exists, so neither a `Live` nor a `Voided`
/// receipt can be issued again. `Voided` is terminal: there is no move back to
/// `Live`, and this type offers no way to express one.
///
/// On chain the status is stored as the `Receipt::is_voided` byte
/// (`0` = `Live`, `1` = `Voided`), so the account layout is unchanged.
/// All writes to that byte go through [`Receipt::issue`] and [`Receipt::void`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReceiptStatus {
    /// Issued and not revoked: verification can match.
    Live,
    /// Revoked by its authority. Terminal.
    Voided,
}

impl ReceiptStatus {
    /// Every status, for exhaustive checks.
    pub const ALL: [ReceiptStatus; 2] = [ReceiptStatus::Live, ReceiptStatus::Voided];

    /// The status a freshly issued receipt starts in.
    pub const INITIAL: ReceiptStatus = ReceiptStatus::Live;

    /// Decode the persisted `is_voided` flag.
    pub const fn from_is_voided(is_voided: bool) -> Self {
        if is_voided {
            ReceiptStatus::Voided
        } else {
            ReceiptStatus::Live
        }
    }

    /// Encode as the persisted `is_voided` flag.
    pub const fn is_voided(self) -> bool {
        match self {
            ReceiptStatus::Live => false,
            ReceiptStatus::Voided => true,
        }
    }

    /// The `void_receipt` transition: `Live -> Voided`, and `Voided -> Voided`
    /// as an idempotent no-op so a retried revocation still succeeds.
    pub const fn void(self) -> Self {
        match self {
            ReceiptStatus::Live | ReceiptStatus::Voided => ReceiptStatus::Voided,
        }
    }

    /// Whether a receipt in this status can verify as a match.
    pub const fn can_match(self) -> bool {
        match self {
            ReceiptStatus::Live => true,
            ReceiptStatus::Voided => false,
        }
    }
}

impl Receipt {
    /// Current lifecycle status, decoded from the stored flag.
    pub fn status(&self) -> ReceiptStatus {
        ReceiptStatus::from_is_voided(self.is_voided)
    }

    /// The `issue_receipt` transition: fill a freshly created account.
    pub fn issue(&mut self, payload_hash: [u8; 32], issuer: Pubkey, slot: u64, memo: String, bump: u8) {
        self.payload_hash = payload_hash;
        self.issuer = issuer;
        self.slot = slot;
        self.memo = memo;
        self.authority = issuer;
        self.is_voided = ReceiptStatus::INITIAL.is_voided();
        self.bump = bump;
    }

    /// The `void_receipt` transition. Only the status changes.
    pub fn void(&mut self) {
        self.is_voided = self.status().void().is_voided();
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
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

impl VerificationResult {
    /// The verdict for `receipt` (stored at `receipt_key`) against a queried
    /// hash. `matches` and `voided` are both derived from the receipt's
    /// status here, so `matches && voided` cannot be produced.
    pub fn for_receipt(receipt_key: Pubkey, receipt: &Receipt, payload_hash: &[u8; 32]) -> Self {
        let status = receipt.status();
        VerificationResult {
            matches: status.can_match() && receipt.payload_hash == *payload_hash,
            receipt: Some(receipt_key),
            issuer: Some(receipt.issuer),
            slot: Some(receipt.slot),
            voided: status.is_voided(),
        }
    }
}

#[program]
pub mod trust_layer {
    use super::*;

    /// Mint a receipt for `payload_hash`. The signer is the issuer.
    /// Fails if the hash is not exactly 32 bytes or the receipt already exists.
    pub fn issue_receipt(ctx: Context<IssueReceipt>, payload_hash: [u8; 32], memo: String) -> Result<()> {
        require!(memo.len() <= 64, ErrorCode::MemoTooLong);
        let slot = Clock::get()?.slot;
        let issuer = ctx.accounts.issuer.key();
        let bump = ctx.bumps.receipt;
        ctx.accounts.receipt.issue(payload_hash, issuer, slot, memo, bump);
        Ok(())
    }

    /// Verify a payload hash against the chain. Returns issuer, slot, and
    /// whether the hash matches a live receipt. Never fails on mismatch —
    /// it reports `matches: false` so clients get a structured verdict.
    pub fn verify_receipt(ctx: Context<VerifyReceipt>, payload_hash: [u8; 32]) -> Result<VerificationResult> {
        let receipt = &ctx.accounts.receipt;
        Ok(VerificationResult::for_receipt(receipt.key(), receipt, &payload_hash))
    }

    /// Void (revoke) a receipt. Only the authority (issuer) may do this.
    /// The receipt account is kept and marked, never closed: revocation has
    /// to stay durable. A closed PDA would be re-initialisable, so any signer
    /// could re-issue the same payload hash right after a revoke.
    pub fn void_receipt(ctx: Context<VoidReceipt>) -> Result<()> {
        ctx.accounts.receipt.void();
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
