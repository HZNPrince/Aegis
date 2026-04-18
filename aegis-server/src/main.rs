//! Aegis Server — runs the full pipeline: oracle → price poller → gRPC indexer → API server
//!
//! Run with: cargo run -p aegis-server

use std::sync::Arc;

use aegis_core::config::AegisConfig;
use aegis_core::state::AppState;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config = AegisConfig::from_env();
    let pool = PgPool::connect(&config.database_url).await?;

    // DB writer channel: bounded at 1000 — the gRPC stream's try_send drops
    // updates when the writer falls behind rather than stalling the hot path.
    let (db_tx, db_rx) = mpsc::channel(1_000);
    let state = Arc::new(AppState::new(pool, db_tx));

    tokio::spawn(aegis_indexer::writer::run_db_writer(db_rx, state.clone()));

    let token_mints =
        aegis_indexer::oracle::discover_mints(&config.rpc_endpoint, &state).await?;

    tokio::spawn(aegis_indexer::oracle::start_jupiter_poller(
        state.clone(),
        token_mints,
    ));

    tokio::spawn(aegis_api::start_server(state.clone()));

    tokio::spawn(aegis_alerts::engine::start_alert_engine(
        state.clone(),
        config.poll_interval_secs,
        config.alert_threshold,
    ));

    // Supervisor loop: reconnects forever on drop.
    aegis_indexer::grpc::start_account_stream(&config.grpc_endpoint, state).await?;

    Ok(())
}
