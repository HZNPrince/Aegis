//! Database writer — the only task that writes position updates to Postgres.
//!
//! Owning all writes in one task gives us: (a) natural backpressure via the
//! mpsc channel, (b) a single place to batch if we want to later, (c) no
//! write-write races across the gRPC stream and backfill paths.

use std::sync::Arc;

use aegis_core::{state::AppState, types::PositionUpdate};
use tokio::sync::mpsc;
use tracing::info;

pub async fn run_db_writer(mut rx: mpsc::Receiver<PositionUpdate>, state: Arc<AppState>) {
    info!("Database writer spawned");

    while let Some(pos) = rx.recv().await {
        let _ = sqlx::query(
            "INSERT INTO wallets (pubkey) VALUES ($1) ON CONFLICT (pubkey) DO NOTHING",
        )
        .bind(&pos.owner)
        .execute(&state.db_pool)
        .await;

        let _ = sqlx::query(
            "INSERT INTO positions (wallet_pubkey, obligation_pubkey, protocol, collateral_usd, debt_usd, last_slot)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (obligation_pubkey)
             DO UPDATE SET collateral_usd = $4, debt_usd = $5, last_slot = $6, updated_at = NOW()
             WHERE positions.last_slot < $6",
        )
        .bind(&pos.owner)
        .bind(&pos.pubkey)
        .bind(&pos.protocol)
        .bind(pos.collateral_usd)
        .bind(pos.debt_usd)
        .bind(pos.slot as i64)
        .execute(&state.db_pool)
        .await;
    }
}
