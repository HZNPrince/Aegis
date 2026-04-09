//! Quick test binary to verify the gRPC stream is working.
//! Run with: cargo run --bin test-stream
//!
//! Expected output: you should see account updates flowing in
//! from Kamino, Save, and Marginfi accounts every few seconds.

use std::sync::Arc;

use aegis_indexer::state::AppState;
use dotenv::var;
use sqlx::PgPool;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Initialize logging — RUST_LOG=info shows our messages
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Connect to postgres
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let rpc_url = std::env::var("RPC_ENDPOINT").expect("RPC_ENDPOINT must be set in .env");

    tracing::info!("Connecting to Postgres...");
    let pool = PgPool::connect(&db_url).await?;

    // Initialize the AppState
    let state = Arc::new(AppState::new(pool));

    // Boot the Oracle Engine
    let token_mints = aegis_indexer::oracle::discover_mints(&rpc_url, &state).await?;
    tokio::spawn(aegis_indexer::oracle::start_jupiter_poller(
        state.clone(),
        token_mints,
    ));

    let dummy_user = "YubozzSnKomEnH3pkmYsdatUUwUTcm7s4mHJVmefEWj";
    state.monitored_wallets.insert(dummy_user.to_string(), true);
    tracing::info!("Mocking User Login: Now monitoring wallet {}", dummy_user);

    let endpoint = std::env::var("GRPC_ENDPOINT")
        .unwrap_or_else(|_| "https://solana-rpc.parafi.tech:10443".to_string());

    tracing::info!("Starting Aegis test stream...");

    aegis_indexer::grpc::start_account_stream(&endpoint, state).await?;

    Ok(())
}
