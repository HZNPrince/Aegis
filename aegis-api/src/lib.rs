//! Aegis API — Axum HTTP server exposing cached DeFi position data.
//!
//! Endpoints:
//!   GET  /api/status                       — system health (positions cached, prices loaded)
//!   GET  /api/prices                       — all cached token prices
//!   GET  /api/ticker                       — prices + 24h change for the ticker rail
//!   GET  /api/wallets/:wallet              — wallet settings (telegram_chat_id, etc.)
//!   POST /api/wallets/:wallet              — link wallet + backfill positions
//!   GET  /api/health/:wallet               — wallet risk score + positions
//!   POST /api/scenario                     — shocked risk simulation
//!   GET  /api/alerts/:wallet               — persisted alert history
//!   GET  /api/wallets/:wallet/guard-rules  — guard rules for a wallet
//!   POST /api/guard-rules                  — create or update a guard rule
//!   DELETE /api/guard-rules/:id            — delete a guard rule
//!   GET  /api/wallets/by-chat/:chat_id     — reverse-lookup wallet by Telegram chat ID
//!   PATCH /api/wallets/:wallet/telegram    — link Telegram chat ID
//!   PATCH /api/wallets/:wallet/email       — link email address

mod handlers;

use std::sync::Arc;

use aegis_core::state::AppState;
use axum::{Router, routing::{delete, get, patch, post}};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

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
        .route("/api/ticker", get(handlers::ticker))
        .route("/api/wallets/{wallet}", get(handlers::get_wallet_settings).post(handlers::link_wallet))
        .route("/api/health/{wallet}", get(handlers::wallet_health))
        .route("/api/scenario", post(handlers::scenario))
        .route("/api/alerts/{wallet}", get(handlers::list_alerts))
        .route("/api/wallets/{wallet}/guard-rules", get(handlers::list_guard_rules))
        .route("/api/guard-rules", post(handlers::upsert_guard_rule))
        .route("/api/guard-rules/{rule_id}", delete(handlers::delete_guard_rule))
        .route("/api/wallets/by-chat/{chat_id}", get(handlers::wallet_by_chat))
        .route("/api/wallets/{wallet}/telegram", patch(handlers::link_telegram))
        .route("/api/wallets/{wallet}/email", patch(handlers::link_email))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("API server listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}
