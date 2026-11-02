# 1. Clone repository & configure environment
git clone https://github.com/solana-madrid-2026/promotional-protocol.git
cd promotional-protocol
cp .env.example .env

# 2. Build on-chain program
anchor build

# 3. Deploy to Solana Devnet (or Mainnet)
solana program deploy ./target/deploy/mad_encuentro.so \
  --program-id ./target/deploy/mad_encuentro-keypair.json \
  --url devnet

# 4. Run full test suite with 100% assertions passing
anchor test