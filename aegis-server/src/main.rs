//! Aegis Server — runs the full pipeline: oracle → price poller → gRPC indexer → API server
//!
//! Run with: cargo run -p aegis-server

use std::sync::Arc;

use aegis_core::config::AegisConfig;
use aegis_core::state::AppState;
use sqlx::{PgPool, Row};
use tokio::sync::mpsc;
use tracing::info;
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
        .with_target(false)
        .init();

    info!("╔═══════════════════════════════════════════════╗");
    info!("║  AEGIS  —  Solana Lending Risk Engine  v0.1   ║");
    info!("╚═══════════════════════════════════════════════╝");

    let config = AegisConfig::from_env();
    info!("[boot] config loaded (poll={}s threshold={})", config.poll_interval_secs, config.alert_threshold);

    let pool = PgPool::connect(&config.database_url).await?;
    info!("[boot] postgres connected");

    sqlx::migrate!("../migrations").run(&pool).await?;
    info!("[boot] migrations applied");

    // DB writer channel: bounded at 1000 — the gRPC stream's try_send drops
    // updates when the writer falls behind rather than stalling the hot path.
    let (db_tx, db_rx) = mpsc::channel(1_000);
    let state = Arc::new(AppState::new(pool, db_tx));

    tokio::spawn(aegis_indexer::writer::run_db_writer(db_rx, state.clone()));
    info!("[boot] db writer online");

    // Rehydrate monitored wallets before gRPC starts — updates are dropped until wallets are registered.
    let rows = sqlx::query("SELECT pubkey FROM wallets WHERE is_monitored = true")
        .fetch_all(&state.db_pool)
        .await?;
    let rehydrated = rows.len();
    for row in rows {
        let pubkey: String = row.try_get("pubkey")?;
        state.monitored_wallets.insert(pubkey, true);
    }
    info!("[boot] rehydrated {} monitored wallets from DB", rehydrated);

    let token_mints =
        aegis_indexer::oracle::discover_mints(&config.rpc_endpoint, &state).await?;
    info!("[boot] oracle discovery complete: {} mints", token_mints.len());

    tokio::spawn(aegis_indexer::oracle::start_jupiter_poller(
        state.clone(),
        token_mints,
    ));
    info!("[boot] jupiter price poller online");

    tokio::spawn(aegis_indexer::oracle::start_bank_refresh_loop(
        config.rpc_endpoint.clone(),
        state.clone(),
    ));
    info!("[boot] marginfi bank refresh loop online");

    tokio::spawn(aegis_indexer::oracle::start_kamino_reserve_refresh_loop(
        config.rpc_endpoint.clone(),
        state.clone(),
    ));
    info!("[boot] kamino reserve refresh loop online");

    tokio::spawn(aegis_indexer::oracle::start_save_reserve_refresh_loop(
        config.rpc_endpoint.clone(),
        state.clone(),
    ));
    info!("[boot] save reserve refresh loop online");

    tokio::spawn(aegis_api::start_server(state.clone()));
    info!("[boot] api server online");

    let mut dispatchers: Vec<Arc<dyn aegis_alerts::dispatch::Dispatcher>> =
        vec![Arc::new(aegis_alerts::dispatch::LogDispatcher)];
    if let Some(tg) = aegis_alerts::dispatch::TelegramDispatcher::from_env() {
        dispatchers.push(Arc::new(tg));
        info!("[boot] telegram dispatcher registered");
    } else {
        info!("[boot] telegram dispatcher disabled (TELEGRAM_BOT_TOKEN not set)");
    }

    tokio::spawn(aegis_alerts::engine::start_alert_engine(
        state.clone(),
        dispatchers,
        config.poll_interval_secs,
        config.alert_threshold,
    ));
    info!("[boot] alert engine online");

    info!("[boot] all subsystems up — starting gRPC supervisor");

    aegis_indexer::grpc::start_account_stream(&config.grpc_endpoint, state).await?;

    Ok(())
}
