use anchor_lang::prelude::*;

pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Encu26MadridPromo111111111111111111111111111");

#[program]
pub mod encuentro_promo {
    use super::*;

    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        initial_deposit: u64,
    ) -> Result<()> {
        instructions::initialize_campaign::handler(ctx, initial_deposit)
    }

    pub fn submit_content(
        ctx: Context<SubmitContent>,
        content_hash: [u8; 32],
        platform_id: u8,
    ) -> Result<()> {
        instructions::submit_content::handler(ctx, content_hash, platform_id)
    }

    pub fn verify_and_reward(
        ctx: Context<VerifyAndReward>,
        tier: u8,
        reward_amount: u64,
    ) -> Result<()> {
        instructions::verify_and_reward::handler(ctx, tier, reward_amount)
    }
}