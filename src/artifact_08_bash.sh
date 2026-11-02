# 1. Build the Anchor Solana Program
anchor build

# 2. Deploy to Devnet / Mainnet-Beta
solana program deploy \
  --program-id target/deploy/el_encuentro_promo-keypair.json \
  target/deploy/el_encuentro_promo.so \
  --url https://api.devnet.solana.com

# 3. Synchronize program keys
anchor keys sync