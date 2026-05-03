use std::{sync::Arc, time::Duration};

use aegis_core::{
    state::AppState,
    types::{AlertRecord, AlertSeverity, GuardRule, PositionSide, WalletRisk},
};
use aegis_risk::health::wallet_risk;
use sqlx::Row;
use tracing::{error, info};

use crate::{
    dispatch::{Dispatcher, broadcast},
    llm::{LlmClient, into_alert_record},
};

/// Orchestration entrypoint: one evaluation pass for one wallet.
///
/// Decision tree:
///   1. If the wallet has active guard rules: fire only when at least one rule
///      trips AND that rule's own cooldown has elapsed. Stamp `last_fired_at`
///      on every rule that contributed.
///   2. If the wallet has no rules: fall back to the global `alert_threshold`
///      as a default safety net, with a 60-minute per-wallet dedupe.
///
/// Either way, we only call the LLM once per evaluation and broadcast to every
/// configured dispatcher.
pub async fn evaluate_wallet(
    state: Arc<AppState>,
    llm: &LlmClient,
    dispatchers: &[Arc<dyn Dispatcher>],
    wallet: &str,
    alert_threshold: f64,
) -> anyhow::Result<Option<AlertRecord>> {
    let risk = wallet_risk(&state, wallet);
    if risk.total_collateral_usd <= 0.0 && risk.total_debt_usd <= 0.0 {
        return Ok(None);
    }

    let prev_health = state.last_risk.get(wallet).map(|r| r.health_score);

    if let Some(last_risk_ref) = state.last_risk.get(wallet) {
        let last = &*last_risk_ref;
        let chat_id = load_telegram_chat_id(&state, wallet).await.ok().flatten();
        let mut smart_alerts = Vec::new();

        // Rapid health drop
        let health_drop = last.health_score - risk.health_score;
        if health_drop >= 15.0 {
            smart_alerts.push(AlertRecord {
                id: None,
                wallet: wallet.to_string(),
                severity: aegis_core::types::AlertSeverity::Warning,
                title: format!("Health dropped {:.0} points rapidly", health_drop),
                message: format!(
                    "Health fell from {:.1} to {:.1} in a single cycle. Check your positions immediately.",
                    last.health_score, risk.health_score
                ),
                health_score: risk.health_score,
                ltv: risk.ltv,
                suggested_actions: vec![
                    "Review market prices for your collateral assets.".to_string(),
                    "Consider adding collateral or repaying debt.".to_string(),
                ],
                metadata: serde_json::json!({ "velocity_drop": health_drop }),
                created_at: None,
                telegram_chat_id: chat_id,
            });
        }

        // LTV improvement
        let ltv_improvement = last.ltv - risk.ltv;
        if ltv_improvement >= 0.05 {
            smart_alerts.push(AlertRecord {
                id: None,
                wallet: wallet.to_string(),
                severity: aegis_core::types::AlertSeverity::Info,
                title: format!("Position Healthier (LTV improved by {:.1}%)", ltv_improvement * 100.0),
                message: format!(
                    "Your LTV improved from {:.1}% to {:.1}%.",
                    last.ltv * 100.0, risk.ltv * 100.0
                ),
                health_score: risk.health_score,
                ltv: risk.ltv,
                suggested_actions: vec![],
                metadata: serde_json::json!({ "ltv_improvement": ltv_improvement }),
                created_at: None,
                telegram_chat_id: chat_id,
            });
        }

        // LTV crossing into liquidation danger zone (first time within 5% of threshold)
        let margin_now = risk.liquidation_threshold - risk.ltv;
        let margin_last = last.liquidation_threshold - last.ltv;
        if margin_now <= 0.05 && margin_now > 0.0 && margin_last > 0.05 {
            smart_alerts.push(AlertRecord {
                id: None,
                wallet: wallet.to_string(),
                severity: aegis_core::types::AlertSeverity::Critical,
                title: "Approaching Liquidation Threshold".to_string(),
                message: format!(
                    "Your LTV ({:.1}%) is dangerously close to your liquidation threshold ({:.1}%).",
                    risk.ltv * 100.0, risk.liquidation_threshold * 100.0
                ),
                health_score: risk.health_score,
                ltv: risk.ltv,
                suggested_actions: vec![
                    "Repay debt or add collateral immediately to avoid liquidation.".to_string()
                ],
                metadata: serde_json::json!({ "ltv": risk.ltv, "threshold": risk.liquidation_threshold }),
                created_at: None,
                telegram_chat_id: chat_id,
            });
        }

        // Price-driven collateral drop: native amount unchanged but USD value fell >5%
        let mut total_collateral_native_now = 0;
        for pos in &risk.positions {
            for leg in &pos.legs {
                if leg.side == aegis_core::types::PositionSide::Collateral {
                    total_collateral_native_now += leg.amount_native;
                }
            }
        }
        let mut total_collateral_native_last = 0;
        for pos in &last.positions {
            for leg in &pos.legs {
                if leg.side == aegis_core::types::PositionSide::Collateral {
                    total_collateral_native_last += leg.amount_native;
                }
            }
        }
        
        if total_collateral_native_now >= total_collateral_native_last && total_collateral_native_last > 0 {
            if last.total_collateral_usd > 0.0 {
                let usd_drop_pct = (last.total_collateral_usd - risk.total_collateral_usd) / last.total_collateral_usd;
                if usd_drop_pct > 0.05 {
                    smart_alerts.push(AlertRecord {
                        id: None,
                        wallet: wallet.to_string(),
                        severity: aegis_core::types::AlertSeverity::Warning,
                        title: format!("Market Movement (Collateral value dropped {:.1}%)", usd_drop_pct * 100.0),
                        message: format!(
                            "Your collateral value dropped from ${:.2} to ${:.2} due to token price changes.",
                            last.total_collateral_usd, risk.total_collateral_usd
                        ),
                        health_score: risk.health_score,
                        ltv: risk.ltv,
                        suggested_actions: vec!["Monitor market conditions closely.".to_string()],
                        metadata: serde_json::json!({ "usd_drop_pct": usd_drop_pct }),
                        created_at: None,
                        telegram_chat_id: chat_id,
                    });
                }
            }
        }

        // Position change detection: new, increased, decreased, closed
        let mut last_amounts: std::collections::HashMap<(String, String, PositionSide), (f64, String)> = std::collections::HashMap::new();
        for pos in &last.positions {
            for leg in &pos.legs {
                let key = (pos.protocol.clone(), leg.reserve_or_bank.clone(), leg.side.clone());
                last_amounts.insert(key, (leg.amount_ui, leg.asset_symbol.clone()));
            }
        }

        let mut current_keys = std::collections::HashSet::new();

        for pos in &risk.positions {
            for leg in &pos.legs {
                let key = (pos.protocol.clone(), leg.reserve_or_bank.clone(), leg.side.clone());
                current_keys.insert(key.clone());
                
                let prev_amount = last_amounts.get(&key).map(|(a, _)| *a).unwrap_or(0.0);
                
                // Ignore sub-0.1% changes (interest accrual); catch real deposits/withdrawals.
                let amount_diff = (leg.amount_ui - prev_amount).abs();
                let pct_change = if prev_amount > 0.0 { amount_diff / prev_amount } else { 1.0 };
                let is_significant = pct_change > 0.001 || (leg.value_usd > 0.0 && (amount_diff / leg.amount_ui * leg.value_usd) > 1.0);

                if is_significant {
                    match leg.side {
                        PositionSide::Borrow => {
                            if prev_amount == 0.0 {
                                smart_alerts.push(AlertRecord {
                                    id: None,
                                    wallet: wallet.to_string(),
                                    severity: AlertSeverity::Info,
                                    title: format!("New borrow opened: {} {:.4}", leg.asset_symbol, leg.amount_ui),
                                    message: format!(
                                        "You opened a new {} borrow of {:.4} (≈${:.2}) on {}.",
                                        leg.asset_symbol, leg.amount_ui, leg.value_usd, pos.protocol,
                                    ),
                                    health_score: risk.health_score,
                                    ltv: risk.ltv,
                                    suggested_actions: vec![],
                                    metadata: serde_json::json!({
                                        "asset": leg.asset_symbol,
                                        "amount_ui": leg.amount_ui,
                                        "value_usd": leg.value_usd,
                                        "protocol": pos.protocol,
                                    }),
                                    created_at: None,
                                    telegram_chat_id: chat_id,
                                });
                            } else if leg.amount_ui > prev_amount {
                                smart_alerts.push(AlertRecord {
                                    id: None,
                                    wallet: wallet.to_string(),
                                    severity: AlertSeverity::Info,
                                    title: format!("Borrowed more: {}", leg.asset_symbol),
                                    message: format!(
                                        "You borrowed an additional {:.4} {} on {}. Total borrow is now {:.4} (≈${:.2}).",
                                        amount_diff, leg.asset_symbol, pos.protocol, leg.amount_ui, leg.value_usd,
                                    ),
                                    health_score: risk.health_score,
                                    ltv: risk.ltv,
                                    suggested_actions: vec![],
                                    metadata: serde_json::json!({ "asset": leg.asset_symbol, "amount_increased": amount_diff }),
                                    created_at: None,
                                    telegram_chat_id: chat_id,
                                });
                            } else {
                                smart_alerts.push(AlertRecord {
                                    id: None,
                                    wallet: wallet.to_string(),
                                    severity: AlertSeverity::Info,
                                    title: format!("Repaid borrow: {}", leg.asset_symbol),
                                    message: format!(
                                        "You repaid {:.4} {} on {}. Remaining borrow is {:.4} (≈${:.2}).",
                                        amount_diff, leg.asset_symbol, pos.protocol, leg.amount_ui, leg.value_usd,
                                    ),
                                    health_score: risk.health_score,
                                    ltv: risk.ltv,
                                    suggested_actions: vec![],
                                    metadata: serde_json::json!({ "asset": leg.asset_symbol, "amount_decreased": amount_diff }),
                                    created_at: None,
                                    telegram_chat_id: chat_id,
                                });
                            }
                        }
                        PositionSide::Collateral => {
                            if prev_amount == 0.0 {
                                smart_alerts.push(AlertRecord {
                                    id: None,
                                    wallet: wallet.to_string(),
                                    severity: AlertSeverity::Info,
                                    title: format!("New deposit: {} {:.4}", leg.asset_symbol, leg.amount_ui),
                                    message: format!(
                                        "You deposited {:.4} {} (≈${:.2}) as collateral on {}.",
                                        leg.amount_ui, leg.asset_symbol, leg.value_usd, pos.protocol,
                                    ),
                                    health_score: risk.health_score,
                                    ltv: risk.ltv,
                                    suggested_actions: vec![],
                                    metadata: serde_json::json!({
                                        "asset": leg.asset_symbol,
                                        "amount_ui": leg.amount_ui,
                                        "value_usd": leg.value_usd,
                                        "protocol": pos.protocol,
                                    }),
                                    created_at: None,
                                    telegram_chat_id: chat_id,
                                });
                            } else if leg.amount_ui > prev_amount {
                                smart_alerts.push(AlertRecord {
                                    id: None,
                                    wallet: wallet.to_string(),
                                    severity: AlertSeverity::Info,
                                    title: format!("Added collateral: {}", leg.asset_symbol),
                                    message: format!(
                                        "You deposited an additional {:.4} {} on {}. Total collateral is now {:.4} (≈${:.2}).",
                                        amount_diff, leg.asset_symbol, pos.protocol, leg.amount_ui, leg.value_usd,
                                    ),
                                    health_score: risk.health_score,
                                    ltv: risk.ltv,
                                    suggested_actions: vec![],
                                    metadata: serde_json::json!({ "asset": leg.asset_symbol, "amount_increased": amount_diff }),
                                    created_at: None,
                                    telegram_chat_id: chat_id,
                                });
                            } else {
                                smart_alerts.push(AlertRecord {
                                    id: None,
                                    wallet: wallet.to_string(),
                                    severity: AlertSeverity::Info,
                                    title: format!("Withdrew collateral: {}", leg.asset_symbol),
                                    message: format!(
                                        "You withdrew {:.4} {} from {}. Remaining collateral is {:.4} (≈${:.2}).",
                                        amount_diff, leg.asset_symbol, pos.protocol, leg.amount_ui, leg.value_usd,
                                    ),
                                    health_score: risk.health_score,
                                    ltv: risk.ltv,
                                    suggested_actions: vec![],
                                    metadata: serde_json::json!({ "asset": leg.asset_symbol, "amount_decreased": amount_diff }),
                                    created_at: None,
                                    telegram_chat_id: chat_id,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Fully closed positions (key existed last cycle but not this one)
        for (key, (prev_amount, symbol)) in &last_amounts {
            if !current_keys.contains(key) && *prev_amount > 0.0 {
                let side_str = match key.2 {
                    PositionSide::Borrow => "borrow",
                    PositionSide::Collateral => "collateral deposit",
                };
                smart_alerts.push(AlertRecord {
                    id: None,
                    wallet: wallet.to_string(),
                    severity: AlertSeverity::Info,
                    title: format!("Position closed: {}", symbol),
                    message: format!(
                        "You fully closed your {} {} on {}.",
                        symbol, side_str, key.0,
                    ),
                    health_score: risk.health_score,
                    ltv: risk.ltv,
                    suggested_actions: vec![],
                    metadata: serde_json::json!({ "asset": symbol, "closed_side": side_str }),
                    created_at: None,
                    telegram_chat_id: chat_id,
                });
            }
        }

        for alert in smart_alerts {
            persist_alert(&state, &alert).await?;
            broadcast(dispatchers, &alert).await;
        }
    }

    state.last_risk.insert(wallet.to_string(), risk.clone());

    let rules = load_guard_rules(&state, wallet).await?;
    let tripped: Vec<&GuardRule> = matching_guard_rules(&risk, &rules, prev_health);

    // Mode selection: rules-driven vs. threshold fallback.
    let fire = if rules.is_empty() {
        // No rules configured — use the global default threshold.
        if risk.health_score > alert_threshold {
            return Ok(None);
        }
        if alert_recently_sent(&state, wallet).await? {
            return Ok(None);
        }
        true
    } else {
        let now = chrono::Utc::now();
        let mut fireable: Vec<&GuardRule> = Vec::new();
        for rule in tripped.iter().copied() {
            if !rule_cooldown_elapsed(rule, now) {
                continue;
            }
            if rule.daily_limit_usd > 0.0 {
                if let Some(id) = &rule.id {
                    let count = daily_alert_count(&state, id).await.unwrap_or(0);
                    if count >= rule.daily_limit_usd as i64 {
                        continue;
                    }
                }
            }
            fireable.push(rule);
        }
        if fireable.is_empty() {
            return Ok(None);
        }
        for rule in &fireable {
            if let Some(id) = rule.id.as_ref() {
                stamp_rule_fired(&state, id).await?;
            }
        }
        true
    };

    if !fire {
        return Ok(None);
    }

    let payload = llm.explain_risk(&risk).await?;
    let mut alert: AlertRecord = into_alert_record(risk, payload);
    alert.telegram_chat_id = load_telegram_chat_id(&state, &alert.wallet).await.ok().flatten();
    persist_alert(&state, &alert).await?;
    broadcast(dispatchers, &alert).await;
    Ok(Some(alert))
}

pub async fn start_alert_engine(
    state: Arc<AppState>,
    dispatchers: Vec<Arc<dyn Dispatcher>>,
    poll_interval_secs: u64,
    alert_threshold: f64,
) {
    let llm = LlmClient::from_env();
    info!(
        "[alerts] engine started: poll={}s threshold={} dispatchers={}",
        poll_interval_secs,
        alert_threshold,
        dispatchers.len()
    );

    loop {
        let wallets: Vec<String> = state
            .monitored_wallets
            .iter()
            .filter_map(|entry| (*entry.value()).then(|| entry.key().clone()))
            .collect();

        for wallet in wallets {
            if let Err(err) =
                evaluate_wallet(state.clone(), &llm, &dispatchers, &wallet, alert_threshold).await
            {
                error!("[alerts] evaluation failed for {}: {}", wallet, err);
            }
        }

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
    }
}

/// A rule's cooldown has elapsed if it has never fired, or enough time has
/// passed since it last did.
fn rule_cooldown_elapsed(rule: &GuardRule, now: chrono::DateTime<chrono::Utc>) -> bool {
    let Some(last) = rule.last_fired_at else {
        return true;
    };
    (now - last).num_seconds() >= rule.cooldown_seconds
}

async fn stamp_rule_fired(state: &AppState, rule_id: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE guard_rules SET last_fired_at = NOW() WHERE id = $1::uuid")
        .bind(rule_id)
        .execute(&state.db_pool)
        .await?;
    Ok(())
}

pub async fn load_guard_rules(state: &AppState, wallet: &str) -> anyhow::Result<Vec<GuardRule>> {
    let rows = sqlx::query(
        "SELECT id, wallet_pubkey, protocol, trigger_kind, trigger_value, action_kind, action_token, action_amount_usd, max_usd_per_action, daily_limit_usd, cooldown_seconds, is_active, created_at, updated_at, last_fired_at
         FROM guard_rules
         WHERE wallet_pubkey = $1
         ORDER BY created_at DESC",
    )
    .bind(wallet)
    .fetch_all(&state.db_pool)
    .await?;

    rows.into_iter().map(map_guard_rule).collect()
}

pub fn matching_guard_rules<'a>(risk: &WalletRisk, rules: &'a [GuardRule], last_health: Option<f64>) -> Vec<&'a GuardRule> {
    rules.iter()
        .filter(|rule| rule.is_active)
        .filter(|rule| rule.wallet == risk.wallet)
        .filter(|rule| {
            if let Some(protocol) = &rule.protocol {
                risk.protocols.iter().any(|p| &p.protocol == protocol)
            } else {
                true
            }
        })
        .filter(|rule| match rule.trigger_kind {
            aegis_core::types::TriggerKind::HealthBelow => risk.health_score < rule.trigger_value,
            aegis_core::types::TriggerKind::LtvAbove => risk.ltv > rule.trigger_value,
            aegis_core::types::TriggerKind::DebtAboveUsd => risk.total_debt_usd > rule.trigger_value,
            aegis_core::types::TriggerKind::HealthDropped => {
                // Skips first cycle after restart (last_health is None, no baseline yet).
                let Some(last) = last_health else { return false };
                if last <= 0.0 { return false; }
                let pct_drop = (last - risk.health_score) / last;
                pct_drop > rule.trigger_value
            }
        })
        .collect()
}

async fn load_telegram_chat_id(state: &AppState, wallet: &str) -> anyhow::Result<Option<i64>> {
    let row = sqlx::query("SELECT telegram_chat_id FROM wallets WHERE pubkey = $1")
        .bind(wallet)
        .fetch_optional(&state.db_pool)
        .await?;
    Ok(row.and_then(|r| r.try_get::<Option<i64>, _>("telegram_chat_id").ok().flatten()))
}

async fn daily_alert_count(state: &AppState, rule_id: &str) -> anyhow::Result<i64> {
    let row = sqlx::query(
        "SELECT COUNT(*) as cnt FROM alerts
         WHERE guard_rule_id = $1::uuid AND created_at >= NOW() - INTERVAL '24 hours'",
    )
    .bind(rule_id)
    .fetch_one(&state.db_pool)
    .await?;
    Ok(row.try_get::<i64, _>("cnt").unwrap_or(0))
}

async fn alert_recently_sent(state: &AppState, wallet: &str) -> anyhow::Result<bool> {
    let row = sqlx::query(
        "SELECT created_at
         FROM alerts
         WHERE wallet_pubkey = $1
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(wallet)
    .fetch_optional(&state.db_pool)
    .await?;

    let Some(row) = row else {
        return Ok(false);
    };

    let created_at: chrono::DateTime<chrono::Utc> = row.try_get("created_at")?;
    Ok((chrono::Utc::now() - created_at).num_minutes() < 60)
}

async fn persist_alert(state: &AppState, alert: &AlertRecord) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO alerts (wallet_pubkey, severity, title, message, health_score, ltv, suggested_actions, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&alert.wallet)
    .bind(format!("{:?}", alert.severity))
    .bind(&alert.title)
    .bind(&alert.message)
    .bind(alert.health_score)
    .bind(alert.ltv)
    .bind(sqlx::types::Json(&alert.suggested_actions))
    .bind(sqlx::types::Json(&alert.metadata))
    .execute(&state.db_pool)
    .await?;

    Ok(())
}

fn map_guard_rule(row: sqlx::postgres::PgRow) -> anyhow::Result<GuardRule> {
    Ok(GuardRule {
        id: Some(row.try_get::<uuid::Uuid, _>("id")?.to_string()),
        wallet: row.try_get("wallet_pubkey")?,
        protocol: row.try_get("protocol")?,
        trigger_kind: parse_trigger_kind(&row.try_get::<String, _>("trigger_kind")?)?,
        trigger_value: row.try_get("trigger_value")?,
        action_kind: parse_action_kind(&row.try_get::<String, _>("action_kind")?)?,
        action_token: row.try_get("action_token")?,
        action_amount_usd: row.try_get("action_amount_usd")?,
        max_usd_per_action: row.try_get("max_usd_per_action")?,
        daily_limit_usd: row.try_get("daily_limit_usd")?,
        cooldown_seconds: row.try_get("cooldown_seconds")?,
        is_active: row.try_get("is_active")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_fired_at: row.try_get("last_fired_at")?,
    })
}

fn parse_trigger_kind(value: &str) -> anyhow::Result<aegis_core::types::TriggerKind> {
    match value {
        "health_below" => Ok(aegis_core::types::TriggerKind::HealthBelow),
        "ltv_above" => Ok(aegis_core::types::TriggerKind::LtvAbove),
        "debt_above_usd" => Ok(aegis_core::types::TriggerKind::DebtAboveUsd),
        "health_dropped" => Ok(aegis_core::types::TriggerKind::HealthDropped),
        _ => anyhow::bail!("unknown trigger kind: {value}"),
    }
}

fn parse_action_kind(value: &str) -> anyhow::Result<aegis_core::types::ActionKind> {
    match value {
        "notify_only" => Ok(aegis_core::types::ActionKind::NotifyOnly),
        "add_collateral" => Ok(aegis_core::types::ActionKind::AddCollateral),
        "repay_debt" => Ok(aegis_core::types::ActionKind::RepayDebt),
        "deleverage" => Ok(aegis_core::types::ActionKind::Deleverage),
        _ => anyhow::bail!("unknown action kind: {value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegis_core::types::{ActionKind, AlertSeverity, ProtocolHealth, TriggerKind};
    use chrono::Duration;

    fn rule(wallet: &str, trigger: TriggerKind, value: f64) -> GuardRule {
        GuardRule {
            id: Some("rule-1".into()),
            wallet: wallet.into(),
            protocol: None,
            trigger_kind: trigger,
            trigger_value: value,
            action_kind: ActionKind::NotifyOnly,
            action_token: None,
            action_amount_usd: None,
            max_usd_per_action: 0.0,
            daily_limit_usd: 0.0,
            cooldown_seconds: 3600,
            is_active: true,
            created_at: None,
            updated_at: None,
            last_fired_at: None,
        }
    }

    fn risk(wallet: &str, health: f64, ltv: f64, debt: f64) -> WalletRisk {
        WalletRisk {
            wallet: wallet.into(),
            health_score: health,
            severity: AlertSeverity::Warning,
            total_collateral_usd: 1_000.0,
            total_debt_usd: debt,
            ltv,
            liquidation_threshold: 0.85,
            liquidation_buffer_usd: 100.0,
            protocols: vec![ProtocolHealth {
                protocol: "Kamino".into(),
                collateral_usd: 1_000.0,
                debt_usd: debt,
                ltv,
                liquidation_threshold: 0.85,
            }],
            positions: vec![],
        }
    }

    #[test]
    fn inactive_rules_are_skipped() {
        let mut r = rule("w", TriggerKind::HealthBelow, 50.0);
        r.is_active = false;
        let matched = matching_guard_rules(&risk("w", 10.0, 0.9, 500.0), std::slice::from_ref(&r), None);
        assert!(matched.is_empty());
    }

    #[test]
    fn wallet_mismatch_is_skipped() {
        let r = rule("other", TriggerKind::HealthBelow, 50.0);
        let matched = matching_guard_rules(&risk("w", 10.0, 0.9, 500.0), std::slice::from_ref(&r), None);
        assert!(matched.is_empty());
    }

    #[test]
    fn protocol_filter_requires_match() {
        let mut r = rule("w", TriggerKind::HealthBelow, 50.0);
        r.protocol = Some("Save".into());
        let matched = matching_guard_rules(&risk("w", 10.0, 0.9, 500.0), std::slice::from_ref(&r), None);
        assert!(matched.is_empty(), "risk has only Kamino — Save rule should not match");
    }

    #[test]
    fn health_below_trigger_fires_when_under() {
        let r = rule("w", TriggerKind::HealthBelow, 50.0);
        assert_eq!(matching_guard_rules(&risk("w", 40.0, 0.5, 0.0), std::slice::from_ref(&r), None).len(), 1);
        assert!(matching_guard_rules(&risk("w", 60.0, 0.5, 0.0), std::slice::from_ref(&r), None).is_empty());
    }

    #[test]
    fn ltv_above_trigger_fires_when_over() {
        let r = rule("w", TriggerKind::LtvAbove, 0.7);
        assert_eq!(matching_guard_rules(&risk("w", 50.0, 0.8, 0.0), std::slice::from_ref(&r), None).len(), 1);
        assert!(matching_guard_rules(&risk("w", 50.0, 0.5, 0.0), std::slice::from_ref(&r), None).is_empty());
    }

    #[test]
    fn debt_above_usd_trigger_fires_when_over() {
        let r = rule("w", TriggerKind::DebtAboveUsd, 1_000.0);
        assert_eq!(matching_guard_rules(&risk("w", 50.0, 0.5, 2_000.0), std::slice::from_ref(&r), None).len(), 1);
        assert!(matching_guard_rules(&risk("w", 50.0, 0.5, 500.0), std::slice::from_ref(&r), None).is_empty());
    }

    #[test]
    fn health_dropped_trigger_fires_on_large_drop() {
        // trigger_value = 0.15 → fire when health drops more than 15%.
        let r = rule("w", TriggerKind::HealthDropped, 0.15);
        // Health was 80, now 60 → 25% drop → should fire.
        assert_eq!(
            matching_guard_rules(&risk("w", 60.0, 0.5, 0.0), std::slice::from_ref(&r), Some(80.0)).len(),
            1
        );
        // Health was 80, now 75 → 6.25% drop → should NOT fire.
        assert!(
            matching_guard_rules(&risk("w", 75.0, 0.5, 0.0), std::slice::from_ref(&r), Some(80.0)).is_empty()
        );
        // No previous health (restart) → should NOT fire.
        assert!(
            matching_guard_rules(&risk("w", 60.0, 0.5, 0.0), std::slice::from_ref(&r), None).is_empty()
        );
    }

    #[test]
    fn cooldown_elapsed_when_never_fired() {
        let r = rule("w", TriggerKind::HealthBelow, 50.0);
        assert!(rule_cooldown_elapsed(&r, chrono::Utc::now()));
    }

    #[test]
    fn cooldown_not_elapsed_when_recent() {
        let mut r = rule("w", TriggerKind::HealthBelow, 50.0);
        let now = chrono::Utc::now();
        r.last_fired_at = Some(now - Duration::seconds(100));
        r.cooldown_seconds = 3600;
        assert!(!rule_cooldown_elapsed(&r, now));
    }

    #[test]
    fn cooldown_elapsed_after_window() {
        let mut r = rule("w", TriggerKind::HealthBelow, 50.0);
        let now = chrono::Utc::now();
        r.last_fired_at = Some(now - Duration::seconds(4000));
        r.cooldown_seconds = 3600;
        assert!(rule_cooldown_elapsed(&r, now));
    }
}
