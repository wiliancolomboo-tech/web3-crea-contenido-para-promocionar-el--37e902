# 1. Clone repository and install dependencies
git clone https://github.com/solana-espana/el-encuentro-2026-madridpulse.git
cd el-encuentro-2026-madridpulse
npm install

# 2. Build Anchor Smart Program
anchor build

# 3. Synchronize program address in Anchor.toml & lib.rs
solana address -k target/deploy/madrid_pulse-keypair.json
# (Replace declare_id with output)

# 4. Deploy to Solana Devnet / Mainnet
solana config set --url https://api.devnet.solana.com
anchor deploy --provider.cluster devnet

# 5. Run full test suite
anchor test