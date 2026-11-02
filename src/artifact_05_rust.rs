use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("EnCuentroMadrid2026Promo11111111111111111");

pub const MAX_TAG_LEN: usize = 32;
pub const MAX_NAME_LEN: usize = 64;
pub const BASIS_POINTS_DIVISOR: u64 = 10_000;

#[program]
pub mod encuentro_promo {
    use super::*;

    /// Initializes the El Encuentro 2026 Madrid event master account.
    pub fn initialize_event(
        ctx: Context<InitializeEvent>,
        ticket_price_lamports: u64,
        referral_reward_bps: u64,
    ) -> Result<()> {
        require!(
            referral_reward_bps <= 3000,
            EncuentroError::ExcessiveReferralBps
        ); // Max 30% commission
        require!(ticket_price_lamports > 0, EncuentroError::InvalidTicketPrice);

        let event_config = &mut ctx.accounts.event_config;
        event_config.authority = ctx.accounts.authority.key();
        event_config.treasury = ctx.accounts.treasury.key();
        event_config.ticket_price_lamports = ticket_price_lamports;
        event_config.referral_reward_bps = referral_reward_bps;
        event_config.total_tickets_sold = 0;
        event_config.total_volume_lamports = 0;
        event_config.is_paused = false;
        event_config.bump = ctx.bumps.event_config;

        msg!("El Encuentro 2026 Madrid configuration initialized.");
        Ok(())
    }

    /// Registers a promoter/content creator profile with a custom referral handle.
    pub fn register_promoter(
        ctx: Context<RegisterPromoter>,
        promo_tag: String,
        display_name: String,
    ) -> Result<()> {
        require!(
            !promo_tag.is_empty() && promo_tag.len() <= MAX_TAG_LEN,
            EncuentroError::InvalidTagLength
        );
        require!(
            !display_name.is_empty() && display_name.len() <= MAX_NAME_LEN,
            EncuentroError::InvalidNameLength
        );

        let promoter_profile = &mut ctx.accounts.promoter_profile;
        promoter_profile.promoter_authority = ctx.accounts.promoter.key();
        promoter_profile.payout_destination = ctx.accounts.payout_destination.key();
        promoter_profile.promo_tag = promo_tag.clone();
        promoter_profile.display_name = display_name;
        promoter_profile.total_referrals = 0;
        promoter_profile.total_earned_lamports = 0;
        promoter_profile.created_at = Clock::get()?.unix_timestamp;
        promoter_profile.bump = ctx.bumps.promoter_profile;

        msg!("Promoter registered: Tag={}, Authority={}", promo_tag, ctx.accounts.promoter.key());
        Ok(())
    }

    /// Purchases an event ticket with attribution to a promoter.
    /// Executes an atomic fee split between creator and Madrid summit treasury.
    pub fn purchase_with_referral(
        ctx: Context<PurchaseWithReferral>,
        buyer_email_hash: [u8; 32],
    ) -> Result<()> {
        let event_config = &mut ctx.accounts.event_config;
        require!(!event_config.is_paused, EncuentroError::EventSalesPaused);

        let promoter_profile = &mut ctx.accounts.promoter_profile;
        let ticket_price = event_config.ticket_price_lamports;
        let commission_bps = event_config.referral_reward_bps;

        // Calculate atomic split amounts
        let creator_reward = ticket_price
            .checked_mul(commission_bps)
            .ok_or(EncuentroError::MathOverflow)?
            .checked_div(BASIS_POINTS_DIVISOR)
            .ok_or(EncuentroError::MathOverflow)?;

        let treasury_amount = ticket_price
            .checked_sub(creator_reward)
            .ok_or(EncuentroError::MathOverflow)?;

        // Execute Lamport transfer from buyer to creator payout wallet
        if creator_reward > 0 {
            transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.buyer.to_account_info(),
                        to: ctx.accounts.promoter_payout.to_account_info(),
                    },
                ),
                creator_reward,
            )?;
        }

        // Execute Lamport transfer from buyer to Madrid Summit Treasury
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

        // Update promoter metrics
        promoter_profile.total_referrals = promoter_profile
            .total_referrals
            .checked_add(1)
            .ok_or(EncuentroError::MathOverflow)?;
        promoter_profile.total_earned_lamports = promoter_profile
            .total_earned_lamports
            .checked_add(creator_reward)
            .ok_or(EncuentroError::MathOverflow)?;

        // Update global event metrics
        event_config.total_tickets_sold = event_config
            .total_tickets_sold
            .checked_add(1)
            .ok_or(EncuentroError::MathOverflow)?;
        event_config.total_volume_lamports = event_config
            .total_volume_lamports
            .checked_add(ticket_price)
            .ok_or(EncuentroError::MathOverflow)?;

        // Initialize ticket reservation record
        let reservation = &mut ctx.accounts.reservation;
        reservation.buyer = ctx.accounts.buyer.key();
        reservation.promoter = promoter_profile.key();
        reservation.email_hash = buyer_email_hash;
        reservation.paid_lamports = ticket_price;
        reservation.timestamp = Clock::get()?.unix_timestamp;
        reservation.bump = ctx.bumps.reservation;

        // Emit typed event for indexer to dispatch Resend email & Discord updates
        emit!(PromotionalConversionEvent {
            buyer: ctx.accounts.buyer.key(),
            promoter_authority: promoter_profile.promoter_authority,
            promo_tag: promoter_profile.promo_tag.clone(),
            email_hash: buyer_email_hash,
            ticket_price,
            creator_reward,
            treasury_amount,
            timestamp: Clock::get()?.unix_timestamp,
        });

        msg!(
            "Ticket purchased! Buyer={}, PromoTag={}, RewardLamports={}",
            ctx.accounts.buyer.key(),
            promoter_profile.promo_tag,
            creator_reward
        );

        Ok(())
    }

    /// Admin switch to pause or resume ticket sales.
    pub fn toggle_pause(ctx: Context<TogglePause>) -> Result<()> {
        let config = &mut ctx.accounts.event_config;
        config.is_paused = !config.is_paused;
        msg!("Sales active state changed to: {}", !config.is_paused);
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// ACCOUNT VALIDATION STRUCTS
// -----------------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeEvent<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + EventConfig::INIT_SPACE,
        seeds = [b"encuentro_2026_config"],
        bump
    )]
    pub event_config: Account<'info, EventConfig>,

    /// CHECK: Validated as summit treasury wallet receiving pass revenue
    pub treasury: AccountInfo<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(promo_tag: String)]
pub struct RegisterPromoter<'info> {
    #[account(
        init,
        payer = promoter,
        space = 8 + PromoterProfile::INIT_SPACE,
        seeds = [b"promoter_profile", promo_tag.as_bytes()],
        bump
    )]
    pub promoter_profile: Account<'info, PromoterProfile>,

    /// CHECK: Payout destination for commission splits
    pub payout_destination: AccountInfo<'info>,

    #[account(mut)]
    pub promoter: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(buyer_email_hash: [u8; 32])]
pub struct PurchaseWithReferral<'info> {
    #[account(
        mut,
        seeds = [b"encuentro_2026_config"],
        bump = event_config.bump,
        has_one = treasury @ EncuentroError::TreasuryMismatch
    )]
    pub event_config: Account<'info, EventConfig>,

    #[account(
        mut,
        seeds = [b"promoter_profile", promoter_profile.promo_tag.as_bytes()],
        bump = promoter_profile.bump,
        has_one = payout_destination @ EncuentroError::InvalidPayoutDestination
    )]
    pub promoter_profile: Account<'info, PromoterProfile>,

    #[account(
        init,
        payer = buyer,
        space = 8 + TicketReservation::INIT_SPACE,
        seeds = [
            b"ticket_reservation",
            event_config.key().as_ref(),
            buyer.key().as_ref()
        ],
        bump
    )]
    pub reservation: Account<'info, TicketReservation>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: Treasury receiving ticket share
    #[account(mut)]
    pub treasury: AccountInfo<'info>,

    /// CHECK: Promoter wallet receiving attribution share
    #[account(mut)]
    pub promoter_payout: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TogglePause<'info> {
    #[account(
        mut,
        seeds = [b"encuentro_2026_config"],
        bump = event_config.bump,
        has_one = authority @ EncuentroError::Unauthorized
    )]
    pub event_config: Account<'info, EventConfig>,

    pub authority: Signer<'info>,
}

// -----------------------------------------------------------------------------
// STATE SCHEMAS
// -----------------------------------------------------------------------------

#[account]
#[derive(InitSpace)]
pub struct EventConfig {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub ticket_price_lamports: u64,
    pub referral_reward_bps: u64,
    pub total_tickets_sold: u64,
    pub total_volume_lamports: u64,
    pub is_paused: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PromoterProfile {
    pub promoter_authority: Pubkey,
    pub payout_destination: Pubkey,
    #[max_len(32)]
    pub promo_tag: String,
    #[max_len(64)]
    pub display_name: String,
    pub total_referrals: u64,
    pub total_earned_lamports: u64,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct TicketReservation {
    pub buyer: Pubkey,
    pub promoter: Pubkey,
    pub email_hash: [u8; 32],
    pub paid_lamports: u64,
    pub timestamp: i64,
    pub bump: u8,
}

// -----------------------------------------------------------------------------
// EVENTS
// -----------------------------------------------------------------------------

#[event]
pub struct PromotionalConversionEvent {
    pub buyer: Pubkey,
    pub promoter_authority: Pubkey,
    pub promo_tag: String,
    pub email_hash: [u8; 32],
    pub ticket_price: u64,
    pub creator_reward: u64,
    pub treasury_amount: u64,
    pub timestamp: i64,
}

// -----------------------------------------------------------------------------
// ERROR CODES
// -----------------------------------------------------------------------------

#[error_code]
pub mod EncuentroError {
    #[msg("Referral reward fee basis points cannot exceed 3000 (30%).")]
    ExcessiveReferralBps,
    #[msg("Ticket price must be greater than zero.")]
    InvalidTicketPrice,
    #[msg("Promotion tag must be between 1 and 32 ASCII characters.")]
    InvalidTagLength,
    #[msg("Display name must be between 1 and 64 characters.")]
    InvalidNameLength,
    #[msg("Ticket purchases are currently paused.")]
    EventSalesPaused,
    #[msg("Calculation overflow occurred during fee settlement.")]
    MathOverflow,
    #[msg("Treasury account does not match event configuration.")]
    TreasuryMismatch,
    #[msg("Promoter payout destination does not match registered profile.")]
    InvalidPayoutDestination,
    #[msg("Unauthorized operation.")]
    Unauthorized,
}