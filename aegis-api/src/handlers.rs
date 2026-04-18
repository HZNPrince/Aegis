use std::{collections::HashMap, sync::Arc};

use aegis_alerts::engine::load_guard_rules;
use aegis_core::{
    state::AppState,
    types::{ActionKind, AlertRecord, AlertSeverity, GuardRule, TriggerKind},
};
use aegis_risk::{
    health::wallet_risk,
    scenario::{ScenarioRequest, ScenarioResponse, simulate},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;
use sqlx::Row;
use tracing::error;

#[derive(Serialize)]
pub struct StatusResponse {
    pub positions_cached: usize,
    pub prices_loaded: usize,
    pub wallets_monitored: usize,
    pub bank_cache_size: usize,
}

pub async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    Json(StatusResponse {
        positions_cached: state.positions.len(),
        prices_loaded: state.token_prices.len(),
        wallets_monitored: state.monitored_wallets.len(),
        bank_cache_size: state.bank_cache.len(),
    })
}

pub async fn prices(State(state): State<Arc<AppState>>) -> Json<HashMap<String, f64>> {
    let mut map = HashMap::new();
    for entry in state.token_prices.iter() {
        map.insert(entry.key().clone(), *entry.value());
    }
    Json(map)
}

#[derive(Serialize)]
pub struct LinkWalletResponse {
    pub wallet: String,
    pub backfilled_positions: usize,
}

/// Link a wallet: mark it monitored, persist it, and run a best-effort
/// backfill of its current positions across Kamino/Save/Marginfi via RPC.
///
/// Backfill is synchronous so the response reports how many positions were
/// found. If RPC rate-limits or times out we still return 200 with 0 — live
/// stream updates will pick the wallet up from here on.
pub async fn link_wallet(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<Json<LinkWalletResponse>, (StatusCode, String)> {
    state.monitored_wallets.insert(wallet.clone(), true);

    if let Err(e) = sqlx::query(
        "INSERT INTO wallets (pubkey) VALUES ($1) ON CONFLICT (pubkey) DO NOTHING",
    )
    .bind(&wallet)
    .execute(&state.db_pool)
    .await
    {
        return Err(internal_error(e));
    }

    let rpc_url = std::env::var("RPC_ENDPOINT")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "RPC_ENDPOINT not set".to_string()))?;

    let count = match aegis_indexer::backfill::backfill_wallet(&rpc_url, state.clone(), &wallet)
        .await
    {
        Ok(n) => n,
        Err(e) => {
            error!("backfill failed for {}: {}", wallet, e);
            0
        }
    };

    Ok(Json(LinkWalletResponse {
        wallet,
        backfilled_positions: count,
    }))
}

pub async fn wallet_health(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Json<aegis_risk::health::WalletRisk> {
    Json(wallet_risk(&state, &wallet))
}

pub async fn scenario(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ScenarioRequest>,
) -> Result<Json<ScenarioResponse>, (StatusCode, String)> {
    let base = wallet_risk(&state, &request.wallet);
    if base.positions.is_empty() {
        return Err((StatusCode::NOT_FOUND, "wallet has no tracked positions".to_string()));
    }

    Ok(Json(simulate(base, &request)))
}

pub async fn list_alerts(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<Json<Vec<AlertRecord>>, (StatusCode, String)> {
    let rows = sqlx::query(
        "SELECT id, wallet_pubkey, severity, title, message, health_score, ltv, suggested_actions, metadata, created_at
         FROM alerts
         WHERE wallet_pubkey = $1
         ORDER BY created_at DESC
         LIMIT 100",
    )
    .bind(&wallet)
    .fetch_all(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let alerts = rows
        .into_iter()
        .map(map_alert_record)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(alerts))
}

pub async fn list_guard_rules(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<Json<Vec<GuardRule>>, (StatusCode, String)> {
    load_guard_rules(&state, &wallet)
        .await
        .map(Json)
        .map_err(internal_error)
}

pub async fn upsert_guard_rule(
    State(state): State<Arc<AppState>>,
    Json(rule): Json<GuardRule>,
) -> Result<Json<GuardRule>, (StatusCode, String)> {
    let mut stored = rule;
    let row = if let Some(id) = stored.id.as_ref() {
        sqlx::query(
            "UPDATE guard_rules
             SET protocol = $2,
                 trigger_kind = $3,
                 trigger_value = $4,
                 action_kind = $5,
                 action_token = $6,
                 action_amount_usd = $7,
                 max_usd_per_action = $8,
                 daily_limit_usd = $9,
                 cooldown_seconds = $10,
                 is_active = $11,
                 updated_at = NOW()
             WHERE id = $1::uuid
             RETURNING id, created_at, updated_at",
        )
        .bind(id)
        .bind(&stored.protocol)
        .bind(trigger_kind_db(stored.trigger_kind))
        .bind(stored.trigger_value)
        .bind(action_kind_db(stored.action_kind))
        .bind(&stored.action_token)
        .bind(stored.action_amount_usd)
        .bind(stored.max_usd_per_action)
        .bind(stored.daily_limit_usd)
        .bind(stored.cooldown_seconds)
        .bind(stored.is_active)
        .fetch_one(&state.db_pool)
        .await
        .map_err(internal_error)?
    } else {
        sqlx::query(
            "INSERT INTO guard_rules
             (wallet_pubkey, protocol, trigger_kind, trigger_value, action_kind, action_token, action_amount_usd, max_usd_per_action, daily_limit_usd, cooldown_seconds, is_active)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING id, created_at, updated_at",
        )
        .bind(&stored.wallet)
        .bind(&stored.protocol)
        .bind(trigger_kind_db(stored.trigger_kind))
        .bind(stored.trigger_value)
        .bind(action_kind_db(stored.action_kind))
        .bind(&stored.action_token)
        .bind(stored.action_amount_usd)
        .bind(stored.max_usd_per_action)
        .bind(stored.daily_limit_usd)
        .bind(stored.cooldown_seconds)
        .bind(stored.is_active)
        .fetch_one(&state.db_pool)
        .await
        .map_err(internal_error)?
    };

    stored.id = Some(row.try_get::<uuid::Uuid, _>("id").map_err(internal_error)?.to_string());
    stored.created_at = row.try_get("created_at").map_err(internal_error)?;
    stored.updated_at = row.try_get("updated_at").map_err(internal_error)?;

    Ok(Json(stored))
}

fn map_alert_record(row: sqlx::postgres::PgRow) -> anyhow::Result<AlertRecord> {
    let suggested_actions: sqlx::types::Json<Vec<String>> = row.try_get("suggested_actions")?;
    let metadata: sqlx::types::Json<serde_json::Value> = row.try_get("metadata")?;

    Ok(AlertRecord {
        id: Some(row.try_get::<uuid::Uuid, _>("id")?.to_string()),
        wallet: row.try_get("wallet_pubkey")?,
        severity: parse_alert_severity(&row.try_get::<String, _>("severity")?),
        title: row.try_get("title")?,
        message: row.try_get("message")?,
        health_score: row.try_get("health_score")?,
        ltv: row.try_get("ltv")?,
        suggested_actions: suggested_actions.0,
        metadata: metadata.0,
        created_at: row.try_get("created_at")?,
    })
}

fn parse_alert_severity(value: &str) -> AlertSeverity {
    match value {
        "Critical" => AlertSeverity::Critical,
        "Warning" => AlertSeverity::Warning,
        _ => AlertSeverity::Info,
    }
}

fn trigger_kind_db(kind: TriggerKind) -> &'static str {
    match kind {
        TriggerKind::HealthBelow => "health_below",
        TriggerKind::LtvAbove => "ltv_above",
        TriggerKind::DebtAboveUsd => "debt_above_usd",
    }
}

fn action_kind_db(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::NotifyOnly => "notify_only",
        ActionKind::AddCollateral => "add_collateral",
        ActionKind::RepayDebt => "repay_debt",
        ActionKind::Deleverage => "deleverage",
    }
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
