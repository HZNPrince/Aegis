//! Shared application state accessible across all async tasks.
//!
//! Uses DashMap (lock-free concurrent hashmap) for hot-path data that the
//! gRPC stream writes and the API reads simultaneously.

use std::sync::atomic::AtomicU64;

use dashmap::DashMap;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::types::PositionUpdate;

/// Cached data from a Marginfi Bank needed to convert shares → USD.
#[derive(Debug, Clone)]
pub struct BankData {
    pub mint: String,
    pub mint_decimals: u8,
    pub asset_share_value: f64,
    pub liability_share_value: f64,
}

/// Cached data from a Kamino Reserve — just what the parser needs to turn
/// a per-obligation leg into mint + decimals + value. The Marginfi analog
/// is `BankData`; Kamino's share-exchange math happens at refresh-obligation
/// time on-chain, so we don't need to hold share_values here.
#[derive(Debug, Clone)]
pub struct ReserveData {
    pub mint: String,
    pub mint_decimals: u8,
}

pub struct AppState {
    pub positions: DashMap<String, PositionUpdate>,
    pub monitored_wallets: DashMap<String, bool>,
    pub update_count: AtomicU64,
    pub token_prices: DashMap<String, f64>,
    /// Jupiter-reported 24h price change in percent (e.g. 1.52 = +1.52%).
    /// Populated from the same poll that writes `token_prices`.
    pub token_price_changes: DashMap<String, f64>,
    pub token_mints: DashMap<String, String>,
    pub bank_cache: DashMap<String, BankData>,
    pub reserve_cache: DashMap<String, ReserveData>,
    pub db_pool: PgPool,
    /// Sender half of the DB writer channel. Both the gRPC stream and the
    /// backfill job push position updates through this single channel so the
    /// writer stays the only task touching `positions`/`wallets` tables.
    pub db_writer_tx: mpsc::Sender<PositionUpdate>,
}

impl AppState {
    pub fn new(db_pool: PgPool, db_writer_tx: mpsc::Sender<PositionUpdate>) -> Self {
        Self {
            positions: DashMap::new(),
            monitored_wallets: DashMap::new(),
            update_count: AtomicU64::new(0),
            token_prices: DashMap::new(),
            token_price_changes: DashMap::new(),
            token_mints: DashMap::new(),
            bank_cache: DashMap::new(),
            reserve_cache: DashMap::new(),
            db_pool,
            db_writer_tx,
        }
    }
}
