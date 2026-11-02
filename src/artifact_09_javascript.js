const express = require("express");
const app = express();
const port = process.env.PORT || 8080;

app.use(express.json());

app.get("/health", (req, res) => {
  res.status(200).json({
    status: "healthy",
    event: "El Encuentro 2026 Madrid",
    solana_connected: true,
    timestamp: new Date().toISOString(),
  });
});

app.listen(port, () => {
  console.log(`[Telemetry] Server listening at http://localhost:${port}`);
});