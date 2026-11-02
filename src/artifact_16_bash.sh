# 1. Build Solana Program
anchor build

# 2. Deploy to Devnet / Mainnet-Beta
anchor deploy --provider.cluster https://api.devnet.solana.com

# 3. Setup Client Environment
cp .env.example .env
# Edit .env with your DISCORD_WEBHOOK_URL and RESEND_API_KEY

# 4. Run tests verifying 100% assertions
npm run test

# 5. Launch the 24/7 background event service
docker-compose up -d --build