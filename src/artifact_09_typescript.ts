import Fastify from "fastify";
import { Connection } from "@solana/web3.js";

const fastify = Fastify({ logger: true });
const RPC_URL = process.env.SOLANA_RPC_URL || "https://api.devnet.solana.com";
const connection = new Connection(RPC_URL, "confirmed");

// Healthcheck endpoint for AWS ALB / GCP Cloud Run
fastify.get("/health", async (_request, reply) => {
  try {
    const slot = await connection.getSlot();
    const version = await connection.getVersion();
    return reply.status(200).send({
      status: "HEALTHY",
      service: "El Encuentro 2026 Madrid Promotion Engine",
      solanaSlot: slot,
      solanaVersion: version["solana-core"],
      timestamp: new Date().toISOString(),
    });
  } catch (error: any) {
    return reply.status(503).send({
      status: "DEGRADED",
      error: error.message,
    });
  }
});

const start = async () => {
  try {
    const port = Number(process.env.PORT) || 8080;
    await fastify.listen({ port, host: "0.0.0.0" });
    console.log(`Server listening on port ${port}`);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
};

start();