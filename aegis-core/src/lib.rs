//! aegis-core — shared foundation for all Aegis crates.
//! Provides types (Position, WalletRisk, PositionUpdate), configuration, error handling,
//! concurrent application state, and symbol lookup utilities. No business logic — only definitions.

pub mod config;
pub mod error;
pub mod state;
pub mod symbols;
pub mod types;
