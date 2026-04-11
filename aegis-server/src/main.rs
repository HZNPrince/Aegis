//! Aegis Server — runs the full pipeline: oracle → price poller → gRPC indexer → API server
//!
//! Run with: cargo run -p aegis-server

use std::sync::Arc;

use aegis_core::state::AppState;
use sqlx::PgPool;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let rpc_url = std::env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT must be set in .env");

    let pool = PgPool::connect(&db_url).await?;
    let state = Arc::new(AppState::new(pool));

    // Phase 1: Discover all token mints from Marginfi Banks + Kamino Reserves
    let token_mints = aegis_indexer::oracle::discover_mints(&rpc_url, &state).await?;

    // Phase 2: Start background price polling
    tokio::spawn(aegis_indexer::oracle::start_jupiter_poller(
        state.clone(),
        token_mints,
    ));

    // Phase 3: Start Axum API server in background
    tokio::spawn(aegis_api::start_server(state.clone()));

    // Phase 4: Mock a monitored wallet and start gRPC stream (foreground)
    let dummy_user = "YubozzSnKomEnH3pkmYsdatUUwUTcm7s4mHJVmefEWj";
    state.monitored_wallets.insert(dummy_user.to_string(), true);
    tracing::info!("Monitoring wallet: {}", dummy_user);

    let endpoint = std::env::var("GRPC_ENDPOINT").expect("GRPC_ENDPOINT not set up in .env");

    aegis_indexer::grpc::start_account_stream(&endpoint, state).await?;

    Ok(())
}
