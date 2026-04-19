use std::{sync::Arc, time::Duration};

use aegis_core::{
    state::AppState,
    types::{AlertRecord, GuardRule},
};
use aegis_risk::health::{WalletRisk, wallet_risk};
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

    let rules = load_guard_rules(&state, wallet).await?;
    let tripped: Vec<&GuardRule> = matching_guard_rules(&risk, &rules);

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
        // Rules exist — only fire for rules whose cooldown has elapsed.
        let now = chrono::Utc::now();
        let fireable: Vec<&GuardRule> = tripped
            .iter()
            .copied()
            .filter(|r| rule_cooldown_elapsed(r, now))
            .collect();
        if fireable.is_empty() {
            return Ok(None);
        }
        // Stamp last_fired_at on each rule that will fire so cooldowns advance.
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
    let alert: AlertRecord = into_alert_record(risk, payload);
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

pub fn matching_guard_rules<'a>(risk: &WalletRisk, rules: &'a [GuardRule]) -> Vec<&'a GuardRule> {
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
        })
        .collect()
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
    use aegis_core::types::{ActionKind, AlertSeverity, TriggerKind};
    use aegis_risk::health::ProtocolHealth;
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
        let matched = matching_guard_rules(&risk("w", 10.0, 0.9, 500.0), std::slice::from_ref(&r));
        assert!(matched.is_empty());
    }

    #[test]
    fn wallet_mismatch_is_skipped() {
        let r = rule("other", TriggerKind::HealthBelow, 50.0);
        let matched = matching_guard_rules(&risk("w", 10.0, 0.9, 500.0), std::slice::from_ref(&r));
        assert!(matched.is_empty());
    }

    #[test]
    fn protocol_filter_requires_match() {
        let mut r = rule("w", TriggerKind::HealthBelow, 50.0);
        r.protocol = Some("Save".into());
        let matched = matching_guard_rules(&risk("w", 10.0, 0.9, 500.0), std::slice::from_ref(&r));
        assert!(matched.is_empty(), "risk has only Kamino — Save rule should not match");
    }

    #[test]
    fn health_below_trigger_fires_when_under() {
        let r = rule("w", TriggerKind::HealthBelow, 50.0);
        assert_eq!(matching_guard_rules(&risk("w", 40.0, 0.5, 0.0), std::slice::from_ref(&r)).len(), 1);
        assert!(matching_guard_rules(&risk("w", 60.0, 0.5, 0.0), std::slice::from_ref(&r)).is_empty());
    }

    #[test]
    fn ltv_above_trigger_fires_when_over() {
        let r = rule("w", TriggerKind::LtvAbove, 0.7);
        assert_eq!(matching_guard_rules(&risk("w", 50.0, 0.8, 0.0), std::slice::from_ref(&r)).len(), 1);
        assert!(matching_guard_rules(&risk("w", 50.0, 0.5, 0.0), std::slice::from_ref(&r)).is_empty());
    }

    #[test]
    fn debt_above_usd_trigger_fires_when_over() {
        let r = rule("w", TriggerKind::DebtAboveUsd, 1_000.0);
        assert_eq!(matching_guard_rules(&risk("w", 50.0, 0.5, 2_000.0), std::slice::from_ref(&r)).len(), 1);
        assert!(matching_guard_rules(&risk("w", 50.0, 0.5, 500.0), std::slice::from_ref(&r)).is_empty());
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
