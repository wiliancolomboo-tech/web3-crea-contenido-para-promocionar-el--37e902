use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

pub mod errors;
pub mod events;
pub mod state;

use errors::EncuentroError;
use events::*;
use state::*;

declare_id!("Encu26MadridPr0tocoL111111111111111111111111");

#[program]
pub mod encuentro_protocol {
    use super::*;

    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        campaign_id: u64,
        base_ticket_price_lamports: u64,
        advocate_commission_bps: u16,
        buyer_discount_bps: u16,
        max_capacity: u32,
    ) -> Result<()> {
        let config = &mut ctx.accounts.campaign_config;
        config.authority = ctx.accounts.authority.key();
        config.treasury = ctx.accounts.treasury.key();
        config.campaign_id = campaign_id;
        config.base_ticket_price_lamports = base_ticket_price_lamports;
        config.advocate_commission_bps = advocate_commission_bps;
        config.buyer_discount_bps = buyer_discount_bps;
        config.total_tickets_sold = 0;
        config.max_capacity = max_capacity;
        config.is_active = true;
        config.bump = ctx.bumps.campaign_config;

        emit!(CampaignInitializedEvent {
            campaign_id,
            authority: config.authority,
            treasury: config.treasury,
            base_ticket_price_lamports,
            max_capacity,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    pub fn register_advocate(
        ctx: Context<RegisterAdvocate>,
        campaign_id: u64,
        referral_code_hash: [u8; 32],
        tier: u8,
    ) -> Result<()> {
        let advocate = &mut ctx.accounts.advocate_profile;
        advocate.campaign_id = campaign_id;
        advocate.advocate_wallet = ctx.accounts.advocate_wallet.key();
        advocate.referral_code_hash = referral_code_hash;
        advocate.referrals_count = 0;
        advocate.total_earnings_lamports = 0;
        advocate.tier = tier;
        advocate.bump = ctx.bumps.advocate_profile;

        emit!(AdvocateRegisteredEvent {
            campaign_id,
            advocate_pubkey: ctx.accounts.advocate_wallet.key(),
            referral_code_hash,
            tier,
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    pub fn purchase_ticket_with_advocate(
        ctx: Context<PurchaseTicketWithAdvocate>,
        ticket_id: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.campaign_config;
        let advocate = &mut ctx.accounts.advocate_profile;
        let clock = Clock::get()?;

        require!(config.is_active, EncuentroError::CampaignInactive);
        require!(config.total_tickets_sold < config.max_capacity, EncuentroError::SoldOut);

        // Compute discounted ticket price
        let discount = (config.base_ticket_price_lamports as u128)
            .checked_mul(config.buyer_discount_bps as u128)
            .ok_or(EncuentroError::NumericalOverflow)?
            .checked_div(10000)
            .ok_or(EncuentroError::NumericalOverflow)? as u64;

        let final_price = config
            .base_ticket_price_lamports
            .checked_sub(discount)
            .ok_or(EncuentroError::NumericalOverflow)?;

        // Compute advocate reward cut
        let advocate_reward = (final_price as u128)
            .checked_mul(config.advocate_commission_bps as u128)
            .ok_or(EncuentroError::NumericalOverflow)?
            .checked_div(10000)
            .ok_or(EncuentroError::NumericalOverflow)? as u64;

        let treasury_amount = final_price
            .checked_sub(advocate_reward)
            .ok_or(EncuentroError::NumericalOverflow)?;

        // 1. Transfer advocate commission directly to creator
        if advocate_reward > 0 {
            transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.buyer.to_account_info(),
                        to: ctx.accounts.advocate_payout.to_account_info(),
                    },
                ),
                advocate_reward,
            )?;
        }

        // 2. Transfer remainder directly to Madrid 2026 Treasury
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            treasury_amount,
        )?;

        // Update state
        config.total_tickets_sold = config
            .total_tickets_sold
            .checked_add(1)
            .ok_or(EncuentroError::NumericalOverflow)?;

        advocate.referrals_count = advocate
            .referrals_count
            .checked_add(1)
            .ok_or(EncuentroError::NumericalOverflow)?;

        advocate.total_earnings_lamports = advocate
            .total_earnings_lamports
            .checked_add(advocate_reward)
            .ok_or(EncuentroError::NumericalOverflow)?;

        // Initialize Ticket Pass PDA
        let ticket_pass = &mut ctx.accounts.ticket_pass;
        ticket_pass.ticket_id = ticket_id;
        ticket_pass.campaign_id = config.campaign_id;
        ticket_pass.attendee_wallet = ctx.accounts.buyer.key();
        ticket_pass.advocate_wallet = advocate.advocate_wallet;
        ticket_pass.purchase_price_lamports = final_price;
        ticket_pass.is_checked_in = false;
        ticket_pass.check_in_timestamp = 0;
        ticket_pass.bump = ctx.bumps.ticket_pass;

        emit!(TicketPurchasedEvent {
            campaign_id: config.campaign_id,
            buyer: ctx.accounts.buyer.key(),
            advocate: advocate.advocate_wallet,
            ticket_id,
            amount_paid_lamports: final_price,
            advocate_reward_lamports: advocate_reward,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn check_in_attendee(
        ctx: Context<CheckInAttendee>,
        _ticket_id: u64,
    ) -> Result<()> {
        let ticket = &mut ctx.accounts.ticket_pass;
        let clock = Clock::get()?;

        require!(!ticket.is_checked_in, EncuentroError::TicketAlreadyRedeemed);
        ticket.is_checked_in = true;
        ticket.check_in_timestamp = clock.unix_timestamp;

        emit!(TicketCheckedInEvent {
            ticket_id: ticket.ticket_id,
            attendee: ticket.attendee_wallet,
            checked_in_by: ctx.accounts.authority.key(),
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct InitializeCampaign<'info> {
    #[account(
        init,
        payer = authority,
        space = CampaignConfig::LEN,
        seeds = [b"campaign", campaign_id.to_le_bytes().as_ref()],
        bump
    )]
    pub campaign_config: Account<'info, CampaignConfig>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Safe treasury address receiving event ticketing proceeds
    pub treasury: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(campaign_id: u64)]
pub struct RegisterAdvocate<'info> {
    #[account(
        init,
        payer = advocate_wallet,
        space = AdvocateProfile::LEN,
        seeds = [b"advocate", campaign_id.to_le_bytes().as_ref(), advocate_wallet.key().as_ref()],
        bump
    )]
    pub advocate_profile: Account<'info, AdvocateProfile>,
    #[account(mut)]
    pub advocate_wallet: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(ticket_id: u64)]
pub struct PurchaseTicketWithAdvocate<'info> {
    #[account(
        mut,
        seeds = [b"campaign", campaign_config.campaign_id.to_le_bytes().as_ref()],
        bump = campaign_config.bump
    )]
    pub campaign_config: Account<'info, CampaignConfig>,

    #[account(
        mut,
        seeds = [b"advocate", campaign_config.campaign_id.to_le_bytes().as_ref(), advocate_profile.advocate_wallet.as_ref()],
        bump = advocate_profile.bump
    )]
    pub advocate_profile: Account<'info, AdvocateProfile>,

    #[account(
        init,
        payer = buyer,
        space = TicketPass::LEN,
        seeds = [b"ticket", campaign_config.campaign_id.to_le_bytes().as_ref(), ticket_id.to_le_bytes().as_ref()],
        bump
    )]
    pub ticket_pass: Account<'info, TicketPass>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: Advocate wallet matching the registered profile to receive commission
    #[account(mut, address = advocate_profile.advocate_wallet)]
    pub advocate_payout: AccountInfo<'info>,

    /// CHECK: Treasury wallet configured on campaign initialization
    #[account(mut, address = campaign_config.treasury)]
    pub treasury: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(ticket_id: u64)]
pub struct CheckInAttendee<'info> {
    #[account(
        has_one = authority @ EncuentroError::UnauthorizedAdmin,
        seeds = [b"campaign", campaign_config.campaign_id.to_le_bytes().as_ref()],
        bump = campaign_config.bump
    )]
    pub campaign_config: Account<'info, CampaignConfig>,

    #[account(
        mut,
        seeds = [b"ticket", campaign_config.campaign_id.to_le_bytes().as_ref(), ticket_id.to_le_bytes().as_ref()],
        bump = ticket_pass.bump
    )]
    pub ticket_pass: Account<'info, TicketPass>,

    pub authority: Signer<'info>,
}