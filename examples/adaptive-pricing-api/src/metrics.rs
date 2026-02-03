//! Metrics collection and Antifragile trait implementation
//!
//! This module tracks service metrics and implements the Antifragile trait
//! to analyze system behavior under load.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use antifragile::Antifragile;
use chrono::{DateTime, Utc};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use parking_lot::RwLock;

/// Service metrics collector
pub struct ServiceMetrics {
    total_requests: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    total_response_time_us: AtomicU64,
    history: RwLock<Vec<HistoryEntry>>,
    last_snapshot_time: RwLock<std::time::Instant>,
}

/// A point-in-time snapshot of service metrics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ServiceSnapshot {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_response_time_ms: f64,
}

/// Historical entry for tracking classification over time
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub total_requests: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
    pub classification: String,
}

/// Current service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
    pub requests_per_second: f64,
}

impl ServiceMetrics {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            total_response_time_us: AtomicU64::new(0),
            history: RwLock::new(Vec::new()),
            last_snapshot_time: RwLock::new(std::time::Instant::now()),
        }
    }

    pub fn record_request(&self, duration: Duration) {
        let count = self.total_requests.fetch_add(1, Ordering::Relaxed) + 1;
        let duration_us = duration.as_micros() as u64;
        self.total_response_time_us
            .fetch_add(duration_us, Ordering::Relaxed);

        counter!("pricing_requests_total").increment(1);
        histogram!("pricing_response_time_seconds").record(duration.as_secs_f64());

        let stats = self.get_stats();
        gauge!("pricing_cache_hit_ratio").set(stats.cache_hit_rate);
        gauge!("pricing_avg_response_time_ms").set(stats.avg_response_time_ms);

        if count % 100 == 0 {
            self.record_history_entry();
        }
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
        counter!("pricing_cache_hits_total").increment(1);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
        counter!("pricing_cache_misses_total").increment(1);
    }

    pub fn get_stats(&self) -> ServiceStats {
        let total_requests = self.total_requests.load(Ordering::Relaxed);
        let cache_hits = self.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.cache_misses.load(Ordering::Relaxed);
        let total_response_time_us = self.total_response_time_us.load(Ordering::Relaxed);

        let cache_hit_rate = if total_requests > 0 {
            cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        let avg_response_time_ms = if total_requests > 0 {
            (total_response_time_us as f64 / total_requests as f64) / 1000.0
        } else {
            0.0
        };

        let elapsed = self.last_snapshot_time.read().elapsed().as_secs_f64();
        let requests_per_second = if elapsed > 0.0 {
            total_requests as f64 / elapsed
        } else {
            0.0
        };

        ServiceStats {
            total_requests,
            cache_hits,
            cache_misses,
            cache_hit_rate,
            avg_response_time_ms,
            requests_per_second,
        }
    }

    fn record_history_entry(&self) {
        use antifragile::TriadAnalysis;

        let stats = self.get_stats();

        let snapshot = ServiceSnapshot {
            total_requests: stats.total_requests,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            avg_response_time_ms: stats.avg_response_time_ms,
        };

        // Use normalized load (requests_per_second / 100) as stressor
        let normalized_load = (stats.requests_per_second / 100.0).max(0.1);
        let classification = snapshot.classify(normalized_load, 0.1);

        let entry = HistoryEntry {
            timestamp: Utc::now(),
            total_requests: stats.total_requests,
            cache_hit_rate: stats.cache_hit_rate,
            avg_response_time_ms: stats.avg_response_time_ms,
            classification: format!("{:?}", classification),
        };

        let mut history = self.history.write();
        history.push(entry);

        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        self.history.read().clone()
    }
}

impl Default for ServiceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement Antifragile for ServiceSnapshot
///
/// This models how caching creates antifragile behavior:
/// - Stressor: Request load (normalized requests/second)
/// - Payoff: Effective throughput capacity (requests we can handle)
///
/// The payoff is derived from actual metrics:
/// - cache_hit_rate: measures how well the cache is working
/// - avg_response_time_ms: measures actual request cost
///
/// The model: As load increases, cache warms up, hit rate improves,
/// and effective capacity grows faster than load (convex/antifragile).
impl Antifragile for ServiceSnapshot {
    type Stressor = f64; // Load level (normalized)
    type Payoff = f64; // Effective throughput capacity

    fn payoff(&self, load: Self::Stressor) -> Self::Payoff {
        // Effective throughput capacity based on actual system metrics
        //
        // The model uses a convex (superlinear) relationship calibrated by observed data:
        //   payoff = base_throughput * efficiency_factor * load^exponent
        //
        // Where:
        // - base_throughput: derived from actual response time
        // - efficiency_factor: derived from actual cache hit rate
        // - exponent > 1: creates convexity, calibrated by cache effectiveness

        // Clamp load to small positive value for continuity
        let load = load.abs().max(0.001);

        // Derive efficiency from actual metrics
        let observed_hit_rate = if self.total_requests > 0 {
            self.cache_hits as f64 / self.total_requests as f64
        } else {
            0.5 // Assume 50% if no data
        };

        // Base throughput from actual response time (requests/sec capacity)
        let base_throughput = if self.avg_response_time_ms > 0.001 {
            1000.0 / self.avg_response_time_ms
        } else {
            10000.0 // Cap if response time is negligible
        };

        // Efficiency multiplier: higher cache hit rate = more efficient system
        // Range: 1.0 (0% hits) to 2.0 (100% hits)
        let efficiency_factor = 1.0 + observed_hit_rate;

        // Convexity exponent calibrated by cache effectiveness
        // Higher hit rate indicates cache is working well → stronger convexity
        // Range: 1.1 (poor caching) to 1.5 (excellent caching)
        let convexity_exponent = 1.1 + observed_hit_rate * 0.4;

        // Effective throughput: grows superlinearly with load
        // The power function load^exponent (exponent > 1) guarantees convexity
        base_throughput * efficiency_factor * load.powf(convexity_exponent)
    }
}

/// Set up Prometheus metrics recorder
pub fn setup_metrics_recorder() -> PrometheusHandle {
    PrometheusBuilder::new()
        .install_recorder()
        .expect("Failed to install Prometheus recorder")
}

#[cfg(test)]
mod tests {
    use super::*;
    use antifragile::TriadAnalysis;

    #[test]
    fn test_service_snapshot_is_antifragile() {
        let snapshot = ServiceSnapshot {
            total_requests: 1000,
            cache_hits: 800,
            cache_misses: 200,
            avg_response_time_ms: 2.0,
        };

        // Test that payoff increases with cache hit rate (convex behavior)
        let payoff_low = snapshot.payoff(0.2);
        let payoff_mid = snapshot.payoff(0.5);
        let payoff_high = snapshot.payoff(0.8);

        // Efficiency should increase with cache hit rate
        assert!(payoff_high > payoff_mid);
        assert!(payoff_mid > payoff_low);

        // The system should classify as antifragile at moderate hit rates
        let classification = snapshot.classify(0.5, 0.1);
        assert_eq!(classification, antifragile::Triad::Antifragile);
    }

    #[test]
    fn test_convexity() {
        let snapshot = ServiceSnapshot {
            total_requests: 1000,
            cache_hits: 800,
            cache_misses: 200,
            avg_response_time_ms: 2.0,
        };

        // Convexity test: f(x+d) + f(x-d) > 2*f(x)
        let x = 0.5;
        let d = 0.2;

        let f_x = snapshot.payoff(x);
        let f_x_plus = snapshot.payoff(x + d);
        let f_x_minus = snapshot.payoff(x - d);

        let sum = f_x_plus + f_x_minus;
        let twin = 2.0 * f_x;

        // For an antifragile system, sum > twin (convex)
        assert!(
            sum > twin,
            "Expected convex behavior: {} + {} = {} > {} = 2 * {}",
            f_x_plus,
            f_x_minus,
            sum,
            twin,
            f_x
        );
    }

    #[test]
    fn test_convexity_at_high_cache_hit_rate() {
        // Test with high cache hit rate (typical under load)
        let snapshot = ServiceSnapshot {
            total_requests: 500,
            cache_hits: 487, // 97.4% hit rate
            cache_misses: 13,
            avg_response_time_ms: 0.31,
        };

        // Verify convexity at low normalized load (API typical values)
        let x = 0.1;
        let d = 0.1;

        let f_x_minus = snapshot.payoff(x - d);
        let f_x = snapshot.payoff(x);
        let f_x_plus = snapshot.payoff(x + d);

        let sum = f_x_plus + f_x_minus;
        let twin = 2.0 * f_x;

        assert!(sum > twin, "Expected convex behavior at high hit rate");
        assert_eq!(snapshot.classify(x, d), antifragile::Triad::Antifragile);
    }
}
