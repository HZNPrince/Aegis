use std::{sync::Arc, time::Duration};

use aegis_core::{
    state::AppState,
    types::{AlertRecord, GuardRule},
};
use aegis_risk::health::{WalletRisk, wallet_risk};
use sqlx::Row;
use tracing::{error, info};

use crate::llm::{LlmClient, into_alert_record};

pub async fn evaluate_wallet(
    state: Arc<AppState>,
    llm: &LlmClient,
    wallet: &str,
    alert_threshold: f64,
) -> anyhow::Result<Option<AlertRecord>> {
    let risk = wallet_risk(&state, wallet);
    if risk.total_collateral_usd <= 0.0 && risk.total_debt_usd <= 0.0 {
        return Ok(None);
    }

    if risk.health_score > alert_threshold {
        return Ok(None);
    }

    if alert_recently_sent(&state, wallet).await? {
        return Ok(None);
    }

    let payload = llm.explain_risk(&risk).await?;
    let alert: AlertRecord = into_alert_record(risk, payload);
    persist_alert(&state, &alert).await?;
    Ok(Some(alert))
}

pub async fn start_alert_engine(
    state: Arc<AppState>,
    poll_interval_secs: u64,
    alert_threshold: f64,
) {
    let llm = LlmClient::from_env();
    info!(
        "alert engine started: poll_interval={}s threshold={}",
        poll_interval_secs, alert_threshold
    );

    loop {
        let wallets: Vec<String> = state
            .monitored_wallets
            .iter()
            .filter_map(|entry| (*entry.value()).then(|| entry.key().clone()))
            .collect();

        for wallet in wallets {
            if let Err(err) = evaluate_wallet(state.clone(), &llm, &wallet, alert_threshold).await {
                error!("alert evaluation failed for {}: {}", wallet, err);
            }
        }

        tokio::time::sleep(Duration::from_secs(poll_interval_secs)).await;
    }
}

pub async fn load_guard_rules(state: &AppState, wallet: &str) -> anyhow::Result<Vec<GuardRule>> {
    let rows = sqlx::query(
        "SELECT id, wallet_pubkey, protocol, trigger_kind, trigger_value, action_kind, action_token, action_amount_usd, max_usd_per_action, daily_limit_usd, cooldown_seconds, is_active, created_at, updated_at
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
