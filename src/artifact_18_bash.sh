# Set Solana CLI target
solana config set --url https://api.devnet.solana.com

# Build Anchor project
anchor build

# Deploy program
solana program deploy target/deploy/encuentro_promo.so --program-id target/deploy/encuentro_promo-keypair.json

# Execute automated test suite
anchor test