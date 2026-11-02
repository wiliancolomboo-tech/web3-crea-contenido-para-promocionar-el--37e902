use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{Campaign, Submission};
use crate::errors::EncuentroError;

#[derive(Accounts)]
pub struct VerifyAndReward<'info> {
    #[account(
        mut,
        address = campaign.oracle_authority @ EncuentroError::UnauthorizedSigner
    )]
    pub oracle: Signer<'info>,

    #[account(mut)]
    pub campaign: Account<'info, Campaign>,

    #[account(
        mut,
        seeds = [b"submission", campaign.key().as_ref(), submission.content_hash.as_ref()],
        bump = submission.bump,
        constraint = !submission.is_verified @ EncuentroError::SubmissionAlreadyProcessed,
        constraint = submission.campaign == campaign.key()
    )]
    pub submission: Account<'info, Submission>,

    #[account(
        mut,
        seeds = [b"escrow-vault", campaign.key().as_ref()],
        bump = campaign.escrow_bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = creator_token_account.owner == submission.creator
    )]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(
    ctx: Context<VerifyAndReward>,
    tier: u8,
    reward_amount: u64,
) -> Result<()> {
    require!(tier >= 1 && tier <= 4, EncuentroError::InvalidRewardTier);

    let campaign = &mut ctx.accounts.campaign;
    let available_funds = campaign
        .total_allocated
        .checked_sub(campaign.total_distributed)
        .ok_or(EncuentroError::MathOverflow)?;

    require!(available_funds >= reward_amount, EncuentroError::InsufficientEscrowFunds);

    // CPI Token Transfer via Escrow PDA Signer
    let campaign_key = campaign.key();
    let seeds = &[
        b"escrow-vault",
        campaign_key.as_ref(),
        &[campaign.escrow_bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.escrow_vault.to_account_info(),
        to: ctx.accounts.creator_token_account.to_account_info(),
        authority: ctx.accounts.escrow_vault.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, reward_amount)?;

    // Update Program State
    let clock = Clock::get()?;
    let submission = &mut ctx.accounts.submission;
    submission.is_verified = true;
    submission.tier_awarded = tier;
    submission.reward_amount = reward_amount;
    submission.verified_time = clock.unix_timestamp;

    campaign.total_distributed = campaign
        .total_distributed
        .checked_add(reward_amount)
        .ok_or(EncuentroError::MathOverflow)?;

    msg!("Content verified! Tier {} awarded. Reward {} SPL transferred.", tier, reward_amount);
    Ok(())
}