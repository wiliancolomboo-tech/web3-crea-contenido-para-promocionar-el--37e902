use anchor_lang::prelude::*;
use crate::state::{Campaign, Submission};
use crate::errors::EncuentroError;

#[derive(Accounts)]
#[instruction(content_hash: [u8; 32])]
pub struct SubmitContent<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        constraint = campaign.is_active @ EncuentroError::CampaignInactive
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(
        init,
        payer = creator,
        space = Submission::LEN,
        seeds = [b"submission", campaign.key().as_ref(), content_hash.as_ref()],
        bump
    )]
    pub submission: Account<'info, Submission>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<SubmitContent>,
    content_hash: [u8; 32],
    platform_id: u8,
) -> Result<()> {
    let clock = Clock::get()?;
    let submission = &mut ctx.accounts.submission;
    submission.creator = ctx.accounts.creator.key();
    submission.campaign = ctx.accounts.campaign.key();
    submission.content_hash = content_hash;
    submission.platform_id = platform_id;
    submission.tier_awarded = 0;
    submission.reward_amount = 0;
    submission.is_verified = false;
    submission.submission_time = clock.unix_timestamp;
    submission.verified_time = 0;
    submission.bump = ctx.bumps.submission;

    let campaign = &mut ctx.accounts.campaign;
    campaign.total_submissions = campaign
        .total_submissions
        .checked_add(1)
        .ok_or(EncuentroError::MathOverflow)?;

    msg!("New promotional content submitted for El Encuentro 2026! Platform ID: {}", platform_id);
    Ok(())
}