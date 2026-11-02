use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("Encu26MadR1dPromoXxxx11111111111111111111111");

pub const CAMPAIGN_SEED: &[u8] = b"encuentro_2026_campaign";
pub const AMBASSADOR_SEED: &[u8] = b"ambassador_account";
pub const PROOF_SEED: &[u8] = b"content_proof";
pub const VAULT_SEED: &[u8] = b"campaign_vault";

#[program]
pub mod el_encuentro_promo {
    use super::*;

    /// Initializes the El Encuentro 2026 Master Promotional Campaign.
    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        reward_per_verified_submission: u64,
        max_submissions_per_creator: u32,
    ) -> Result<()> {
        require!(reward_per_verified_submission > 0, PromoError::InvalidRewardAmount);
        require!(max_submissions_per_creator > 0, PromoError::InvalidMaxSubmissions);

        let campaign = &mut ctx.accounts.campaign;
        campaign.admin = ctx.accounts.admin.key();
        campaign.verifier = ctx.accounts.verifier.key();
        campaign.reward_mint = ctx.accounts.reward_mint.key();
        campaign.vault = ctx.accounts.vault.key();
        campaign.reward_per_submission = reward_per_verified_submission;
        campaign.max_submissions_per_creator = max_submissions_per_creator;
        campaign.total_distributed = 0;
        campaign.total_verified_creators = 0;
        campaign.bump = ctx.bumps.campaign;
        campaign.vault_bump = ctx.bumps.vault;

        emit!(CampaignInitialized {
            admin: campaign.admin,
            verifier: campaign.verifier,
            reward_per_submission: reward_per_verified_submission,
        });

        Ok(())
    }

    /// Registers a new Madrid 2026 Ambassador / Creator with custom attribution handle.
    pub fn register_ambassador(
        ctx: Context<RegisterAmbassador>,
        handle: String,
    ) -> Result<()> {
        require!(handle.len() >= 3 && handle.len() <= 32, PromoError::InvalidHandleLength);

        let ambassador = &mut ctx.accounts.ambassador;
        ambassador.creator = ctx.accounts.creator.key();
        ambassador.handle = handle.clone();
        ambassador.total_submissions = 0;
        ambassador.approved_submissions = 0;
        ambassador.total_rewards_earned = 0;
        ambassador.vip_pass_awarded = false;
        ambassador.bump = ctx.bumps.ambassador;

        emit!(AmbassadorRegistered {
            creator: ctx.accounts.creator.key(),
            handle,
        });

        Ok(())
    }

    /// Submits a proof of content creation (e.g., tweet hash, video identifier hash).
    pub fn submit_content_proof(
        ctx: Context<SubmitContentProof>,
        content_hash: [u8; 32],
        platform_id: u8, // 0 = X/Twitter, 1 = YouTube, 2 = Medium/Mirror, 3 = TikTok
    ) -> Result<()> {
        let ambassador = &mut ctx.accounts.ambassador;
        let campaign = &ctx.accounts.campaign;

        require!(
            ambassador.total_submissions < campaign.max_submissions_per_creator,
            PromoError::MaxSubmissionsExceeded
        );

        let proof = &mut ctx.accounts.proof;
        proof.creator = ctx.accounts.creator.key();
        proof.content_hash = content_hash;
        proof.platform_id = platform_id;
        proof.verified = false;
        proof.submission_timestamp = Clock::get()?.unix_timestamp;
        proof.reward_disbursed = 0;
        proof.bump = ctx.bumps.proof;

        ambassador.total_submissions = ambassador
            .total_submissions
            .checked_add(1)
            .ok_or(PromoError::MathOverflow)?;

        emit!(ContentProofSubmitted {
            creator: ctx.accounts.creator.key(),
            content_hash,
            platform_id,
        });

        Ok(())
    }

    /// Oracle/Admin verified trigger that disburses SPL tokens to creator & awards VIP badge status.
    pub fn verify_and_disburse(
        ctx: Context<VerifyAndDisburse>,
        content_hash: [u8; 32],
    ) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let proof = &mut ctx.accounts.proof;
        let ambassador = &mut ctx.accounts.ambassador;

        require!(!proof.verified, PromoError::AlreadyVerified);
        require!(proof.content_hash == content_hash, PromoError::HashMismatch);

        let reward_amount = campaign.reward_per_submission;
        require!(
            ctx.accounts.vault.amount >= reward_amount,
            PromoError::InsufficientVaultLiquidity
        );

        // Perform Vault CPI transfer using PDA signer seeds
        let seeds = &[
            VAULT_SEED,
            campaign.to_account_info().key.as_ref(),
            &[campaign.vault_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.creator_token_account.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer,
        );
        token::transfer(cpi_ctx, reward_amount)?;

        // Update Accounting Records
        proof.verified = true;
        proof.reward_disbursed = reward_amount;

        ambassador.approved_submissions = ambassador
            .approved_submissions
            .checked_add(1)
            .ok_or(PromoError::MathOverflow)?;

        ambassador.total_rewards_earned = ambassador
            .total_rewards_earned
            .checked_add(reward_amount)
            .ok_or(PromoError::MathOverflow)?;

        campaign.total_distributed = campaign
            .total_distributed
            .checked_add(reward_amount)
            .ok_or(PromoError::MathOverflow)?;

        // Check if creator meets VIP Pass threshold for El Encuentro 2026 (>= 3 verified posts)
        if ambassador.approved_submissions >= 3 && !ambassador.vip_pass_awarded {
            ambassador.vip_pass_awarded = true;
            campaign.total_verified_creators = campaign
                .total_verified_creators
                .checked_add(1)
                .ok_or(PromoError::MathOverflow)?;

            emit!(VipPassUnlocked {
                creator: ambassador.creator,
                handle: ambassador.handle.clone(),
                timestamp: Clock::get()?.unix_timestamp,
            });
        }

        emit!(RewardDisbursed {
            creator: ambassador.creator,
            amount: reward_amount,
            content_hash,
        });

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Account Contexts
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeCampaign<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + CampaignState::INIT_SPACE,
        seeds = [CAMPAIGN_SEED],
        bump
    )]
    pub campaign: Account<'info, CampaignState>,

    pub reward_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = admin,
        seeds = [VAULT_SEED, campaign.key().as_ref()],
        bump,
        token::mint = reward_mint,
        token::authority = vault,
    )]
    pub vault: Account<'info, TokenAccount>,

    /// CHECK: Trusted verifier authority key
    pub verifier: UncheckedAccount<'info>,

    #[account(mut)]
    pub admin: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(handle: String)]
pub struct RegisterAmbassador<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + AmbassadorState::INIT_SPACE,
        seeds = [AMBASSADOR_SEED, creator.key().as_ref()],
        bump
    )]
    pub ambassador: Account<'info, AmbassadorState>,

    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(content_hash: [u8; 32], platform_id: u8)]
pub struct SubmitContentProof<'info> {
    #[account(
        seeds = [CAMPAIGN_SEED],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignState>,

    #[account(
        mut,
        seeds = [AMBASSADOR_SEED, creator.key().as_ref()],
        bump = ambassador.bump,
        has_one = creator
    )]
    pub ambassador: Account<'info, AmbassadorState>,

    #[account(
        init,
        payer = creator,
        space = 8 + ContentReceipt::INIT_SPACE,
        seeds = [PROOF_SEED, creator.key().as_ref(), &content_hash],
        bump
    )]
    pub proof: Account<'info, ContentReceipt>,

    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(content_hash: [u8; 32])]
pub struct VerifyAndDisburse<'info> {
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED],
        bump = campaign.bump,
        has_one = verifier,
        has_one = vault
    )]
    pub campaign: Account<'info, CampaignState>,

    #[account(
        mut,
        seeds = [AMBASSADOR_SEED, ambassador.creator.as_ref()],
        bump = ambassador.bump
    )]
    pub ambassador: Account<'info, AmbassadorState>,

    #[account(
        mut,
        seeds = [PROOF_SEED, ambassador.creator.as_ref(), &content_hash],
        bump = proof.bump
    )]
    pub proof: Account<'info, ContentReceipt>,

    #[account(
        mut,
        seeds = [VAULT_SEED, campaign.key().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = creator_token_account.owner == ambassador.creator,
        constraint = creator_token_account.mint == campaign.reward_mint
    )]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub verifier: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// ---------------------------------------------------------------------------
// State Structures
// ---------------------------------------------------------------------------

#[account]
#[derive(InitSpace)]
pub struct CampaignState {
    pub admin: Pubkey,
    pub verifier: Pubkey,
    pub reward_mint: Pubkey,
    pub vault: Pubkey,
    pub reward_per_submission: u64,
    pub max_submissions_per_creator: u32,
    pub total_distributed: u64,
    pub total_verified_creators: u32,
    pub bump: u8,
    pub vault_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AmbassadorState {
    pub creator: Pubkey,
    #[max_len(32)]
    pub handle: String,
    pub total_submissions: u32,
    pub approved_submissions: u32,
    pub total_rewards_earned: u64,
    pub vip_pass_awarded: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ContentReceipt {
    pub creator: Pubkey,
    pub content_hash: [u8; 32],
    pub platform_id: u8,
    pub verified: bool,
    pub submission_timestamp: i64,
    pub reward_disbursed: u64,
    pub bump: u8,
}

// ---------------------------------------------------------------------------
// Events & Errors
// ---------------------------------------------------------------------------

#[event]
pub struct CampaignInitialized {
    pub admin: Pubkey,
    pub verifier: Pubkey,
    pub reward_per_submission: u64,
}

#[event]
pub struct AmbassadorRegistered {
    pub creator: Pubkey,
    pub handle: String,
}

#[event]
pub struct ContentProofSubmitted {
    pub creator: Pubkey,
    pub content_hash: [u8; 32],
    pub platform_id: u8,
}

#[event]
pub struct RewardDisbursed {
    pub creator: Pubkey,
    pub amount: u64,
    pub content_hash: [u8; 32],
}

#[event]
pub struct VipPassUnlocked {
    pub creator: Pubkey,
    pub handle: String,
    pub timestamp: i64,
}

#[error_code]
pub enum PromoError {
    #[msg("Reward amount must be greater than zero.")]
    InvalidRewardAmount,
    #[msg("Maximum submissions must be greater than zero.")]
    InvalidMaxSubmissions,
    #[msg("Ambassador handle must be between 3 and 32 characters.")]
    InvalidHandleLength,
    #[msg("Ambassador has reached the maximum submission limit.")]
    MaxSubmissionsExceeded,
    #[msg("This content proof has already been verified and rewarded.")]
    AlreadyVerified,
    #[msg("Content hash mismatch against the registered proof receipt.")]
    HashMismatch,
    #[msg("Insufficient balance in the campaign rewards vault.")]
    InsufficientVaultLiquidity,
    #[msg("Math arithmetic overflow occurred.")]
    MathOverflow,
}