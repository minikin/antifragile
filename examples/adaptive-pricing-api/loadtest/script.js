/**
 * k6 Load Test - Demonstrates Fragile → Robust → Antifragile transition
 *
 * Run: docker compose --profile loadtest up loadtest
 */

import http from 'k6/http';
import { sleep } from 'k6';
import exec from 'k6/execution';

const BASE_URL = __ENV.API_URL || 'http://api:3000';

export const options = {
  scenarios: {
    phase1_fragile: {
      executor: 'constant-vus',
      vus: 10,
      duration: '60s',
      startTime: '0s',
      env: { PHASE: '1' },
    },
    phase2_robust: {
      executor: 'constant-vus',
      vus: 10,
      duration: '60s',
      startTime: '65s',
      env: { PHASE: '2' },
    },
    phase3_antifragile: {
      executor: 'constant-vus',
      vus: 10,
      duration: '60s',
      startTime: '130s',
      env: { PHASE: '3' },
    },
  },
};

export function setup() {
  const res = http.get(`${BASE_URL}/health`);
  if (res.status !== 200) throw new Error('API not healthy');

  console.log('\n' + '='.repeat(60));
  console.log('  ANTIFRAGILE TRANSITION TEST (~3 minutes)');
  console.log('='.repeat(60));
  console.log('  Phase 1 (0:00-1:00):  Unique products   → Fragile');
  console.log('  Phase 2 (1:05-2:05):  Mixed products    → Robust');
  console.log('  Phase 3 (2:10-3:10):  Repeated products → Antifragile');
  console.log('='.repeat(60) + '\n');
}

export default function () {
  const phase = __ENV.PHASE;
  const iter = exec.scenario.iterationInTest;

  let productId;
  if (phase === '1') {
    // Phase 1: All unique products - 0% cache hits
    productId = `unique-${Date.now()}-${iter}-${Math.random()}`;
  } else if (phase === '2') {
    // Phase 2: Mix of products - ~50% cache hits
    productId = `product-${iter % 100}`;
  } else {
    // Phase 3: Few repeated products - high cache hits
    productId = `product-${iter % 5}`;
  }

  http.post(
    `${BASE_URL}/price`,
    JSON.stringify({ product_id: productId, quantity: 1 }),
    { headers: { 'Content-Type': 'application/json' } }
  );

  sleep(0.05);
}

export function handleSummary(data) {
  const res = http.get(`${BASE_URL}/antifragile/status`);
  let status = { classification: 'unknown', metrics: {}, analysis: {} };
  if (res.status === 200) status = JSON.parse(res.body);

  const output = `
${'='.repeat(60)}
  FINAL STATUS
${'='.repeat(60)}
  Classification: ${status.classification}
  Cache Hit Rate: ${((status.metrics.cache_hit_rate || 0) * 100).toFixed(0)}%
  Exponent:       ${(status.analysis.exponent || 0).toFixed(2)} (${status.analysis.curve_shape || 'unknown'})
  Total Requests: ${status.metrics.total_requests || 0}
${'='.repeat(60)}

  Total HTTP Requests: ${data.metrics.http_reqs?.values?.count || 0}
  Avg Response Time:   ${(data.metrics.http_req_duration?.values?.avg || 0).toFixed(2)}ms
`;

  return { stdout: output };
}
