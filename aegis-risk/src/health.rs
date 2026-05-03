use std::collections::HashMap;

use aegis_core::{
    state::AppState,
    types::{AlertSeverity, PositionUpdate, ProtocolHealth, WalletRisk, default_liquidation_threshold},
};

pub fn wallet_risk(state: &AppState, wallet: &str) -> WalletRisk {
    let mut positions: Vec<PositionUpdate> = state
        .positions
        .iter()
        .filter_map(|entry| {
            let pos = entry.value();
            (pos.owner == wallet).then(|| pos.clone())
        })
        .collect();

    // Reprice legs with live Jupiter prices — on-chain USD is stale until the next user interaction.
    for pos in &mut positions {
        for leg in &mut pos.legs {
            if !leg.asset_mint.is_empty() {
                if let Some(price) = state.token_prices.get(&leg.asset_mint) {
                    leg.value_usd = leg.amount_ui * *price;
                }
            }
        }
        // Legs are the source of truth: re-derive totals so a fully-repaid borrow shows 0 debt.
        if !pos.legs.is_empty() {
            pos.collateral_usd = pos.legs.iter()
                .filter(|l| matches!(l.side, aegis_core::types::PositionSide::Collateral))
                .map(|l| l.value_usd)
                .sum();
            pos.debt_usd = pos.legs.iter()
                .filter(|l| matches!(l.side, aegis_core::types::PositionSide::Borrow))
                .map(|l| l.value_usd)
                .sum();
        }
    }

    risk_from_positions(wallet.to_string(), positions)
}

pub fn risk_from_positions(wallet: String, positions: Vec<PositionUpdate>) -> WalletRisk {
    // Group by protocol, accumulating collateral/debt and a weighted threshold.
    struct ProtocolAccum {
        collateral_usd: f64,
        debt_usd: f64,
        weighted_threshold_sum: f64,
    }
    let mut grouped: HashMap<String, ProtocolAccum> = HashMap::new();

    for position in &positions {
        let entry = grouped
            .entry(position.protocol.clone())
            .or_insert(ProtocolAccum {
                collateral_usd: 0.0,
                debt_usd: 0.0,
                weighted_threshold_sum: 0.0,
            });
        let col = position.collateral_usd.max(0.0);
        let debt = position.debt_usd.max(0.0);
        entry.collateral_usd += col;
        entry.debt_usd += debt;
        entry.weighted_threshold_sum += col * position.liquidation_threshold;
    }

    let mut protocols = Vec::with_capacity(grouped.len());
    let mut total_collateral_usd = 0.0_f64;
    let mut total_debt_usd = 0.0_f64;
    let mut global_weighted_threshold_sum = 0.0_f64;

    for (protocol, accum) in grouped {
        total_collateral_usd += accum.collateral_usd;
        total_debt_usd += accum.debt_usd;

        let threshold = if accum.collateral_usd > 0.0 {
            accum.weighted_threshold_sum / accum.collateral_usd
        } else {
            default_liquidation_threshold()
        };
        global_weighted_threshold_sum += accum.collateral_usd * threshold;

        protocols.push(ProtocolHealth {
            protocol,
            collateral_usd: accum.collateral_usd,
            debt_usd: accum.debt_usd,
            ltv: safe_div(accum.debt_usd, accum.collateral_usd),
            liquidation_threshold: threshold,
        });
    }

    protocols.sort_by(|a, b| {
        b.ltv
            .partial_cmp(&a.ltv)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let global_threshold = if total_collateral_usd > 0.0 {
        global_weighted_threshold_sum / total_collateral_usd
    } else {
        default_liquidation_threshold()
    };

    let ltv = safe_div(total_debt_usd, total_collateral_usd);
    let liquidation_buffer_usd =
        (total_collateral_usd * global_threshold - total_debt_usd).max(0.0);
    // Degenerate case: zero collateral with active debt is fully undercollateralized.
    let health_score = if total_collateral_usd <= f64::EPSILON && total_debt_usd > f64::EPSILON {
        0.0
    } else {
        ((1.0 - ltv / global_threshold) * 100.0).clamp(0.0, 100.0)
    };

    WalletRisk {
        wallet,
        health_score,
        severity: classify_severity(health_score, liquidation_buffer_usd),
        total_collateral_usd,
        total_debt_usd,
        ltv,
        liquidation_threshold: global_threshold,
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
            liquidation_threshold: default_liquidation_threshold(),
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

    #[test]
    fn per_protocol_threshold_used_in_health() {
        // Position with 70% threshold should give a lower health score than one with 85%.
        let mut p = pos("Kamino", 1_000.0, 600.0);
        p.liquidation_threshold = 0.70;
        let risk = risk_from_positions("wallet".to_string(), vec![p]);
        // ltv = 0.6, threshold = 0.7 → health = (1 - 0.6/0.7) * 100 ≈ 14.3
        assert!(risk.health_score < 20.0, "health was {}", risk.health_score);
    }
}
