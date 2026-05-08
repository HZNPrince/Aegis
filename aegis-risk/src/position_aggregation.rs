// Position aggregation helpers — centralized logic for calculating collateral, debt, and native amounts.
// Used by health.rs and engine.rs to avoid duplicated position iteration patterns.

use aegis_core::types::{PositionSide, PositionUpdate, WalletRisk};

/// Sum all collateral amounts in USD for a wallet's positions.
pub fn total_collateral_usd(positions: &[PositionUpdate]) -> f64 {
    positions.iter().map(|p| p.collateral_usd).sum()
}

/// Sum all debt amounts in USD for a wallet's positions.
pub fn total_debt_usd(positions: &[PositionUpdate]) -> f64 {
    positions.iter().map(|p| p.debt_usd).sum()
}

/// Sum all collateral in native units (pre-decimals) across all positions and legs.
pub fn total_collateral_native(positions: &[PositionUpdate]) -> u64 {
    positions
        .iter()
        .flat_map(|pos| &pos.legs)
        .filter(|leg| leg.side == PositionSide::Collateral)
        .map(|leg| leg.amount_native)
        .sum()
}

/// Sum all debt in native units (pre-decimals) across all positions and legs.
pub fn total_debt_native(positions: &[PositionUpdate]) -> u64 {
    positions
        .iter()
        .flat_map(|pos| &pos.legs)
        .filter(|leg| leg.side == PositionSide::Borrow)
        .map(|leg| leg.amount_native)
        .sum()
}

/// Check if wallet has any active positions (collateral or debt > 0).
pub fn has_active_positions(positions: &[PositionUpdate]) -> bool {
    let col = total_collateral_usd(positions);
    let debt = total_debt_usd(positions);
    col > 0.0 || debt > 0.0
}

/// Get a copy of previous leg amounts indexed by (protocol, reserve_or_bank, side).
pub fn previous_leg_snapshot(
    positions: &[PositionUpdate],
) -> std::collections::HashMap<(String, String, PositionSide), (f64, String)> {
    let mut map = std::collections::HashMap::new();
    for pos in positions {
        for leg in &pos.legs {
            let key = (pos.protocol.clone(), leg.reserve_or_bank.clone(), leg.side);
            map.insert(key, (leg.amount_ui, leg.asset_symbol.clone()));
        }
    }
    map
}
