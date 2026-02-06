//! Adaptive cache implementation
//!
//! This cache is the key to the antifragile behavior of the service.
//! As load increases, more queries are repeated, leading to higher cache hit rates
//! and better overall performance.

use dashmap::DashMap;
use std::time::{Duration, Instant};

use crate::pricing::{PriceQuery, PriceResult};

/// A cached price entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    result: PriceResult,
    created_at: Instant,
    hit_count: u64,
}

/// Adaptive cache for pricing results
///
/// This cache demonstrates antifragile behavior:
/// - Under low load: Few cache hits, most requests require computation
/// - Under high load: Many repeated queries, high cache hit rate, faster responses
///
/// The system literally gets BETTER under stress because popular price queries
/// are served from cache.
pub struct AdaptiveCache {
    entries: DashMap<PriceQuery, CacheEntry>,
    ttl: Duration,
    max_capacity: usize,
}

impl AdaptiveCache {
    const DEFAULT_MAX_CAPACITY: usize = 10_000;

    /// Create a new cache with default TTL of 5 minutes and 10k entry cap
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            ttl: Duration::from_secs(300),
            max_capacity: Self::DEFAULT_MAX_CAPACITY,
        }
    }

    /// Create a new cache with custom TTL and capacity
    #[allow(dead_code)]
    pub fn with_ttl_and_capacity(ttl: Duration, max_capacity: usize) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
            max_capacity,
        }
    }

    /// Get a cached result if it exists and hasn't expired
    pub fn get(&self, query: &PriceQuery) -> Option<PriceResult> {
        if let Some(mut entry) = self.entries.get_mut(query) {
            if entry.created_at.elapsed() < self.ttl {
                entry.hit_count += 1;
                return Some(entry.result.clone());
            } else {
                // Entry expired, will be replaced
                drop(entry);
                self.entries.remove(query);
            }
        }
        None
    }

    /// Insert a new entry into the cache, evicting stale or oldest entries if at capacity
    pub fn insert(&self, query: PriceQuery, result: PriceResult) {
        if self.entries.len() >= self.max_capacity {
            self.cleanup();
        }

        // If still at capacity after cleanup, evict the oldest entry
        if self.entries.len() >= self.max_capacity {
            self.evict_oldest();
        }

        self.entries.insert(
            query,
            CacheEntry {
                result,
                created_at: Instant::now(),
                hit_count: 0,
            },
        );
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.len();
        let total_hits: u64 = self.entries.iter().map(|e| e.hit_count).sum();

        CacheStats {
            entries,
            total_hits,
        }
    }

    /// Clear expired entries (called on insert when at capacity, and by the background task)
    pub fn cleanup(&self) {
        self.entries
            .retain(|_, entry| entry.created_at.elapsed() < self.ttl);
    }

    /// Evict the oldest entry by creation time
    fn evict_oldest(&self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|entry| entry.created_at)
            .map(|entry| entry.key().clone());

        if let Some(key) = oldest {
            self.entries.remove(&key);
        }
    }

    /// Clear all entries
    #[allow(dead_code)]
    pub fn clear(&self) {
        self.entries.clear();
    }
}

impl Default for AdaptiveCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub total_hits: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_hit() {
        let cache = AdaptiveCache::new();

        let query = PriceQuery {
            product_id: "test".to_string(),
            quantity: 1,
            options: vec![],
        };

        let result = PriceResult {
            base_price: 10.0,
            quantity_discount: 0.0,
            options_cost: 0.0,
            total_price: 10.0,
        };

        cache.insert(query.clone(), result.clone());
        let cached = cache.get(&query);

        assert!(cached.is_some());
        assert!((cached.unwrap().total_price - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_cache_miss() {
        let cache = AdaptiveCache::new();

        let query = PriceQuery {
            product_id: "nonexistent".to_string(),
            quantity: 1,
            options: vec![],
        };

        assert!(cache.get(&query).is_none());
    }

    #[test]
    fn test_cache_expiry() {
        let cache = AdaptiveCache::with_ttl_and_capacity(Duration::from_millis(10), 100);

        let query = PriceQuery {
            product_id: "test".to_string(),
            quantity: 1,
            options: vec![],
        };

        let result = PriceResult {
            base_price: 10.0,
            quantity_discount: 0.0,
            options_cost: 0.0,
            total_price: 10.0,
        };

        cache.insert(query.clone(), result);

        // Should exist immediately
        assert!(cache.get(&query).is_some());

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(20));

        // Should be expired now
        assert!(cache.get(&query).is_none());
    }

    #[test]
    fn test_cache_capacity_limit() {
        let cache = AdaptiveCache::with_ttl_and_capacity(Duration::from_secs(300), 3);

        let result = PriceResult {
            base_price: 10.0,
            quantity_discount: 0.0,
            options_cost: 0.0,
            total_price: 10.0,
        };

        for i in 0..5 {
            let query = PriceQuery {
                product_id: format!("product-{i}"),
                quantity: 1,
                options: vec![],
            };
            cache.insert(query, result.clone());
        }

        // Cache should never exceed max_capacity + 1 (insert happens after eviction)
        assert!(cache.stats().entries <= 4);
    }
}
