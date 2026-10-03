# 1. Clone repository and install dependencies
git clone https://github.com/solana-esp/encuentro-2026-madrid-boost.git
cd encuentro-2026-madrid-boost
npm install

# 2. Build Anchor smart program
anchor build

# 3. Synchronize program keys and deploy to Solana Devnet
solana config set --url https://api.devnet.solana.com
anchor deploy --provider.cluster devnet

# 4. Initialize Campaign & Escrow Vault via CLI
npm run cli -- init-campaign \
  --name "madrid_encuentro_2026" \
  --reward 0.5 \
  --max 100 \
  --deposit 5.0