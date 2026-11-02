use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    ed25519_program,
    instruction::Instruction,
    sysvar::instructions as ix_sysvar,
};

declare_id!("Encu26Madrid111111111111111111111111111111111");

pub const MAX_URI_LEN: usize = 128;
pub const DISCRIMINATOR_LENGTH: usize = 8;

#[program]
pub mod mad_encuentro {
    use super::*;

    /// Initializes a new promotional campaign for El Encuentro 2026 Madrid.
    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        campaign_id: u64,
        oracle_authority: Pubkey,
        reward_per_claim_lamports: u64,
        max_claims: u32,
        end_timestamp: i64,
    ) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(
            end_timestamp > clock.unix_timestamp,
            MadEncuentroError::InvalidTimestamp
        );
        require!(
            reward_per_claim_lamports > 0,
            MadEncuentroError::InvalidRewardAmount
        );
        require!(max_claims > 0, MadEncuentroError::InvalidMaxClaims);

        campaign.organizer = ctx.accounts.organizer.key();
        campaign.oracle_authority = oracle_authority;
        campaign.campaign_id = campaign_id;
        campaign.reward_per_claim = reward_per_claim_lamports;
        campaign.max_claims = max_claims;
        campaign.claims_count = 0;
        campaign.end_timestamp = end_timestamp;
        campaign.is_active = true;
        campaign.bump = ctx.bumps.campaign;
        campaign.vault_bump = ctx.bumps.campaign_vault;

        emit!(CampaignCreatedEvent {
            campaign_id,
            organizer: campaign.organizer,
            reward_per_claim: reward_per_claim_lamports,
            max_claims,
            end_timestamp,
        });

        Ok(())
    }

    /// Deposits funds into the campaign reward vault.
    pub fn fund_campaign(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
        require!(amount > 0, MadEncuentroError::InvalidDepositAmount);

        let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.funder.key(),
            &ctx.accounts.campaign_vault.key(),
            amount,
        );
        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.funder.to_account_info(),
                ctx.accounts.campaign_vault.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        emit!(CampaignFundedEvent {
            campaign_id: ctx.accounts.campaign.campaign_id,
            funder: ctx.accounts.funder.key(),
            amount,
        });

        Ok(())
    }

    /// Submits verified content and claims a promotional bounty and ticket tier allocation.
    pub fn claim_reward(
        ctx: Context<ClaimReward>,
        content_hash: [u8; 32],
        content_uri: String,
        oracle_signature: [u8; 64],
    ) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(campaign.is_active, MadEncuentroError::CampaignInactive);
        require!(
            clock.unix_timestamp <= campaign.end_timestamp,
            MadEncuentroError::CampaignExpired
        );
        require!(
            campaign.claims_count < campaign.max_claims,
            MadEncuentroError::MaxClaimsExceeded
        );
        require!(
            content_uri.as_bytes().len() <= MAX_URI_LEN,
            MadEncuentroError::UriTooLong
        );

        // Construct expected attestation payload:
        // SHA256(creator_pubkey || campaign_id || content_hash)
        let mut payload: Vec<u8> = Vec::with_capacity(32 + 8 + 32);
        payload.extend_from_slice(&ctx.accounts.creator.key().to_bytes());
        payload.extend_from_slice(&campaign.campaign_id.to_le_bytes());
        payload.extend_from_slice(&content_hash);

        // Verify that the Oracle validated this claim via Secp256k1/Ed25519 pre-instruction
        // or through in-instruction programmatic Ed25519 signature checks.
        verify_oracle_signature(
            &ctx.accounts.ix_sysvar.to_account_info(),
            &campaign.oracle_authority,
            &payload,
            &oracle_signature,
        )?;

        // Update campaign state
        campaign.claims_count = campaign
            .claims_count
            .checked_add(1)
            .ok_or(MadEncuentroError::ArithmeticOverflow)?;

        // Update creator claim record
        let claim_record = &mut ctx.accounts.claim_record;
        claim_record.creator = ctx.accounts.creator.key();
        claim_record.campaign_id = campaign.campaign_id;
        claim_record.content_hash = content_hash;
        claim_record.claimed_at = clock.unix_timestamp;
        claim_record.reward_amount = campaign.reward_per_claim;
        claim_record.bump = ctx.bumps.claim_record;

        // Disburse lamports from vault PDA to creator
        let campaign_id_bytes = campaign.campaign_id.to_le_bytes();
        let seeds = &[
            b"campaign_vault",
            campaign_id_bytes.as_ref(),
            &[campaign.vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        **ctx
            .accounts
            .campaign_vault
            .to_account_info()
            .try_borrow_mut_lamports()? -= campaign.reward_per_claim;
        **ctx
            .accounts
            .creator
            .to_account_info()
            .try_borrow_mut_lamports()? += campaign.reward_per_claim;

        emit!(ContentClaimedEvent {
            campaign_id: campaign.campaign_id,
            creator: ctx.accounts.creator.key(),
            content_hash,
            content_uri,
            reward_amount: campaign.reward_per_claim,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Emergency pause/stop by the organizer.
    pub fn toggle_campaign_status(ctx: Context<ToggleStatus>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        campaign.is_active = !campaign.is_active;
        emit!(CampaignStatusChangedEvent {
            campaign_id: campaign.campaign_id,
            is_active: campaign.is_active,
        });
        Ok(())
    }

    /// Recovers remaining unspent treasury after campaign termination.
    pub fn close_and_refund(ctx: Context<CloseAndRefund>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let clock = Clock::get()?;

        require!(
            clock.unix_timestamp > campaign.end_timestamp || !campaign.is_active,
            MadEncuentroError::CampaignStillActive
        );

        let vault_lamports = ctx.accounts.campaign_vault.lamports();
        require!(vault_lamports > 0, MadEncuentroError::NoRemainingFunds);

        **ctx
            .accounts
            .campaign_vault
            .to_account_info()
            .try_borrow_mut_lamports()? = 0;
        **ctx
            .accounts
            .organizer
            .to_account_info()
            .try_borrow_mut_lamports()? += vault_lamports;

        emit!(CampaignRefundedEvent {
            campaign_id: campaign.campaign_id,
            organizer: ctx.accounts.organizer.key(),
            refunded_amount: vault_lamports,
        });

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Validation Helper
// -----------------------------------------------------------------------------
fn verify_oracle_signature(
    ix_sysvar: &AccountInfo,
    expected_oracle: &Pubkey,
    message: &[u8],
    oracle_signature: &[u8; 64],
) -> Result<()> {
    // If the Ed25519 instruction was passed in the transaction before this ix:
    let current_index = ix_sysvar::load_current_index_checked(ix_sysvar)?;
    if current_index == 0 {
        return Err(MadEncuentroError::MissingSignatureVerificationInstruction.into());
    }

    let ed25519_ix = ix_sysvar::load_instruction_at_checked(
        (current_index - 1) as usize,
        ix_sysvar,
    )?;

    // Verify program ID
    require_keys_eq!(
        ed25519_ix.program_id,
        ed25519_program::ID,
        MadEncuentroError::InvalidVerificationInstruction
    );

    // Verify that the signer public key and data in the pre-instruction match
    validate_ed25519_instruction_data(&ed25519_ix, expected_oracle, message, oracle_signature)?;

    Ok(())
}

fn validate_ed25519_instruction_data(
    instruction: &Instruction,
    expected_pubkey: &Pubkey,
    expected_msg: &[u8],
    expected_sig: &[u8; 64],
) -> Result<()> {
    let data = &instruction.data;
    // Ed25519 instruction format header layout
    require!(data.len() >= 16 + 32 + 64, MadEncuentroError::MalformedInstruction);

    // Offset 16..48 contains public key
    let pubkey_slice = &data[16..48];
    require!(
        pubkey_slice == expected_pubkey.as_ref(),
        MadEncuentroError::OraclePubkeyMismatch
    );

    // Offset 48..112 contains the 64-byte signature
    let sig_slice = &data[48..112];
    require!(
        sig_slice == expected_sig.as_ref(),
        MadEncuentroError::SignatureMismatch
    );

    // Offset 112.. is the signed payload
    let msg_slice = &data[112..];
    require!(
        msg_slice == expected_msg,
        MadEncuentroError::SignedPayloadMismatch
    );

    Ok(())
}

// -----------------------------------------------------------------------------
// Account Contexts
// -----------------------------------------------------------------------------
#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct InitializeCampaign<'info> {
    #[account(
        init,
        payer = organizer,
        space = DISCRIMINATOR_LENGTH + CampaignAccount::LEN,
        seeds = [b"campaign", campaign_id.to_le_bytes().as_ref()],
        bump
    )]
    pub campaign: Account<'info, CampaignAccount>,

    /// CHECK: PDA used strictly as SOL vault for rewards
    #[account(
        init,
        payer = organizer,
        space = 0,
        seeds = [b"campaign_vault", campaign_id.to_le_bytes().as_ref()],
        bump
    )]
    pub campaign_vault: AccountInfo<'info>,

    #[account(mut)]
    pub organizer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FundCampaign<'info> {
    #[account(
        mut,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignAccount>,

    /// CHECK: Reward vault PDA
    #[account(
        mut,
        seeds = [b"campaign_vault", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.vault_bump
    )]
    pub campaign_vault: AccountInfo<'info>,

    #[account(mut)]
    pub funder: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(content_hash: [u8; 32])]
pub struct ClaimReward<'info> {
    #[account(
        mut,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignAccount>,

    /// CHECK: Campaign Vault PDA
    #[account(
        mut,
        seeds = [b"campaign_vault", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.vault_bump
    )]
    pub campaign_vault: AccountInfo<'info>,

    #[account(
        init,
        payer = creator,
        space = DISCRIMINATOR_LENGTH + ClaimRecord::LEN,
        seeds = [
            b"claim",
            campaign.campaign_id.to_le_bytes().as_ref(),
            content_hash.as_ref()
        ],
        bump
    )]
    pub claim_record: Account<'info, ClaimRecord>,

    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: Instructions Sysvar Account
    #[account(address = ix_sysvar::ID)]
    pub ix_sysvar: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ToggleStatus<'info> {
    #[account(
        mut,
        has_one = organizer @ MadEncuentroError::UnauthorizedOrganizer,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignAccount>,

    pub organizer: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseAndRefund<'info> {
    #[account(
        mut,
        has_one = organizer @ MadEncuentroError::UnauthorizedOrganizer,
        seeds = [b"campaign", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignAccount>,

    /// CHECK: Vault to refund from
    #[account(
        mut,
        seeds = [b"campaign_vault", campaign.campaign_id.to_le_bytes().as_ref()],
        bump = campaign.vault_bump
    )]
    pub campaign_vault: AccountInfo<'info>,

    #[account(mut)]
    pub organizer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// -----------------------------------------------------------------------------
// State Structures
// -----------------------------------------------------------------------------
#[account]
pub struct CampaignAccount {
    pub organizer: Pubkey,         // 32
    pub oracle_authority: Pubkey,  // 32
    pub campaign_id: u64,          // 8
    pub reward_per_claim: u64,      // 8
    pub max_claims: u32,           // 4
    pub claims_count: u32,         // 4
    pub end_timestamp: i64,        // 8
    pub is_active: bool,           // 1
    pub bump: u8,                  // 1
    pub vault_bump: u8,            // 1
}

impl CampaignAccount {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 4 + 4 + 8 + 1 + 1 + 1;
}

#[account]
pub struct ClaimRecord {
    pub creator: Pubkey,           // 32
    pub campaign_id: u64,          // 8
    pub content_hash: [u8; 32],    // 32
    pub claimed_at: i64,           // 8
    pub reward_amount: u64,        // 8
    pub bump: u8,                  // 1
}

impl ClaimRecord {
    pub const LEN: usize = 32 + 8 + 32 + 8 + 8 + 1;
}

// -----------------------------------------------------------------------------
// Events
// -----------------------------------------------------------------------------
#[event]
pub struct CampaignCreatedEvent {
    pub campaign_id: u64,
    pub organizer: Pubkey,
    pub reward_per_claim: u64,
    pub max_claims: u32,
    pub end_timestamp: i64,
}

#[event]
pub struct CampaignFundedEvent {
    pub campaign_id: u64,
    pub funder: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ContentClaimedEvent {
    pub campaign_id: u64,
    pub creator: Pubkey,
    pub content_hash: [u8; 32],
    pub content_uri: String,
    pub reward_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct CampaignStatusChangedEvent {
    pub campaign_id: u64,
    pub is_active: bool,
}

#[event]
pub struct CampaignRefundedEvent {
    pub campaign_id: u64,
    pub organizer: Pubkey,
    pub refunded_amount: u64,
}

// -----------------------------------------------------------------------------
// Error Codes
// -----------------------------------------------------------------------------
#[error_code]
pub enum MadEncuentroError {
    #[msg("Campaign timestamp must be set in the future.")]
    InvalidTimestamp,
    #[msg("Reward per claim must be greater than zero.")]
    InvalidRewardAmount,
    #[msg("Max claims must be greater than zero.")]
    InvalidMaxClaims,
    #[msg("Deposit amount must be greater than zero.")]
    InvalidDepositAmount,
    #[msg("The promotional campaign is currently inactive.")]
    CampaignInactive,
    #[msg("The promotional campaign has expired.")]
    CampaignExpired,
    #[msg("The promotional campaign has reached maximum claim capacity.")]
    MaxClaimsExceeded,
    #[msg("Content URI exceeds maximum allowed length of 128 bytes.")]
    UriTooLong,
    #[msg("Arithmetic overflow occurred.")]
    ArithmeticOverflow,
    #[msg("Missing required Ed25519 verification pre-instruction.")]
    MissingSignatureVerificationInstruction,
    #[msg("Previous instruction is not a valid Ed25519 program call.")]
    InvalidVerificationInstruction,
    #[msg("Ed25519 instruction data is malformed.")]
    MalformedInstruction,
    #[msg("Signer does not match the registered campaign oracle authority.")]
    OraclePubkeyMismatch,
    #[msg("Oracle signature does not match.")]
    SignatureMismatch,
    #[msg("Signed payload does not match expected creator and content parameters.")]
    SignedPayloadMismatch,
    #[msg("Caller is not authorized to manage this campaign.")]
    UnauthorizedOrganizer,
    #[msg("Campaign is still active and has not expired yet.")]
    CampaignStillActive,
    #[msg("No remaining funds to refund in the campaign vault.")]
    NoRemainingFunds,
}