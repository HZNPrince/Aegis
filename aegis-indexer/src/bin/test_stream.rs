//! Quick test binary to verify the gRPC stream is working.
//! Run with: cargo run --bin test-stream
//!
//! Expected output: you should see account updates flowing in
//! from Kamino, Save, and Marginfi accounts every few seconds.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging — RUST_LOG=info shows our messages
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let endpoint = std::env::var("GRPC_ENDPOINT")
        .unwrap_or_else(|_| "https://solana-rpc.parafi.tech:10443".to_string());

    tracing::info!("Starting Aegis test stream...");

    aegis_indexer::grpc::start_account_stream(&endpoint).await?;

    Ok(())
}
