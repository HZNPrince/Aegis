//! Shared application state — concurrent, lock-free data structures for real-time updates.
//! Provides hot-path access to positions, prices, wallet metadata, and cached bank/reserve data
//! without blocking between the gRPC stream (writer), parsers (readers), and API handlers.
//! Uses DashMap (concurrent hashmap) to enable simultaneous reads from multiple subsystems.

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
    /// Maintenance asset weight (0.0–1.0). Higher = more collateral credit.
    /// Used to compute per-bank effective liquidation threshold.
    pub asset_weight_maint: f64,
    /// Maintenance liability weight (≥ 1.0). Higher = harsher debt treatment.
    /// Effective LTV threshold ≈ asset_weight_maint / liability_weight_maint.
    pub liability_weight_maint: f64,
}

/// Cached data from a Kamino Reserve.
#[derive(Debug, Clone)]
pub struct ReserveData {
    pub mint: String,
    pub mint_decimals: u8,
    /// cToken → underlying exchange rate: total_liquidity / collateral_supply.
    /// Used to convert `deposited_amount` (cToken units) to true native token amount.
    /// 1.0 when the reserve is empty (no cTokens minted yet).
    pub exchange_rate: f64,
    /// Liquidation threshold for this reserve (0–100, e.g. 80 = 80%).
    pub liquidation_threshold_pct: u8,
}

/// Cached data from a Save (Solend-fork) Reserve.
/// Exchange rate fields are needed to convert cToken amounts → underlying native amounts.
#[derive(Debug, Clone)]
pub struct SaveReserveData {
    pub liquidity_mint: String,
    pub mint_decimals: u8,
    pub collateral_mint: String,
    /// Total cToken supply — denominator for exchange rate.
    pub collateral_mint_total_supply: u64,
    /// Tokens currently sitting in the vault (not borrowed).
    pub available_amount: u64,
    /// Total outstanding borrows, WAD-scaled (÷ 10^18 for native units).
    pub borrowed_amount_wads: u128,
    /// Accumulated protocol fees, WAD-scaled.
    pub accumulated_fees_wads: u128,
    /// Protocol-level liquidation threshold (0–100, e.g. 80 = 80 %).
    pub liquidation_threshold_pct: u8,
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
    /// Save (Solend-fork) reserve cache. Keyed by reserve account pubkey.
    pub save_reserve_cache: DashMap<String, SaveReserveData>,
    /// Last-observed risk profile per wallet. Used to detect rapid deterioration,
    /// LTV improvements, and price impacts.
    /// On restart it's empty — delta checks skip the first cycle (None = no baseline).
    pub last_risk: DashMap<String, crate::types::WalletRisk>,
    /// Wallet pubkey → linked Telegram chat_id. Populated at startup from
    /// `wallets.telegram_chat_id` and on every successful PATCH so the alert
    /// engine can route without hitting the DB on its hot path.
    pub telegram_chat_ids: DashMap<String, i64>,
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
            save_reserve_cache: DashMap::new(),
            last_risk: DashMap::new(),
            telegram_chat_ids: DashMap::new(),
            db_pool,
            db_writer_tx,
        }
    }
}
