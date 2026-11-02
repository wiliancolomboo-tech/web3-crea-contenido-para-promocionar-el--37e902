cd client
npm install
npm run build

# Initialize Campaign via script or CLI
node dist/index.js initialize \
  --campaign madrid2026 \
  --bounty-rate 500000000 \
  --max-creators 1000