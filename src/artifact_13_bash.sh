# 1. Clone repository & install dependencies
git clone https://github.com/solana-esp/el-encuentro-madrid-2026.git
cd el-encuentro-madrid-2026

# 2. Build Anchor Smart Program
anchor build

# 3. Deploy to Solana Devnet (or Mainnet-Beta)
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy

# 4. Initialize Node.js Client Runtime
cd client
npm install
cp .env.example .env
npm run build
npm start