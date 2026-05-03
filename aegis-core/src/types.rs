use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;

/// Which lending protocol a position belongs to.
/// Each has different account layouts, LTV calculations, and liquidation mechanics.
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

/// Normalized single-asset position from any lending protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub protocol: Protocol,
    pub obligation_address: Pubkey,
    pub asset_mint: Pubkey,
    pub asset_symbol: String,
    pub side: PositionSide,
    /// Native token units (decimals applied; not raw lamports)
    pub amount: f64,
    /// Current USD value from oracle
    pub value_usd: f64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PositionSide {
    Collateral,
    Borrow,
}

/// Aggregated risk view of a wallet across all protocols — returned by the API.
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

/// Per-protocol LTV breakdown.
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
/// Produced by the gRPC parser and stored in the DashMap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdate {
    /// On-chain address of the obligation/account.
    pub pubkey: String,
    /// Wallet owner of this position.
    pub owner: String,
    /// Protocol name: "Kamino", "Marginfi", or "SAVE".
    pub protocol: String,
    /// Aggregate collateral value in USD.
    pub collateral_usd: f64,
    /// Aggregate debt value in USD.
    pub debt_usd: f64,
    /// Solana slot at which this update was observed.
    pub slot: u64,
    /// Per-asset legs (one per deposit/borrow reserve) for execution.
    #[serde(default)]
    pub legs: Vec<PositionLeg>,
    /// USD-weighted average liquidation threshold across deposit reserves (0.0–1.0).
    /// Falls back to 0.85 when unset.
    #[serde(default = "default_liquidation_threshold")]
    pub liquidation_threshold: f64,
}

pub fn default_liquidation_threshold() -> f64 {
    0.85
}

/// One per-asset row inside a `PositionUpdate`.
/// `amount_native` is raw on-chain units fed directly into repay instructions.
/// `reserve_or_bank` is the protocol-side account passed to the instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionLeg {
    pub side: PositionSide,
    pub asset_mint: String,
    pub asset_symbol: String,
    pub amount_native: u64,
    pub amount_ui: f64,
    pub value_usd: f64,
    pub reserve_or_bank: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolHealth {
    pub protocol: String,
    pub collateral_usd: f64,
    pub debt_usd: f64,
    pub ltv: f64,
    pub liquidation_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletRisk {
    pub wallet: String,
    pub health_score: f64,
    pub severity: AlertSeverity,
    pub total_collateral_usd: f64,
    pub total_debt_usd: f64,
    pub ltv: f64,
    pub liquidation_threshold: f64,
    pub liquidation_buffer_usd: f64,
    pub protocols: Vec<ProtocolHealth>,
    pub positions: Vec<PositionUpdate>,
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
    /// Fires when health score drops by more than `trigger_value` percentage
    /// (0.0–1.0) in a single engine cycle relative to the last observation.
    /// E.g. trigger_value = 0.15 → fire when health drops 15%+ since last check.
    HealthDropped,
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
    #[serde(default)]
    pub last_fired_at: Option<DateTime<Utc>>,
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
    /// Per-wallet Telegram chat ID — populated from the wallets table by the alert engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_chat_id: Option<i64>,
}
