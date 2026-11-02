import * as http from "http";

export function startHealthCheckServer(port = 9090) {
  const server = http.createServer((req, res) => {
    if (req.url === "/healthz" || req.url === "/metrics") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          status: "healthy",
          campaign: "El Encuentro Madrid 2026",
          network: "Solana",
          uptime: process.uptime(),
          timestamp: Date.now(),
        })
      );
    } else {
      res.writeHead(404);
      res.end();
    }
  });

  server.listen(port, () => {
    console.log(`🩺 Healthcheck & telemetry endpoint listening at :${port}/healthz`);
  });
}