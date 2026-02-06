# Adaptive Pricing API

Demonstrates the Fragile → Robust → Antifragile transition as the cache warms up.

## Quick Start

```bash
# Start services and run load test
docker compose --profile loadtest up

# Or start services only, then run load test separately
docker compose up -d
docker compose --profile loadtest up loadtest
```

Open Grafana at http://localhost:3001 (admin / antifragile) to watch metrics.

## The Transition

| Phase | Requests | Hit Rate | Exponent | Classification |
| ----- | -------- | -------- | -------- | -------------- |
| 1     | Unique   | ~0%      | 0.70     | Fragile        |
| 2     | Mixed    | ~50%     | 1.00     | Robust         |
| 3     | Repeated | ~80%     | 1.18     | Antifragile    |

The exponent determines curve shape: `exponent = 0.7 + hit_rate × 0.6`

## API

```bash
curl -X POST http://localhost:3000/price \
  -H "Content-Type: application/json" \
  -d '{"product_id": "widget", "quantity": 10}'

curl http://localhost:3000/antifragile/status
curl http://localhost:3000/antifragile/curve
```

## Cleanup

```bash
docker compose down -v
```
