//! Database writer — the only task that writes position updates to Postgres.
//!
//! Owning all writes in one task gives us: (a) natural backpressure via the
//! mpsc channel, (b) a single place to batch if we want to later, (c) no
//! write-write races across the gRPC stream and backfill paths.

use std::sync::Arc;

use aegis_core::{
    state::AppState,
    types::{PositionSide, PositionUpdate},
};
use tokio::sync::mpsc;
use tracing::{info, warn};

pub async fn run_db_writer(mut rx: mpsc::Receiver<PositionUpdate>, state: Arc<AppState>) {
    info!("[writer] spawned — draining position updates into postgres");

    let mut written: u64 = 0;
    let mut failed: u64 = 0;

    while let Some(pos) = rx.recv().await {
        if let Err(e) = sqlx::query(
            "INSERT INTO wallets (pubkey) VALUES ($1) ON CONFLICT (pubkey) DO NOTHING",
        )
        .bind(&pos.owner)
        .execute(&state.db_pool)
        .await
        {
            failed += 1;
            warn!("[writer] wallet upsert failed for {}: {}", pos.owner, e);
            continue;
        }

        match persist_position(&state, &pos).await {
            Ok(_) => {
                written += 1;
                if written % 50 == 0 {
                    info!("[writer] persisted {} positions ({} failed)", written, failed);
                }
            }
            Err(e) => {
                failed += 1;
                warn!("[writer] position upsert failed for {}: {}", pos.pubkey, e);
            }
        }
    }
}

/// Persist one position + its legs atomically. The legs table is rewritten on
/// each update so it always reflects the latest on-chain state — no stale
/// rows after partial repays or token swaps on the obligation.
async fn persist_position(state: &AppState, pos: &PositionUpdate) -> sqlx::Result<()> {
    let mut tx = state.db_pool.begin().await?;

    sqlx::query(
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
    .execute(&mut *tx)
    .await?;

    if !pos.legs.is_empty() {
        sqlx::query("DELETE FROM position_legs WHERE obligation_pubkey = $1")
            .bind(&pos.pubkey)
            .execute(&mut *tx)
            .await?;

        for leg in &pos.legs {
            sqlx::query(
                "INSERT INTO position_legs
                 (obligation_pubkey, side, asset_mint, asset_symbol, amount_native, amount_ui, value_usd, reserve_or_bank)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&pos.pubkey)
            .bind(side_label(leg.side))
            .bind(&leg.asset_mint)
            .bind(&leg.asset_symbol)
            .bind(leg.amount_native.min(i64::MAX as u64) as i64)
            .bind(leg.amount_ui)
            .bind(leg.value_usd)
            .bind(&leg.reserve_or_bank)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await
}

fn side_label(side: PositionSide) -> &'static str {
    match side {
        PositionSide::Collateral => "Collateral",
        PositionSide::Borrow => "Borrow",
    }
}
