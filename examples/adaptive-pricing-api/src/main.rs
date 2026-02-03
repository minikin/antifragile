//! Adaptive Pricing API - A real-world demonstration of antifragile systems
//!
//! This service demonstrates antifragile behavior: it becomes MORE efficient
//! under load due to adaptive caching, exhibiting convex payoff characteristics.

mod cache;
mod metrics;
mod pricing;

use std::sync::Arc;
use std::time::Instant;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::cache::AdaptiveCache;
use crate::metrics::ServiceMetrics;
use crate::pricing::{PriceQuery, calculate_price};

/// Application state shared across handlers
pub struct AppState {
    pub cache: AdaptiveCache,
    pub metrics: ServiceMetrics,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            cache: AdaptiveCache::new(),
            metrics: ServiceMetrics::new(),
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "adaptive_pricing_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let metrics_handle = metrics::setup_metrics_recorder();

    let state = Arc::new(AppState::new());

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/price", post(calculate_price_handler))
        .route(
            "/metrics",
            get(move || std::future::ready(metrics_handle.render())),
        )
        .route("/antifragile/status", get(antifragile_status))
        .route("/antifragile/history", get(antifragile_history))
        .route("/cache/stats", get(cache_stats))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Adaptive Pricing API listening on http://0.0.0.0:3000");
    tracing::info!("Metrics available at http://0.0.0.0:3000/metrics");
    tracing::info!("Antifragile status at http://0.0.0.0:3000/antifragile/status");

    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}

/// Request body for price calculation
#[derive(Debug, Deserialize)]
pub struct PriceRequest {
    pub product_id: String,
    pub quantity: u32,
    #[serde(default)]
    pub options: Vec<String>,
}

/// Response body for price calculation
#[derive(Debug, Serialize)]
pub struct PriceResponse {
    pub price: f64,
    pub currency: &'static str,
    pub cache_hit: bool,
    pub computation_time_ms: f64,
}

/// Calculate price for a product configuration
async fn calculate_price_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PriceRequest>,
) -> Result<Json<PriceResponse>, StatusCode> {
    let start = Instant::now();

    let query = PriceQuery {
        product_id: request.product_id,
        quantity: request.quantity,
        options: request.options,
    };

    let (result, cache_hit) = if let Some(cached) = state.cache.get(&query) {
        state.metrics.record_cache_hit();
        (cached, true)
    } else {
        state.metrics.record_cache_miss();
        let result = calculate_price(&query);
        state.cache.insert(query, result.clone());
        (result, false)
    };

    let elapsed = start.elapsed();
    state.metrics.record_request(elapsed);

    Ok(Json(PriceResponse {
        price: result.total_price,
        currency: "USD",
        cache_hit,
        computation_time_ms: elapsed.as_secs_f64() * 1000.0,
    }))
}

/// Antifragile status response
#[derive(Debug, Serialize)]
pub struct AntifragileStatusResponse {
    pub classification: String,
    pub rank: u8,
    pub description: String,
    pub metrics: CurrentMetrics,
    pub analysis: ConvexityAnalysis,
}

#[derive(Debug, Serialize)]
pub struct CurrentMetrics {
    pub total_requests: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
    pub requests_per_second: f64,
}

#[derive(Debug, Serialize)]
pub struct ConvexityAnalysis {
    pub is_convex: bool,
    pub explanation: String,
}

/// Get current antifragile classification
async fn antifragile_status(State(state): State<Arc<AppState>>) -> Json<AntifragileStatusResponse> {
    use antifragile::{Triad, TriadAnalysis};

    let stats = state.metrics.get_stats();

    // Create a snapshot for analysis
    let snapshot = metrics::ServiceSnapshot {
        total_requests: stats.total_requests,
        cache_hits: stats.cache_hits,
        cache_misses: stats.cache_misses,
        avg_response_time_ms: stats.avg_response_time_ms,
    };

    // Classify the system
    // We use normalized load as the stressor (requests_per_second / 100)
    // The payoff model is convex: capacity grows superlinearly with load due to cache warming
    let normalized_load = (stats.requests_per_second / 100.0).max(0.1);
    let classification = snapshot.classify(normalized_load, 0.1);

    let is_convex = classification == Triad::Antifragile;
    let explanation = match classification {
        Triad::Antifragile => {
            "System exhibits convex behavior: throughput efficiency improves with load \
             as cache warms up. The system benefits from stress."
        }
        Triad::Robust => {
            "System exhibits linear behavior: throughput scales proportionally with load. \
             The system is resilient but doesn't gain from stress."
        }
        Triad::Fragile => {
            "System exhibits concave behavior: throughput degrades under load. \
             The system is harmed by stress."
        }
    };

    Json(AntifragileStatusResponse {
        classification: format!("{:?}", classification),
        rank: classification.rank(),
        description: format!("{}", classification),
        metrics: CurrentMetrics {
            total_requests: stats.total_requests,
            cache_hit_rate: stats.cache_hit_rate,
            avg_response_time_ms: stats.avg_response_time_ms,
            requests_per_second: stats.requests_per_second,
        },
        analysis: ConvexityAnalysis {
            is_convex,
            explanation: explanation.to_string(),
        },
    })
}

/// Historical data point
#[derive(Debug, Serialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub total_requests: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
    pub classification: String,
}

/// Get historical classification data
async fn antifragile_history(State(state): State<Arc<AppState>>) -> Json<Vec<HistoryEntry>> {
    let history = state.metrics.get_history();
    Json(
        history
            .into_iter()
            .map(|h| HistoryEntry {
                timestamp: h.timestamp.to_rfc3339(),
                total_requests: h.total_requests,
                cache_hit_rate: h.cache_hit_rate,
                avg_response_time_ms: h.avg_response_time_ms,
                classification: h.classification,
            })
            .collect(),
    )
}

/// Cache statistics response
#[derive(Debug, Serialize)]
pub struct CacheStatsResponse {
    pub entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

/// Get cache statistics
async fn cache_stats(State(state): State<Arc<AppState>>) -> Json<CacheStatsResponse> {
    let stats = state.cache.stats();
    let metrics = state.metrics.get_stats();

    Json(CacheStatsResponse {
        entries: stats.entries,
        hits: metrics.cache_hits,
        misses: metrics.cache_misses,
        hit_rate: metrics.cache_hit_rate,
    })
}
