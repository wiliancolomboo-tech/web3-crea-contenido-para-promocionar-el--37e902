use anchor_lang::prelude::*;

#[account]
pub struct CampaignConfig {
    pub authority: Pubkey,
    pub treasury: Pubkey,
    pub campaign_id: u64,
    pub base_ticket_price_lamports: u64,
    pub advocate_commission_bps: u16,   // e.g. 1000 = 10%
    pub buyer_discount_bps: u16,        // e.g. 500 = 5%
    pub total_tickets_sold: u32,
    pub max_capacity: u32,
    pub is_active: bool,
    pub bump: u8,
}

impl CampaignConfig {
    pub const LEN: usize = 8 + // discriminator
        32 + // authority
        32 + // treasury
        8  + // campaign_id
        8  + // base_ticket_price_lamports
        2  + // advocate_commission_bps
        2  + // buyer_discount_bps
        4  + // total_tickets_sold
        4  + // max_capacity
        1  + // is_active
        1;   // bump
}

#[account]
pub struct AdvocateProfile {
    pub campaign_id: u64,
    pub advocate_wallet: Pubkey,
    pub referral_code_hash: [u8; 32],
    pub referrals_count: u32,
    pub total_earnings_lamports: u64,
    pub tier: u8, // 0: Community, 1: Influencer, 2: Key Partner
    pub bump: u8,
}

impl AdvocateProfile {
    pub const LEN: usize = 8 + // discriminator
        8  + // campaign_id
        32 + // advocate_wallet
        32 + // referral_code_hash
        4  + // referrals_count
        8  + // total_earnings_lamports
        1  + // tier
        1;   // bump
}

#[account]
pub struct TicketPass {
    pub ticket_id: u64,
    pub campaign_id: u64,
    pub attendee_wallet: Pubkey,
    pub advocate_wallet: Pubkey,
    pub purchase_price_lamports: u64,
    pub is_checked_in: bool,
    pub check_in_timestamp: i64,
    pub bump: u8,
}

impl TicketPass {
    pub const LEN: usize = 8 + // discriminator
        8  + // ticket_id
        8  + // campaign_id
        32 + // attendee_wallet
        32 + // advocate_wallet
        8  + // purchase_price_lamports
        1  + // is_checked_in
        8  + // check_in_timestamp
        1;   // bump
}