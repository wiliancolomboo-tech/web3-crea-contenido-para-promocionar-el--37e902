# 1. Clone & install dependencies
git clone https://github.com/solana-madrid-2026/emave-protocol.git
cd emave-protocol
npm install

# 2. Build Anchor Smart Contracts
anchor build

# 3. Synchronize program keys
solana address -k target/deploy/madrid26_core-keypair.json
# Ensure ID matches declare_id! in src/lib.rs

# 4. Deploy to Solana Devnet
solana config set --url https://api.devnet.solana.com
solana airdrop 2
anchor deploy --provider.cluster Devnet

# 5. Execute Complete Test Matrix
anchor test