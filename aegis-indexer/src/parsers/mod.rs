//! Protocol parsers — each parser knows how to extract position data from
//! one lending protocol's on-chain account format.

pub mod kamino;
pub mod marginfi;
pub mod save;

// Re-export PositionUpdate from aegis-core so existing code doesn't break
pub use aegis_core::types::PositionUpdate;

/// Trait for protocol-specific account parsers.
/// Each implementation knows how to detect and decode one protocol's accounts.
pub trait ProtocolParser: Send + Sync {
    fn program_id(&self) -> &str;
    /// Attempt to parse raw account data into a PositionUpdate.
    /// Returns None if this account type isn't a user position (e.g., it's a Bank/Reserve).
    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate>;
}
