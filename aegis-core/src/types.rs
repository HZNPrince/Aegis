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

/// Normalized position data extracted from any lending protocol account.
/// This is what the gRPC parser produces and the DashMap stores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdate {
    /// On-chain address of the obligation/account
    pub pubkey: String,
    /// Wallet owner of this position
    pub owner: String,
    /// Protocol name: "Kamino", "Marginfi", or "SAVE"
    pub protocol: String,
    /// Total collateral value in USD (aggregate across all tokens)
    pub collateral_usd: f64,
    /// Total debt value in USD (aggregate across all tokens)
    pub debt_usd: f64,
    /// Solana slot this update was observed at
    pub slot: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerKind {
    HealthBelow,
    LtvAbove,
    DebtAboveUsd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    NotifyOnly,
    AddCollateral,
    RepayDebt,
    Deleverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardRule {
    pub id: Option<String>,
    pub wallet: String,
    pub protocol: Option<String>,
    pub trigger_kind: TriggerKind,
    pub trigger_value: f64,
    pub action_kind: ActionKind,
    pub action_token: Option<String>,
    pub action_amount_usd: Option<f64>,
    pub max_usd_per_action: f64,
    pub daily_limit_usd: f64,
    pub cooldown_seconds: i64,
    pub is_active: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRecord {
    pub id: Option<String>,
    pub wallet: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub health_score: f64,
    pub ltv: f64,
    pub suggested_actions: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: Option<DateTime<Utc>>,
}
