//! Aegis API — Axum HTTP server exposing cached DeFi position data.
//!
//! Endpoints:
//!   GET /api/status            — system health (positions cached, prices loaded)
//!   GET /api/prices            — all cached token prices
//!   GET /api/health/:wallet    — wallet health score + positions
//!   POST /api/scenario         — shocked risk simulation
//!   GET /api/alerts/:wallet    — persisted alert history
//!   GET /api/guard-rules/:wallet — guard rules for a wallet
//!   POST /api/guard-rules      — create or update a guard rule

mod handlers;

use std::sync::Arc;

use aegis_core::state::AppState;
use axum::{Router, routing::{get, post}};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Starts the Axum server on the PORT_ADDRESS from .env (default 7878).
pub async fn start_server(state: Arc<AppState>) {
    let port = std::env::var("PORT_ADDRESS").unwrap_or_else(|_| "7878".to_string());
    let addr = format!("0.0.0.0:{}", port);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/status", get(handlers::status))
        .route("/api/prices", get(handlers::prices))
        .route("/api/wallets/{wallet}", post(handlers::link_wallet))
        .route("/api/health/{wallet}", get(handlers::wallet_health))
        .route("/api/scenario", post(handlers::scenario))
        .route("/api/alerts/{wallet}", get(handlers::list_alerts))
        .route("/api/guard-rules/{wallet}", get(handlers::list_guard_rules))
        .route("/api/guard-rules", post(handlers::upsert_guard_rule))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("API server listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
