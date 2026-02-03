# Adaptive Pricing API

Demonstrates antifragile behavior: the system becomes MORE efficient under load due to adaptive caching.

## Quick Start

```bash
# Start services
docker compose up -d

# Open Grafana dashboard
open http://localhost:3001  # admin / antifragile

# Run load test
docker compose --profile loadtest up loadtest
```

## Services

| Service    | URL                   |
| ---------- | --------------------- |
| API        | http://localhost:3000 |
| Prometheus | http://localhost:9090 |
| Grafana    | http://localhost:3001 |

## API Endpoints

```bash
# Calculate price
curl -X POST http://localhost:3000/price \
  -H "Content-Type: application/json" \
  -d '{"product_id": "widget-001", "quantity": 100}'

# Check antifragile status
curl http://localhost:3000/antifragile/status

# Other endpoints
GET /health
GET /metrics
GET /cache/stats
GET /antifragile/history
```

## How It Works

The payoff function uses actual metrics to determine convexity:

```
payoff = base_throughput × efficiency_factor × load^exponent
```

| Parameter           | Source                        |
| ------------------- | ----------------------------- |
| `base_throughput`   | `1000 / avg_response_time_ms` |
| `efficiency_factor` | `1 + cache_hit_rate`          |
| `exponent`          | `1.1 + cache_hit_rate × 0.4`  |

The exponent > 1 guarantees convexity. Higher cache hit rates produce stronger convexity, reflecting that effective caching creates superlinear benefits under load.

## Cleanup

```bash
docker compose down -v
```
