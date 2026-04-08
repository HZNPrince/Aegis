use std::sync::atomic::AtomicU64;

use dashmap::DashMap;
use sqlx::PgPool;

use crate::parsers::PositionUpdate;

/// Global shared state spanning the whole indexer
pub struct AppState {
    /// Lock-free ultra-fast cache of every protocol position
    pub positions: DashMap<String, PositionUpdate>,
    /// Highly optimized cache of wallets we actually care about
    pub monitored_wallets: DashMap<String, bool>,
    /// Thread-safe counter
    pub update_count: AtomicU64,
    /// Postgres database connection pool
    pub db_pool: PgPool,
}

impl AppState {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            positions: DashMap::new(),
            monitored_wallets: DashMap::new(),
            update_count: AtomicU64::new(0),
            db_pool,
        }
    }
}
