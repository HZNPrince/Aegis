pub mod kamino;
pub mod marginfi;
pub mod save;

/// Data extracted from any lending protocol account update.
#[derive(Debug, Clone)]
pub struct PositionUpdate {
    pub pubkey: String,
    pub owner: String,
    pub protocol: String,
    pub collateral_usd: f64,
    pub debt_usd: f64,
    pub slot: u64,
}

pub trait ProtocolParser: Send + Sync {
    fn program_id(&self) -> &str;
    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate>;
}
