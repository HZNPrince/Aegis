//! Shared application state accessible across all async tasks.
//!
//! Uses DashMap (lock-free concurrent hashmap) for hot-path data that the
//! gRPC stream writes and the API reads simultaneously.

use std::sync::atomic::AtomicU64;

use dashmap::DashMap;
use sqlx::PgPool;

use crate::parsers::PositionUpdate;

/// Cached data from a Marginfi Bank needed to convert shares → USD.
#[derive(Debug, Clone)]
pub struct BankData {
    /// Token mint address (e.g., USDC, SOL)
    pub mint: String,
    /// Number of decimals for this token (e.g., 6 for USDC, 9 for SOL)
    pub mint_decimals: u8,
    /// Exchange rate: deposit_shares × asset_share_value = token_amount.
    /// Stored as I80F48 fixed-point (i128 raw, divide by 2^48 to get f64).
    pub asset_share_value: f64,
    /// Exchange rate: borrow_shares × liability_share_value = token_amount.
    pub liability_share_value: f64,
}

pub struct AppState {
    /// Live cache of all parsed positions, keyed by obligation pubkey.
    /// Written by the gRPC stream, read by the API layer.
    pub positions: DashMap<String, PositionUpdate>,

    /// Wallets the user has opted to monitor. Only positions from these
    /// wallets get cached and persisted to Postgres.
    pub monitored_wallets: DashMap<String, bool>,

    /// Global update counter for logging/metrics.
    pub update_count: AtomicU64,

    /// Real-time token prices: mint address → USD price.
    /// Updated every 10s by the Jupiter poller.
    pub token_prices: DashMap<String, f64>,

    /// Maps bank/reserve pubkey → underlying token mint address.
    /// Populated at startup by oracle discovery.
    pub token_mints: DashMap<String, String>,

    /// Marginfi Bank cache: bank pubkey → exchange rates + decimals.
    /// Needed to convert user shares into token amounts for USD calculation.
    pub bank_cache: DashMap<String, BankData>,

    /// Postgres connection pool for persistent storage.
    pub db_pool: PgPool,
}

impl AppState {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            positions: DashMap::new(),
            monitored_wallets: DashMap::new(),
            update_count: AtomicU64::new(0),
            token_prices: DashMap::new(),
            token_mints: DashMap::new(),
            bank_cache: DashMap::new(),
            db_pool,
        }
    }
}
