use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Which lending protocol a postion belongs to.
///
/// Each has different account layouts, LTV calculations and liquidation mechanics.
/// This tag tells the risk engine which formula to apply
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Protocol {
    Kamino,
    Save, // formerly Solend
    Marginfi,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::Kamino => write!(f, "Kamino"),
            Protocol::Save => write!(f, "Save"),
            Protocol::Marginfi => write!(f, "Marginfi"),
        }
    }
}

/// A UNIFIED representation of a single collateral or borrow position within a lending protocol.
/// Since each protocol stores it differently we normalize it so the risk engine dosen't care which protocol the data came from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub protocol: Protocol,
    pub obligation_address: Pubkey,
    pub asset_mint: Pubkey,
    pub asset_symbol: String,
    pub side: PositionSide,
    /// Amount in native token units ( NOT lamports - already converted)
    pub amount: f64,
    /// Current USD value from oracle
    pub value_usd: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PositionSide {
    Collateral,
    Borrow,
}

/// Aggregated risk view of a wallet across all protocols
/// This is what api returns and the dashboard displays
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletHealth {
    pub wallet: Pubkey,
    /// 0 = liquidation imminent, 100 = perfectly safe
    pub health_score: f64,
    /// USD buffer before liquidation triggers
    pub liquidation_buffer_usd: f64,
    pub positions: Vec<Position>,
    pub protocol_ltvs: Vec<ProtocolLtv>,
    pub computed_at: DateTime<Utc>,
}

/// per-protocol LTV breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolLtv {
    pub protocol: Protocol,
    /// Current LTV (0.0 to 1.0+). Above liquidation_threshold = danger.
    pub ltv: f64,
    /// Protocol's liquidation threshold
    pub liquidation_threshold: f64,
    pub total_collateral_usd: f64,
    pub total_borrow_usd: f64,
}
