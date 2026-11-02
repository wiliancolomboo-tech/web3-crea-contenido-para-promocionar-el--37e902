use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("MDRD26x9eQJqF8sWbK3qY2PjT4rZ8uV1mN5cE7wA3vL");

#[program]
pub mod madrid26_core {
    use super::*;

    /// Initializes the El Encuentro Madrid 2026 Global Event State.
    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        ticket_price_lamports: u64,
        advocate_fee_bps: u16, // e.g. 1000 = 10%
    ) -> Result<()> {
        require!(advocate_fee_bps <= 5000, MadridError::FeeTooHigh); // Max 50%
        require!(ticket_price_lamports > 0, MadridError::InvalidPrice);

        let campaign = &mut ctx.accounts.campaign;
        campaign.organizer = ctx.accounts.organizer.key();
        campaign.treasury = ctx.accounts.treasury.key();
        campaign.ticket_price = ticket_price_lamports;
        campaign.advocate_fee_bps = advocate_fee_bps;
        campaign.total_tickets_sold = 0;
        campaign.total_volume = 0;
        campaign.bump = ctx.bumps.campaign;

        emit!(CampaignInitialized {
            organizer: campaign.organizer,
            treasury: campaign.treasury,
            ticket_price: ticket_price_lamports,
            advocate_fee_bps,
        });

        Ok(())
    }

    /// Registers a Madrid 2026 Advocate/Creator and creates their deterministic referral PDA.
    pub fn register_advocate(
        ctx: Context<RegisterAdvocate>,
        referral_code: String,
    ) -> Result<()> {
        require!(referral_code.len() >= 3 && referral_code.len() <= 32, MadridError::InvalidCodeLength);

        let advocate_record = &mut ctx.accounts.advocate_record;
        advocate_record.advocate = ctx.accounts.advocate.key();
        advocate_record.campaign = ctx.accounts.campaign.key();
        advocate_record.referral_code = referral_code.clone();
        advocate_record.tickets_referred = 0;
        advocate_record.total_earned = 0;
        advocate_record.bump = ctx.bumps.advocate_record;

        emit!(AdvocateRegistered {
            advocate: ctx.accounts.advocate.key(),
            referral_code,
        });

        Ok(())
    }

    /// Purchases an El Encuentro Madrid 2026 ticket through an advocate's link.
    /// Splits payment trustlessly between event treasury and creator wallet.
    pub fn purchase_promoted_ticket(
        ctx: Context<PurchasePromotedTicket>,
        attendee_email_hash: [u8; 32],
    ) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        let advocate_record = &mut ctx.accounts.advocate_record;

        let ticket_price = campaign.ticket_price;
        let advocate_amount = (ticket_price as u128)
            .checked_mul(campaign.advocate_fee_bps as u128)
            .ok_or(MadridError::MathOverflow)?
            .checked_div(10000)
            .ok_or(MadridError::MathOverflow)? as u64;

        let treasury_amount = ticket_price
            .checked_sub(advocate_amount)
            .ok_or(MadridError::MathOverflow)?;

        // 1. Transfer referral rebate directly to advocate
        if advocate_amount > 0 {
            let cpi_advocate = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.advocate_wallet.to_account_info(),
                },
            );
            transfer(cpi_advocate, advocate_amount)?;
        }

        // 2. Transfer remainder to Madrid 2026 Event Treasury
        let cpi_treasury = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
            },
        );
        transfer(cpi_treasury, treasury_amount)?;

        // 3. Update State
        campaign.total_tickets_sold = campaign.total_tickets_sold
            .checked_add(1)
            .ok_or(MadridError::MathOverflow)?;
        campaign.total_volume = campaign.total_volume
            .checked_add(ticket_price)
            .ok_or(MadridError::MathOverflow)?;

        advocate_record.tickets_referred = advocate_record.tickets_referred
            .checked_add(1)
            .ok_or(MadridError::MathOverflow)?;
        advocate_record.total_earned = advocate_record.total_earned
            .checked_add(advocate_amount)
            .ok_or(MadridError::MathOverflow)?;

        // 4. Emit execution log for backend worker
        emit!(TicketPurchased {
            campaign: campaign.key(),
            buyer: ctx.accounts.buyer.key(),
            advocate: advocate_record.advocate,
            referral_code: advocate_record.referral_code.clone(),
            ticket_price,
            advocate_earned: advocate_amount,
            email_hash: attendee_email_hash,
            ticket_number: campaign.total_tickets_sold,
        });

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// ACCOUNT VALIDATION STRUCTS
// -----------------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeCampaign<'info> {
    #[account(
        init,
        payer = organizer,
        space = 8 + CampaignState::INIT_SPACE,
        seeds = [b"madrid2026_campaign", organizer.key().as_ref()],
        bump
    )]
    pub campaign: Account<'info, CampaignState>,

    /// CHECK: Validated as designated treasury receiver
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub organizer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(referral_code: String)]
pub struct RegisterAdvocate<'info> {
    #[account(
        init,
        payer = advocate,
        space = 8 + AdvocateRecord::INIT_SPACE,
        seeds = [b"madrid2026_advocate", campaign.key().as_ref(), referral_code.as_bytes()],
        bump
    )]
    pub advocate_record: Account<'info, AdvocateRecord>,

    pub campaign: Account<'info, CampaignState>,

    #[account(mut)]
    pub advocate: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct PurchasePromotedTicket<'info> {
    #[account(
        mut,
        seeds = [b"madrid2026_campaign", campaign.organizer.as_ref()],
        bump = campaign.bump
    )]
    pub campaign: Account<'info, CampaignState>,

    #[account(
        mut,
        seeds = [b"madrid2026_advocate", campaign.key().as_ref(), advocate_record.referral_code.as_bytes()],
        bump = advocate_record.bump,
        has_one = campaign
    )]
    pub advocate_record: Account<'info, AdvocateRecord>,

    /// CHECK: Must match the advocate registered in the advocate_record
    #[account(
        mut,
        constraint = advocate_wallet.key() == advocate_record.advocate @ MadridError::InvalidAdvocateWallet
    )]
    pub advocate_wallet: AccountInfo<'info>,

    /// CHECK: Must match campaign treasury
    #[account(
        mut,
        constraint = treasury.key() == campaign.treasury @ MadridError::InvalidTreasuryWallet
    )]
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// -----------------------------------------------------------------------------
// STATE STRUCTS
// -----------------------------------------------------------------------------

#[account]
#[derive(InitSpace)]
pub struct CampaignState {
    pub organizer: Pubkey,
    pub treasury: Pubkey,
    pub ticket_price: u64,
    pub advocate_fee_bps: u16,
    pub total_tickets_sold: u64,
    pub total_volume: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AdvocateRecord {
    pub advocate: Pubkey,
    pub campaign: Pubkey,
    #[max_len(32)]
    pub referral_code: String,
    pub tickets_referred: u64,
    pub total_earned: u64,
    pub bump: u8,
}

// -----------------------------------------------------------------------------
// EVENTS & ERRORS
// -----------------------------------------------------------------------------

#[event]
pub mod events {
    use super::*;

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct CampaignInitialized {
        pub organizer: Pubkey,
        pub treasury: Pubkey,
        pub ticket_price: u64,
        pub advocate_fee_bps: u16,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct AdvocateRegistered {
        pub advocate: Pubkey,
        pub referral_code: String,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct TicketPurchased {
        pub campaign: Pubkey,
        pub buyer: Pubkey,
        pub advocate: Pubkey,
        pub referral_code: String,
        pub ticket_price: u64,
        pub advocate_earned: u64,
        pub email_hash: [u8; 32],
        pub ticket_number: u64,
    }
}

pub use events::*;

#[error_code]
pub enum MadridError {
    #[msg("Advocate commission percentage exceeds 5000 bps (50%).")]
    FeeTooHigh,
    #[msg("Ticket price must be greater than zero.")]
    InvalidPrice,
    #[msg("Referral code must be between 3 and 32 characters.")]
    InvalidCodeLength,
    #[msg("Target advocate wallet does not match registered advocate PDA.")]
    InvalidAdvocateWallet,
    #[msg("Target treasury does not match campaign treasury.")]
    InvalidTreasuryWallet,
    #[msg("Numerical overflow detected during commission calculations.")]
    MathOverflow,
}