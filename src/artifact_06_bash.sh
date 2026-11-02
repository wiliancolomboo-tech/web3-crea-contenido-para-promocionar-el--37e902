# 1. Build optimized BPF bytecode
anchor build

# 2. Synchronize Program ID
solana address -k target/deploy/el_encuentro_engine-keypair.json
anchor keys sync

# 3. Deploy to Solana Devnet
solana program deploy \
  --program-id target/deploy/el_encuentro_engine-keypair.json \
  target/deploy/el_encuentro_engine.so \
  --url https://api.devnet.solana.com

# 4. Initialize Campaign parameters
npx ts-node cli/admin-init.ts \
  --bronze 100 \
  --silver 500 \
  --gold 1500 \
  --end-date "2026-10-15T00:00:00Z"