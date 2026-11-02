# Start Docker cluster
docker compose up -d --build

# Verify container liveness
curl -I http://localhost:8080/health
# HTTP/1.1 200 OK

# Dispatch integration test pass
curl -X POST http://localhost:8080/api/test-ticket \
  -H "Content-Type: application/json" \
  -d '{"attendee":"HispanoDev Madrid","promoter":"SolanaEsp","ticketId":"2026"}'