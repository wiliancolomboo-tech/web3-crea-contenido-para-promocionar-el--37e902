import * as http from "http";

const HEALTH_PORT = process.env.PORT || 8080;
http
  .createServer(async (req, res) => {
    if (req.url === "/health") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          status: "UP",
          network: "devnet",
          timestamp: new Date().toISOString(),
          conference: "El Encuentro 2026 Madrid",
        })
      );
    } else {
      res.writeHead(404);
      res.end();
    }
  })
  .listen(HEALTH_PORT, () => {
    console.log(`🩺 Healthcheck monitoring live on port :${HEALTH_PORT}`);
  });