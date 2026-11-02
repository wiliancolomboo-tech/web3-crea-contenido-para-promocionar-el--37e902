# 1. Clone & Build Solana Program
git clone https://github.com/solana-madrid-2026/encuentro-amplified.git
cd encuentro-amplified
anchor build

# 2. Sync Program Keypair & Verify Address
anchor keys sync
PROGRAM_ID=$(solana address -k target/deploy/encuentro_amplified-keypair.json)
echo "Deploying to Solana Devnet/Mainnet with Program ID: $PROGRAM_ID"

# 3. Deploy to Solana Devnet / Mainnet-Beta
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy --program-name encuentro_amplified --provider.cluster devnet

# 4. Initialize Campaign via CLI
cd client
npm install
npm run build
node dist/index.js submit-content \
  --campaign "MADRID-2026" \
  --url "https://x.com/solana/status/123456789" \
  --advocate "8x2157A5WjGg21Fas..." \
  --email "builder@madrid2026.org"