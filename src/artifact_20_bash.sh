# 1. Build & Test Anchor Program
cd contracts/encuentro_bounty
anchor build
anchor test

# 2. Deploy to Solana Devnet / Mainnet-Beta
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy --provider.cluster devnet

# 3. Spin up Orchestration Engine
cd ../../
npm install
npm run build
docker-compose up -d --build

# 4. Generate Promotional Badge
npm run cli -- generate-badge -h "@SolanaEspanol" -w "7abc...walletAddress" -c "Technical"