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
#[derive(Default)]
pub struct AppState {
    pub cache: AdaptiveCache,
    pub metrics: ServiceMetrics,
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

    let state = Arc::new(AppState::default());

    // Background task: evict expired cache entries every 60 seconds
    let cleanup_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup_state.cache.cleanup();
        }
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/price", post(calculate_price_handler))
        .route(
            "/metrics",
            get(move || std::future::ready(metrics_handle.render())),
        )
        .route("/antifragile/status", get(antifragile_status))
        .route("/antifragile/curve", get(antifragile_curve))
        .route("/antifragile/history", get(antifragile_history))
        .route("/cache/stats", get(cache_stats))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Adaptive Pricing API listening on http://0.0.0.0:3000");
    tracing::info!("Metrics available at http://0.0.0.0:3000/metrics");
    tracing::info!("Antifragile status at http://0.0.0.0:3000/antifragile/status");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("Shutting down gracefully...");
}

async fn health_check() -> &'static str {
    "OK"
}

/// Request body for price calculation
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    if request.quantity == 0 || request.quantity > 100_000 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.product_id.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if request.options.len() > 20 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let start = Instant::now();

    let query = PriceQuery {
        product_id: request.product_id,
        quantity: request.quantity,
        options: request.options,
    }
    .normalized();

    let (result, cache_hit) = if let Some(cached) = state.cache.get(&query) {
        state.metrics.record_cache_hit();
        (cached, true)
    } else {
        state.metrics.record_cache_miss();
        let result = calculate_price(&query).await;
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
    pub exponent: f64,
    pub curve_shape: String,
    pub explanation: String,
}

/// Get current antifragile classification
async fn antifragile_status(State(state): State<Arc<AppState>>) -> Json<AntifragileStatusResponse> {
    let stats = state.metrics.get_stats();

    let snapshot = metrics::ServiceSnapshot {
        total_requests: stats.total_requests,
        cache_hits: stats.cache_hits,
        cache_misses: stats.cache_misses,
        avg_response_time_ms: stats.avg_response_time_ms,
    };

    let classification = snapshot.classify();
    let exponent = snapshot.exponent();

    let curve_shape = match classification {
        antifragile::Triad::Fragile => "concave",
        antifragile::Triad::Robust => "linear",
        antifragile::Triad::Antifragile => "convex",
    };

    let explanation = match classification {
        antifragile::Triad::Antifragile => "Cache is hot. System benefits from stress.",
        antifragile::Triad::Robust => "Cache is warming. System scales proportionally.",
        antifragile::Triad::Fragile => "Cache is cold. System degrades under load.",
    };

    Json(AntifragileStatusResponse {
        classification: format!("{classification:?}"),
        rank: classification.rank(),
        description: format!("{classification}"),
        metrics: CurrentMetrics {
            total_requests: stats.total_requests,
            cache_hit_rate: stats.cache_hit_rate,
            avg_response_time_ms: stats.avg_response_time_ms,
            requests_per_second: stats.requests_per_second,
        },
        analysis: ConvexityAnalysis {
            exponent,
            curve_shape: curve_shape.to_string(),
            explanation: explanation.to_string(),
        },
    })
}

#[derive(Debug, Serialize)]
pub struct CurveResponse {
    pub exponent: f64,
    pub curve_shape: String,
    pub points: Vec<CurvePoint>,
}

#[derive(Debug, Serialize)]
pub struct CurvePoint {
    pub load: f64,
    pub payoff: f64,
}

async fn antifragile_curve(State(state): State<Arc<AppState>>) -> Json<CurveResponse> {
    let stats = state.metrics.get_stats();

    let snapshot = metrics::ServiceSnapshot {
        total_requests: stats.total_requests,
        cache_hits: stats.cache_hits,
        cache_misses: stats.cache_misses,
        avg_response_time_ms: stats.avg_response_time_ms,
    };

    let exponent = snapshot.exponent();
    let curve_shape = match snapshot.classify() {
        antifragile::Triad::Fragile => "concave (Fragile)",
        antifragile::Triad::Robust => "linear (Robust)",
        antifragile::Triad::Antifragile => "convex (Antifragile)",
    }
    .to_string();

    let points: Vec<CurvePoint> = snapshot
        .curve_data(20)
        .into_iter()
        .map(|(load, payoff)| CurvePoint { load, payoff })
        .collect();

    Json(CurveResponse {
        exponent,
        curve_shape,
        points,
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
            .iter()
            .map(|h| HistoryEntry {
                timestamp: h.timestamp.to_rfc3339(),
                total_requests: h.total_requests,
                cache_hit_rate: h.cache_hit_rate,
                avg_response_time_ms: h.avg_response_time_ms,
                classification: h.classification.clone(),
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
