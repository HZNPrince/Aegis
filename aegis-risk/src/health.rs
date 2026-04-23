use std::collections::HashMap;

use aegis_core::{
    state::AppState,
    types::{AlertSeverity, PositionUpdate},
};
use serde::{Deserialize, Serialize};

const DEFAULT_LIQUIDATION_THRESHOLD: f64 = 0.85;

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

pub fn wallet_risk(state: &AppState, wallet: &str) -> WalletRisk {
    let positions: Vec<PositionUpdate> = state
        .positions
        .iter()
        .filter_map(|entry| {
            let pos = entry.value();
            (pos.owner == wallet).then(|| pos.clone())
        })
        .collect();

    risk_from_positions(wallet.to_string(), positions)
}

pub fn risk_from_positions(wallet: String, positions: Vec<PositionUpdate>) -> WalletRisk {
    let mut grouped: HashMap<String, (f64, f64)> = HashMap::new();

    for position in &positions {
        let protocol = grouped
            .entry(position.protocol.clone())
            .or_insert((0.0, 0.0));
        protocol.0 += position.collateral_usd.max(0.0);
        protocol.1 += position.debt_usd.max(0.0);
    }

    let mut protocols = Vec::with_capacity(grouped.len());
    let mut total_collateral_usd = 0.0;
    let mut total_debt_usd = 0.0;

    for (protocol, (collateral_usd, debt_usd)) in grouped {
        total_collateral_usd += collateral_usd;
        total_debt_usd += debt_usd;

        protocols.push(ProtocolHealth {
            protocol,
            collateral_usd,
            debt_usd,
            ltv: safe_div(debt_usd, collateral_usd),
            liquidation_threshold: DEFAULT_LIQUIDATION_THRESHOLD,
        });
    }

    protocols.sort_by(|a, b| {
        b.ltv
            .partial_cmp(&a.ltv)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let ltv = safe_div(total_debt_usd, total_collateral_usd);
    // Safe room (Limit - borrowed)
    let liquidation_buffer_usd =
        (total_collateral_usd * DEFAULT_LIQUIDATION_THRESHOLD - total_debt_usd).max(0.0);
    let health_score = ((1.0 - ltv / DEFAULT_LIQUIDATION_THRESHOLD) * 100.0).clamp(0.0, 100.0);

    WalletRisk {
        wallet,
        health_score,
        severity: classify_severity(health_score, liquidation_buffer_usd),
        total_collateral_usd,
        total_debt_usd,
        ltv,
        liquidation_threshold: DEFAULT_LIQUIDATION_THRESHOLD,
        liquidation_buffer_usd,
        protocols,
        positions,
    }
}

pub fn classify_severity(health_score: f64, liquidation_buffer_usd: f64) -> AlertSeverity {
    if health_score <= 15.0 || liquidation_buffer_usd <= 100.0 {
        AlertSeverity::Critical
    } else if health_score <= 40.0 || liquidation_buffer_usd <= 500.0 {
        AlertSeverity::Warning
    } else {
        AlertSeverity::Info
    }
}

fn safe_div(a: f64, b: f64) -> f64 {
    if b <= f64::EPSILON { 0.0 } else { a / b }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(protocol: &str, collateral_usd: f64, debt_usd: f64) -> PositionUpdate {
        PositionUpdate {
            pubkey: format!("{protocol}-obligation"),
            owner: "wallet".to_string(),
            protocol: protocol.to_string(),
            collateral_usd,
            debt_usd,
            slot: 1,
            legs: Vec::new(),
        }
    }

    #[test]
    fn computes_expected_wallet_risk() {
        let risk = risk_from_positions(
            "wallet".to_string(),
            vec![pos("Kamino", 1_000.0, 400.0), pos("Marginfi", 500.0, 200.0)],
        );

        assert_eq!(risk.total_collateral_usd, 1_500.0);
        assert_eq!(risk.total_debt_usd, 600.0);
        assert!((risk.ltv - 0.4).abs() < 1e-9);
        assert!(risk.health_score > 50.0);
    }

    #[test]
    fn classifies_critical_for_tight_buffer() {
        let severity = classify_severity(82.0, 50.0);
        assert!(matches!(severity, AlertSeverity::Critical));
    }
}
