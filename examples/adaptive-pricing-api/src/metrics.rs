//! Metrics collection and Antifragile trait implementation
//!
//! This module tracks service metrics and implements the Antifragile trait
//! to analyze system behavior under load.

use std::time::Duration;

use antifragile::Antifragile;
use chrono::{DateTime, Utc};
use metrics::{counter, gauge, histogram};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use parking_lot::RwLock;

/// Raw counters protected by a single lock for snapshot-consistent reads
#[derive(Debug, Clone)]
struct Counters {
    total_requests: u64,
    cache_hits: u64,
    cache_misses: u64,
    total_response_time_us: u64,
}

/// Service metrics collector
pub struct ServiceMetrics {
    counters: RwLock<Counters>,
    history: RwLock<Vec<HistoryEntry>>,
    start_time: std::time::Instant,
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
            counters: RwLock::new(Counters {
                total_requests: 0,
                cache_hits: 0,
                cache_misses: 0,
                total_response_time_us: 0,
            }),
            history: RwLock::new(Vec::new()),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn record_request(&self, duration: Duration) {
        let count = {
            let mut c = self.counters.write();
            c.total_requests += 1;
            c.total_response_time_us += duration.as_micros() as u64;
            c.total_requests
        };

        counter!("pricing_requests_total").increment(1);
        histogram!("pricing_response_time_seconds").record(duration.as_secs_f64());

        let stats = self.get_stats();
        gauge!("pricing_cache_hit_ratio").set(stats.cache_hit_rate);
        gauge!("pricing_avg_response_time_ms").set(stats.avg_response_time_ms);

        // Export antifragile status metrics
        let snapshot = ServiceSnapshot {
            total_requests: stats.total_requests,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            avg_response_time_ms: stats.avg_response_time_ms,
        };
        let exponent = snapshot.exponent();
        gauge!("antifragile_exponent").set(exponent);
        // Classification rank: 0=Fragile, 1=Robust, 2=Antifragile
        let rank = if exponent < 0.95 {
            0.0
        } else if exponent > 1.05 {
            2.0
        } else {
            1.0
        };
        gauge!("antifragile_classification_rank").set(rank);

        if count % 100 == 0 {
            self.record_history_entry();
        }
    }

    pub fn record_cache_hit(&self) {
        self.counters.write().cache_hits += 1;
        counter!("pricing_cache_hits_total").increment(1);
    }

    pub fn record_cache_miss(&self) {
        self.counters.write().cache_misses += 1;
        counter!("pricing_cache_misses_total").increment(1);
    }

    pub fn get_stats(&self) -> ServiceStats {
        let c = self.counters.read();

        let cache_total = c.cache_hits + c.cache_misses;
        let cache_hit_rate = if cache_total > 0 {
            c.cache_hits as f64 / cache_total as f64
        } else {
            0.0
        };

        let avg_response_time_ms = if c.total_requests > 0 {
            (c.total_response_time_us as f64 / c.total_requests as f64) / 1000.0
        } else {
            0.0
        };

        let elapsed = self.start_time.elapsed().as_secs_f64();
        let requests_per_second = if elapsed > 0.0 {
            c.total_requests as f64 / elapsed
        } else {
            0.0
        };

        ServiceStats {
            total_requests: c.total_requests,
            cache_hits: c.cache_hits,
            cache_misses: c.cache_misses,
            cache_hit_rate,
            avg_response_time_ms,
            requests_per_second,
        }
    }

    fn record_history_entry(&self) {
        let stats = self.get_stats();

        let snapshot = ServiceSnapshot {
            total_requests: stats.total_requests,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            avg_response_time_ms: stats.avg_response_time_ms,
        };

        // Classify based on exponent
        let exponent = snapshot.exponent();
        let classification = if exponent < 0.95 {
            antifragile::Triad::Fragile
        } else if exponent > 1.05 {
            antifragile::Triad::Antifragile
        } else {
            antifragile::Triad::Robust
        };

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
/// The model reflects actual system behavior at different cache states:
/// - Cold cache (low hit rate): concave curve → Fragile
/// - Warming cache (medium hit rate): linear curve → Robust
/// - Hot cache (high hit rate): convex curve → Antifragile
impl Antifragile for ServiceSnapshot {
    type Stressor = f64; // Load level (normalized)
    type Payoff = f64; // Effective throughput capacity

    fn payoff(&self, load: Self::Stressor) -> Self::Payoff {
        // Effective throughput capacity based on actual system metrics
        //
        // payoff = base_throughput * efficiency_factor * load^exponent
        //
        // The exponent determines curve shape and classification:
        //   < 1.0: concave (Fragile) - system degrades under load
        //   = 1.0: linear (Robust) - system scales proportionally
        //   > 1.0: convex (Antifragile) - system improves under load

        let load = load.abs().max(0.001);

        let observed_hit_rate = if self.total_requests > 0 {
            self.cache_hits as f64 / self.total_requests as f64
        } else {
            0.0 // No data = cold cache
        };

        let base_throughput = if self.avg_response_time_ms > 0.001 {
            1000.0 / self.avg_response_time_ms
        } else {
            10000.0
        };

        let efficiency_factor = 1.0 + observed_hit_rate;

        // Exponent maps hit rate to curve shape:
        //   0% hit rate → 0.7 (concave/Fragile)
        //  50% hit rate → 1.0 (linear/Robust)
        // 100% hit rate → 1.3 (convex/Antifragile)
        let exponent = 0.7 + observed_hit_rate * 0.6;

        base_throughput * efficiency_factor * load.powf(exponent)
    }
}

impl ServiceSnapshot {
    /// Get the current exponent value (for diagnostics)
    pub fn exponent(&self) -> f64 {
        let hit_rate = if self.total_requests > 0 {
            self.cache_hits as f64 / self.total_requests as f64
        } else {
            0.0
        };
        0.7 + hit_rate * 0.6
    }

    /// Generate payoff curve data points for visualization
    pub fn curve_data(&self, points: usize) -> Vec<(f64, f64)> {
        (0..points)
            .map(|i| {
                let load = (i as f64 + 1.0) / points as f64;
                (load, <Self as Antifragile>::payoff(self, load))
            })
            .collect()
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
    use antifragile::Triad;

    fn make_snapshot(hit_rate: f64) -> ServiceSnapshot {
        let total = 1000;
        let hits = (total as f64 * hit_rate) as u64;
        ServiceSnapshot {
            total_requests: total,
            cache_hits: hits,
            cache_misses: total - hits,
            avg_response_time_ms: 1.0,
        }
    }

    fn classify_snapshot(snapshot: &ServiceSnapshot) -> Triad {
        let exponent = snapshot.exponent();
        if exponent < 0.95 {
            Triad::Fragile
        } else if exponent > 1.05 {
            Triad::Antifragile
        } else {
            Triad::Robust
        }
    }

    #[test]
    fn test_cold_cache_is_fragile() {
        // 10% hit rate → exponent = 0.76 (concave)
        let snapshot = make_snapshot(0.1);
        assert!(snapshot.exponent() < 1.0, "Low hit rate should have exponent < 1");
        assert_eq!(classify_snapshot(&snapshot), Triad::Fragile);
    }

    #[test]
    fn test_warming_cache_is_robust() {
        // 50% hit rate → exponent = 1.0 (linear)
        let snapshot = make_snapshot(0.5);
        let exp = snapshot.exponent();
        assert!((exp - 1.0).abs() < 0.05, "Medium hit rate should have exponent ≈ 1.0");
        assert_eq!(classify_snapshot(&snapshot), Triad::Robust);
    }

    #[test]
    fn test_hot_cache_is_antifragile() {
        // 90% hit rate → exponent = 1.24 (convex)
        let snapshot = make_snapshot(0.9);
        assert!(snapshot.exponent() > 1.0, "High hit rate should have exponent > 1");
        assert_eq!(classify_snapshot(&snapshot), Triad::Antifragile);
    }

    #[test]
    fn test_exponent_range() {
        // Verify exponent maps correctly across hit rate range
        assert!((make_snapshot(0.0).exponent() - 0.7).abs() < 0.01);
        assert!((make_snapshot(0.5).exponent() - 1.0).abs() < 0.01);
        assert!((make_snapshot(1.0).exponent() - 1.3).abs() < 0.01);
    }

    #[test]
    fn test_curve_data() {
        let snapshot = make_snapshot(0.8);
        let curve = snapshot.curve_data(10);

        assert_eq!(curve.len(), 10);
        // Payoff should increase with load
        for i in 1..curve.len() {
            assert!(curve[i].1 > curve[i - 1].1, "Payoff should increase with load");
        }
    }
}
