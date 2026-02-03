/**
 * k6 Load Test Script for Adaptive Pricing API
 *
 * This script demonstrates how the API exhibits antifragile behavior:
 * - Under low load: Cold cache, slow responses
 * - Under high load: Warm cache, fast responses
 *
 * Run with: k6 run script.js
 * Or via Docker Compose: docker compose --profile loadtest up loadtest
 */

import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate, Trend } from 'k6/metrics';

// Custom metrics
const cacheHitRate = new Rate('cache_hit_rate');
const responseTime = new Trend('response_time_ms');

// Test configuration
export const options = {
  scenarios: {
    // Gradually increase load to demonstrate antifragile behavior
    ramp_up: {
      executor: 'ramping-vus',
      startVUs: 1,
      stages: [
        { duration: '30s', target: 5 },    // Warm up - low load, cold cache
        { duration: '1m', target: 20 },    // Medium load - cache warming
        { duration: '1m', target: 50 },    // High load - cache hot
        { duration: '1m', target: 100 },   // Peak load - antifragile benefits visible
        { duration: '30s', target: 10 },   // Cool down
      ],
    },
  },
  thresholds: {
    http_req_duration: ['p(95)<500'],  // 95% of requests under 500ms
    http_req_failed: ['rate<0.01'],    // Less than 1% failure rate
  },
};

// Product catalog - limited set to encourage cache hits
const products = [
  'widget-001',
  'widget-002',
  'gadget-001',
  'gadget-002',
  'premium-001',
];

// Option combinations - limited set to encourage cache hits
const optionSets = [
  [],
  ['express-shipping'],
  ['gift-wrap'],
  ['express-shipping', 'gift-wrap'],
  ['insurance'],
  ['priority-support'],
];

// Quantity buckets - using common values to encourage cache hits
const quantities = [1, 5, 10, 25, 50, 100];

// API base URL (configurable via environment variable)
const BASE_URL = __ENV.API_URL || 'http://api:3000';

/**
 * Main test function - executed for each virtual user iteration
 */
export default function () {
  // Generate a request with values likely to repeat (encouraging cache hits)
  const payload = JSON.stringify({
    product_id: products[Math.floor(Math.random() * products.length)],
    quantity: quantities[Math.floor(Math.random() * quantities.length)],
    options: optionSets[Math.floor(Math.random() * optionSets.length)],
  });

  const params = {
    headers: {
      'Content-Type': 'application/json',
    },
  };

  // Make the pricing request
  const res = http.post(`${BASE_URL}/price`, payload, params);

  // Record custom metrics
  if (res.status === 200) {
    const body = JSON.parse(res.body);
    cacheHitRate.add(body.cache_hit ? 1 : 0);
    responseTime.add(body.computation_time_ms);
  }

  // Validate response
  check(res, {
    'status is 200': (r) => r.status === 200,
    'has price': (r) => {
      const body = JSON.parse(r.body);
      return body.price !== undefined && body.price > 0;
    },
    'has cache_hit field': (r) => {
      const body = JSON.parse(r.body);
      return body.cache_hit !== undefined;
    },
    'response time < 100ms': (r) => r.timings.duration < 100,
  });

  // Small delay between requests
  sleep(0.05 + Math.random() * 0.1);
}

/**
 * Setup function - runs once before the test
 */
export function setup() {
  // Verify the API is healthy
  const healthRes = http.get(`${BASE_URL}/health`);
  if (healthRes.status !== 200) {
    throw new Error(`API health check failed: ${healthRes.status}`);
  }

  console.log('API is healthy, starting load test...');
  console.log(`Target URL: ${BASE_URL}`);

  return {
    startTime: new Date().toISOString(),
  };
}

/**
 * Teardown function - runs once after the test
 */
export function teardown(data) {
  // Fetch final antifragile status
  const statusRes = http.get(`${BASE_URL}/antifragile/status`);

  if (statusRes.status === 200) {
    const status = JSON.parse(statusRes.body);
    console.log('\n=== Final Antifragile Status ===');
    console.log(`Classification: ${status.classification}`);
    console.log(`Description: ${status.description}`);
    console.log(`Cache Hit Rate: ${(status.metrics.cache_hit_rate * 100).toFixed(1)}%`);
    console.log(`Avg Response Time: ${status.metrics.avg_response_time_ms.toFixed(2)}ms`);
    console.log(`Total Requests: ${status.metrics.total_requests}`);
    console.log('\n=== Convexity Analysis ===');
    console.log(`Is Convex: ${status.analysis.is_convex}`);
    console.log(`Explanation: ${status.analysis.explanation}`);
  }

  console.log(`\nTest started at: ${data.startTime}`);
  console.log(`Test ended at: ${new Date().toISOString()}`);
}

/**
 * Handle test summary
 */
export function handleSummary(data) {
  return {
    'stdout': textSummary(data, { indent: '  ', enableColors: true }),
  };
}

/**
 * Generate a text summary of the test results
 */
function textSummary(data, options) {
  const { metrics, root_group } = data;

  let summary = '\n';
  summary += '='.repeat(60) + '\n';
  summary += '  ANTIFRAGILE LOAD TEST SUMMARY\n';
  summary += '='.repeat(60) + '\n\n';

  // Request metrics
  if (metrics.http_reqs) {
    summary += `  Total Requests:     ${metrics.http_reqs.values.count}\n`;
    summary += `  Request Rate:       ${metrics.http_reqs.values.rate.toFixed(1)}/s\n`;
  }

  if (metrics.http_req_duration) {
    summary += `  Avg Response Time:  ${metrics.http_req_duration.values.avg.toFixed(2)}ms\n`;
    summary += `  P95 Response Time:  ${metrics.http_req_duration.values['p(95)'].toFixed(2)}ms\n`;
  }

  // Custom metrics
  if (metrics.cache_hit_rate) {
    summary += `  Cache Hit Rate:     ${(metrics.cache_hit_rate.values.rate * 100).toFixed(1)}%\n`;
  }

  if (metrics.response_time_ms) {
    summary += `  Avg Compute Time:   ${metrics.response_time_ms.values.avg.toFixed(2)}ms\n`;
  }

  summary += '\n';
  summary += '='.repeat(60) + '\n';

  return summary;
}
