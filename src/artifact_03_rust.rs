use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("Encu26MadR1d11111111111111111111111111111111");

pub const CAMPAIGN_SEED: &[u8] = b"encuentro_campaign";
pub const SUBMISSION_SEED: &[u8] = b"content_submission";
pub const VAULT_SEED: &[u8] = b"vault_escrow";

#[program]
pub mod encuentro_boost {
    use super::*;

    /// Initialize a promotional campaign (e.g., El Encuentro 2026 Madrid)
    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        campaign_name: String,
        reward_per_submission: u64,
        max_submissions: u32,
        initial_deposit: u64,
    ) -> Result<()> {
        require!(
            campaign_name.len() <= 64,
            EncuentroError::CampaignNameTooLong
        );
        require!(
            reward_per_submission > 0,
            EncuentroError::InvalidRewardAmount
        );
        require!(max_submissions > 0, EncuentroError::InvalidMaxSubmissions);

        let campaign = &mut ctx.accounts.campaign;
        campaign.authority = ctx.accounts.authority.key();
        campaign.oracle_signer = ctx.accounts.oracle_signer.key();
        campaign.campaign_name = campaign_name;
        campaign.reward_per_submission = reward_per_submission;
        campaign.max_submissions = max_submissions;
        campaign.total_submissions = 0;
        campaign.total_payouts_distributed = 0;
        campaign.is_active = true;
        campaign.bump = ctx.bumps.campaign;
        campaign.vault_bump = ctx.bumps.vault;

        if initial_deposit > 0 {
            let cpi_context = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.authority.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            );
            transfer(cpi_context, initial_deposit)?;
        }

        emit!(CampaignInitializedEvent {
            campaign: campaign.key(),
            authority: campaign.authority,
            reward_per_submission,
            max_submissions,
        });

        Ok(())
    }

    /// Deposit funds into campaign vault
    pub fn fund_campaign(ctx: Context<FundCampaign>, amount: u64) -> Result<()> {
        require!(amount > 0, EncuentroError::InvalidDepositAmount);

        let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.funder.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
        );
        transfer(cpi_context, amount)?;

        emit!(CampaignFundedEvent {
            campaign: ctx.accounts.campaign.key(),
            funder: ctx.accounts.funder.key(),
            amount,
        });

        Ok(())
    }

    /// Content creator registers their promotional Madrid 2026 content
    pub fn submit_content(
        ctx: Context<SubmitContent>,
        content_hash: [u8; 32],
        content_uri: String,
    ) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        require!(campaign.is_active, EncuentroError::CampaignInactive);
        require!(
            campaign.total_submissions < campaign.max_submissions,
            EncuentroError::CampaignSubmissionLimitReached
        );
        require!(
            content_uri.len() <= 128,
            EncuentroError::ContentUriTooLong
        );

        let submission = &mut ctx.accounts.submission;
        submission.campaign = campaign.key();
        submission.creator = ctx.accounts.creator.key();
        submission.content_hash = content_hash;
        submission.content_uri = content_uri.clone();
        submission.timestamp = Clock::get()?.unix_timestamp;
        submission.verified = false;
        submission.rewarded = false;
        submission.bump = ctx.bumps.submission;

        campaign.total_submissions = campaign
            .total_submissions
            .checked_add(1)
            .ok_or(EncuentroError::MathOverflow)?;

        emit!(ContentSubmittedEvent {
            campaign: campaign.key(),
            creator: ctx.accounts.creator.key(),
            content_hash,
            content_uri,
        });

        Ok(())
    }

    /// Oracle verifies engagement metrics and releases the reward to the creator
    pub fn verify_and_reward(ctx: Context<VerifyAndReward>) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let submission = &mut ctx.accounts.submission;

        require!(campaign.is_active, EncuentroError::CampaignInactive);
        require!(!submission.verified, EncuentroError::AlreadyVerified);
        require!(!submission.rewarded, EncuentroError::AlreadyRewarded);

        let reward = campaign.reward_per_submission;
        let vault_lamports = ctx.accounts.vault.to_account_info().lamports();
        require!(vault_lamports >= reward, EncuentroError::InsufficientVaultFunds);

        submission.verified = true;
        submission.rewarded = true;

        campaign.total_payouts_distributed = campaign
            .total_payouts_distributed
            .checked_add(reward)
            .ok_or(EncuentroError::MathOverflow)?;

        let campaign_key = campaign.key();
        let vault_bump = campaign.vault_bump;
        let seeds = &[
            VAULT_SEED,
            campaign_key.as_ref(),
            &[vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? = vault_lamports
            .checked_sub(reward)
            .ok_or(EncuentroError::MathOverflow)?;

        **ctx.accounts.creator.to_account_info().try_borrow_mut_lamports()? = ctx
            .accounts
            .creator
            .to_account_info()
            .lamports()
            .checked_add(reward)
            .ok_or(EncuentroError::MathOverflow)?;

        emit!(ContentRewardedEvent {
            campaign: campaign.key(),
            creator: ctx.accounts.creator.key(),
            submission: submission.key(),
            reward_amount: reward,
        });

        Ok(())
    }

    /// Reclaim remaining balance by campaign authority
    pub fn withdraw_surplus(ctx: Context<WithdrawSurplus>, amount: u64) -> Result<()> {
        let vault_lamports = ctx.accounts.vault.to_account_info().lamports();
        require!(vault_lamports >= amount, EncuentroError::InsufficientVaultFunds);

        let campaign_key = ctx.accounts.campaign.key();
        let vault_bump = ctx.accounts.campaign.vault_bump;
        let seeds = &[
            VAULT_SEED,
            campaign_key.as_ref(),
            &[vault_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? = vault_lamports
            .checked_sub(amount)
            .ok_or(EncuentroError::MathOverflow)?;

        **ctx.accounts.authority.to_account_info().try_borrow_mut_lamports()? = ctx
            .accounts
            .authority
            .to_account_info()
            .lamports()
            .checked_add(amount)
            .ok_or(EncuentroError::MathOverflow)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(campaign_name: String)]
pub struct InitializeCampaign<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + CampaignState::INIT_SPACE,
        seeds = [CAMPAIGN_SEED, campaign_name.as_bytes()],
        bump
    )]
    pub campaign: Account<'info, CampaignState>,

    /// CHECK: Safe PDA Vault for holding campaign reward funds
    #[account(
        mut,
        seeds = [VAULT_SEED, campaign.key().as_ref()],
        bump
    )]
    pub vault: AccountInfo<'info>,

    /// CHECK: Public key authorized to attest and trigger verification
    pub oracle_signer: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct FundCampaign<'info> {
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.campaign_name.as_bytes()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignState>,

    /// CHECK: PDA Vault receiving funds
    #[account(
        mut,
        seeds = [VAULT_SEED, campaign.key().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(mut)]
    pub funder: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(content_hash: [u8; 32])]
pub struct SubmitContent<'info> {
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.campaign_name.as_bytes()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignState>,

    #[account(
        init,
        payer = creator,
        space = 8 + ContentSubmissionState::INIT_SPACE,
        seeds = [SUBMISSION_SEED, campaign.key().as_ref(), content_hash.as_ref()],
        bump
    )]
    pub submission: Account<'info, ContentSubmissionState>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyAndReward<'info> {
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.campaign_name.as_bytes()],
        bump = campaign.bump,
        has_one = oracle_signer @ EncuentroError::UnauthorizedOracle
    )]
    pub campaign: Account<'info, CampaignState>,

    #[account(
        mut,
        seeds = [SUBMISSION_SEED, campaign.key().as_ref(), submission.content_hash.as_ref()],
        bump = submission.bump,
        has_one = campaign @ EncuentroError::CampaignMismatch
    )]
    pub submission: Account<'info, ContentSubmissionState>,

    /// CHECK: Creator account destination for reward
    #[account(
        mut,
        constraint = creator.key() == submission.creator @ EncuentroError::CreatorMismatch
    )]
    pub creator: AccountInfo<'info>,

    /// CHECK: PDA Vault releasing rewards
    #[account(
        mut,
        seeds = [VAULT_SEED, campaign.key().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: AccountInfo<'info>,

    pub oracle_signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawSurplus<'info> {
    #[account(
        mut,
        seeds = [CAMPAIGN_SEED, campaign.campaign_name.as_bytes()],
        bump = campaign.bump,
        has_one = authority @ EncuentroError::UnauthorizedAuthority
    )]
    pub campaign: Account<'info, CampaignState>,

    /// CHECK: PDA Vault
    #[account(
        mut,
        seeds = [VAULT_SEED, campaign.key().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace)]
pub struct CampaignState {
    pub authority: Pubkey,
    pub oracle_signer: Pubkey,
    #[max_len(64)]
    pub campaign_name: String,
    pub reward_per_submission: u64,
    pub max_submissions: u32,
    pub total_submissions: u32,
    pub total_payouts_distributed: u64,
    pub is_active: bool,
    pub bump: u8,
    pub vault_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct ContentSubmissionState {
    pub campaign: Pubkey,
    pub creator: Pubkey,
    pub content_hash: [u8; 32],
    #[max_len(128)]
    pub content_uri: String,
    pub timestamp: i64,
    pub verified: bool,
    pub rewarded: bool,
    pub bump: u8,
}

#[event]
pub struct CampaignInitializedEvent {
    pub campaign: Pubkey,
    pub authority: Pubkey,
    pub reward_per_submission: u64,
    pub max_submissions: u32,
}

#[event]
pub struct CampaignFundedEvent {
    pub campaign: Pubkey,
    pub funder: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ContentSubmittedEvent {
    pub campaign: Pubkey,
    pub creator: Pubkey,
    pub content_hash: [u8; 32],
    pub content_uri: String,
}

#[event]
pub struct ContentRewardedEvent {
    pub campaign: Pubkey,
    pub creator: Pubkey,
    pub submission: Pubkey,
    pub reward_amount: u64,
}

#[error_code]
pub enum EncuentroError {
    #[msg("Campaign name exceeds maximum length of 64 bytes")]
    CampaignNameTooLong,
    #[msg("Reward per submission must be greater than zero")]
    InvalidRewardAmount,
    #[msg("Max submissions must be greater than zero")]
    InvalidMaxSubmissions,
    #[msg("Deposit amount must be greater than zero")]
    InvalidDepositAmount,
    #[msg("Content URI exceeds maximum length of 128 bytes")]
    ContentUriTooLong,
    #[msg("Campaign is currently inactive")]
    CampaignInactive,
    #[msg("Campaign maximum submissions limit has been reached")]
    CampaignSubmissionLimitReached,
    #[msg("Submission is already verified")]
    AlreadyVerified,
    #[msg("Submission reward has already been claimed")]
    AlreadyRewarded,
    #[msg("Vault has insufficient balance to fulfill reward")]
    InsufficientVaultFunds,
    #[msg("Unauthorized Oracle Signer")]
    UnauthorizedOracle,
    #[msg("Unauthorized Campaign Authority")]
    UnauthorizedAuthority,
    #[msg("Submission does not belong to this campaign")]
    CampaignMismatch,
    #[msg("Target creator address does not match submission creator")]
    CreatorMismatch,
    #[msg("Numerical overflow detected during math operation")]
    MathOverflow,
}