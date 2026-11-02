import dotenv from "dotenv";
dotenv.config();

export const CONFIG = {
  SOLANA_RPC_URL: process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com",
  PROGRAM_ID: process.env.PROGRAM_ID || "Encu26MadridPromo111111111111111111111111111",
  ORACLE_PRIVATE_KEY: process.env.ORACLE_PRIVATE_KEY || "",
  RESEND_API_KEY: process.env.RESEND_API_KEY || "",
  DISCORD_WEBHOOK_URL: process.env.DISCORD_WEBHOOK_URL || "",
  TIER_REWARDS: {
    BRONZE: 25_000_000,    // 25 USDC (6 decimals)
    SILVER: 100_000_000,   // 100 USDC
    GOLD: 250_000_000,     // 250 USDC
    PLATINUM_VIP: 500_000_000 // 500 USDC + VIP Keynote Access Madrid 2026
  }
};