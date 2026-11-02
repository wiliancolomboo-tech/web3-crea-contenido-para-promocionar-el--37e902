use anchor_lang::prelude::*;
use crate::state::{CampaignConfig, CreatorRecord};
use crate::errors::EncuentroError;
use crate::events::BountyClaimedEvent;

#[derive(Accounts)]
pub struct ClaimBounty<'info> {
    #[account(
        mut,
        seeds = [b"campaign", campaign.campaign_id.as_bytes()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignConfig>,

    #[account(
        mut,
        seeds = [b"vault", campaign.key().as_ref()],
        bump = campaign.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"creator", campaign.key().as_ref(), creator.key().as_ref()],
        bump = creator_record.bump
    )]
    pub creator_record: Account<'info, CreatorRecord>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimBounty>) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    let record = &mut ctx.accounts.creator_record;

    // Calculate entitlement based on verified submissions & cumulative reach
    let total_earned = (record.verified_submissions as u64)
        .checked_mul(campaign.bounty_rate_lamports)
        .ok_or(EncuentroError::MathOverflow)?;

    let claimable = total_earned
        .checked_sub(record.claimed_lamports)
        .ok_or(EncuentroError::MathOverflow)?;

    require!(claimable > 0, EncuentroError::InsufficientVaultBalance);

    let vault_lamports = ctx.accounts.vault.lamports();
    require!(vault_lamports >= claimable, EncuentroError::InsufficientVaultBalance);

    // Native Solana PDA Transfer from Vault to Creator
    let campaign_key = campaign.key();
    let vault_seeds: &[&[u8]] = &[
        b"vault",
        campaign_key.as_ref(),
        &[campaign.vault_bump],
    ];
    let signer_seeds = &[vault_seeds];

    let transfer_ix = solana_program::system_instruction::transfer(
        &ctx.accounts.vault.key(),
        &ctx.accounts.creator.key(),
        claimable,
    );

    solana_program::program::invoke_signed(
        &transfer_ix,
        &[
            ctx.accounts.vault.to_account_info(),
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        signer_seeds,
    )?;

    record.claimed_lamports = record
        .claimed_lamports
        .checked_add(claimable)
        .ok_or(EncuentroError::MathOverflow)?;

    campaign.total_funds_disbursed = campaign
        .total_funds_disbursed
        .checked_add(claimable)
        .ok_or(EncuentroError::MathOverflow)?;

    emit!(BountyClaimedEvent {
        campaign: campaign.key(),
        creator: ctx.accounts.creator.key(),
        amount_lamports: claimable,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}