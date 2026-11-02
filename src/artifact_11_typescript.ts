import * as http from "http";

const PORT = process.env.PORT || 8080;

export function startHealthServer() {
  const server = http.createServer((req, res) => {
    if (req.url === "/health" && req.method === "GET") {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({
        status: "HEALTHY",
        protocol: "El Encuentro Madrid 2026",
        timestamp: new Date().toISOString(),
      }));
      return;
    }
    res.writeHead(404);
    res.end();
  });

  server.listen(PORT, () => {
    console.log(`[HEALTH] Telemetry server responding on port ${PORT}`);
  });
}