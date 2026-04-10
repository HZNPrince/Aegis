//! Protocol parsers — each parser knows how to extract position data from
//! one lending protocol's on-chain account format.

pub mod kamino;
pub mod marginfi;
pub mod save;

/// Normalized position data extracted from any lending protocol account.
/// Currently stores aggregate USD values per obligation.
///
/// TODO: Add per-token breakdown (Vec<TokenPosition>) for dashboard display.
#[derive(Debug, Clone)]
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

/// Trait for protocol-specific account parsers.
/// Each implementation knows how to detect and decode one protocol's accounts.
pub trait ProtocolParser: Send + Sync {
    fn program_id(&self) -> &str;
    /// Attempt to parse raw account data into a PositionUpdate.
    /// Returns None if this account type isn't a user position (e.g., it's a Bank/Reserve).
    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate>;
}
