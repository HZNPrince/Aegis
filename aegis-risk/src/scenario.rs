use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::health::{WalletRisk, risk_from_positions};
use aegis_core::types::PositionUpdate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRequest {
    pub wallet: String,
    pub collateral_shock_pct: Option<f64>,
    pub debt_shock_pct: Option<f64>,
    pub protocol_overrides: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResponse {
    pub base: WalletRisk,
    pub shocked: WalletRisk,
    pub collateral_change_usd: f64,
    pub debt_change_usd: f64,
    pub breached: bool,
}

pub fn simulate(base: WalletRisk, request: &ScenarioRequest) -> ScenarioResponse {
    let shocked_positions = apply_shocks(&base.positions, request);
    let shocked = risk_from_positions(base.wallet.clone(), shocked_positions);

    ScenarioResponse {
        collateral_change_usd: shocked.total_collateral_usd - base.total_collateral_usd,
        debt_change_usd: shocked.total_debt_usd - base.total_debt_usd,
        breached: shocked.ltv >= shocked.liquidation_threshold,
        base,
        shocked,
    }
}

fn apply_shocks(
    positions: &[PositionUpdate],
    request: &ScenarioRequest,
) -> Vec<PositionUpdate> {
    positions
        .iter()
        .map(|position| {
            let mut next = position.clone();
            let protocol_multiplier = request
                .protocol_overrides
                .get(&position.protocol)
                .copied()
                .unwrap_or(1.0);

            let collateral_multiplier = (1.0 + request.collateral_shock_pct.unwrap_or(0.0)).max(0.0);
            let debt_multiplier = (1.0 + request.debt_shock_pct.unwrap_or(0.0)).max(0.0);

            next.collateral_usd *= protocol_multiplier * collateral_multiplier;
            next.debt_usd *= protocol_multiplier * debt_multiplier;
            next
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::risk_from_positions;

    #[test]
    fn scenario_detects_breach_after_shock() {
        let base = risk_from_positions(
            "wallet".to_string(),
            vec![PositionUpdate {
                pubkey: "1".to_string(),
                owner: "wallet".to_string(),
                protocol: "Kamino".to_string(),
                collateral_usd: 1_000.0,
                debt_usd: 700.0,
                slot: 1,
                legs: Vec::new(),
            }],
        );

        let request = ScenarioRequest {
            wallet: "wallet".to_string(),
            collateral_shock_pct: Some(-0.2),
            debt_shock_pct: None,
            protocol_overrides: HashMap::new(),
        };

        let response = simulate(base, &request);
        assert!(response.breached);
    }
}
