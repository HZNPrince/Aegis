use std::{collections::HashMap, sync::Arc};

use aegis_alerts::engine::load_guard_rules;
use aegis_core::{
    state::AppState,
    types::{ActionKind, AlertRecord, AlertSeverity, GuardRule, TriggerKind},
};
use aegis_executor::{build_repay_tx, BuildRepayRequest, ExecutorContext, UnsignedTx};
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
pub struct PriceTick {
    pub price: f64,
    pub change_24h: Option<f64>,
}

/// Richer price view including Jupiter's 24h change. Used by the ticker rail.
pub async fn ticker(
    State(state): State<Arc<AppState>>,
) -> Json<HashMap<String, PriceTick>> {
    let mut map = HashMap::new();
    for entry in state.token_prices.iter() {
        let mint = entry.key();
        map.insert(
            mint.clone(),
            PriceTick {
                price: *entry.value(),
                change_24h: state.token_price_changes.get(mint).map(|v| *v),
            },
        );
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

#[derive(serde::Deserialize)]
pub struct BuildRepayBody {
    pub wallet: String,
    pub obligation_or_account: String,
    pub protocol: String,
    pub reserve_or_bank: String,
    pub mint: String,
    pub amount_native: u64,
    pub guard_rule_id: Option<String>,
}

#[derive(Serialize)]
pub struct BuildRepayResponse {
    pub intent_id: String,
    pub unsigned: UnsignedTx,
}

/// Build an unsigned repay tx and persist it as an execution_intent.
/// Frontend picks it up, opens Phantom to sign, and calls PATCH later.
pub async fn build_repay(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BuildRepayBody>,
) -> Result<Json<BuildRepayResponse>, (StatusCode, String)> {
    let rpc_url = std::env::var("RPC_ENDPOINT")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "RPC_ENDPOINT not set".to_string()))?;
    let ctx = ExecutorContext::new(&rpc_url);

    let rule = if let Some(id) = &body.guard_rule_id {
        let rules = load_guard_rules(&state, &body.wallet)
            .await
            .map_err(internal_error)?;
        rules.into_iter().find(|r| r.id.as_deref() == Some(id.as_str()))
    } else {
        None
    };

    let req = BuildRepayRequest {
        wallet: body.wallet.clone(),
        obligation_or_account: body.obligation_or_account.clone(),
        protocol: body.protocol.clone(),
        reserve_or_bank: body.reserve_or_bank.clone(),
        mint: body.mint.clone(),
        amount_native: body.amount_native,
        rule,
    };

    let unsigned = build_repay_tx(&ctx, &req)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ctx.intent_ttl_secs);

    let row = sqlx::query(
        "INSERT INTO execution_intents
         (wallet_pubkey, guard_rule_id, protocol, obligation_or_account, reserve_or_bank,
          mint, amount_native, unsigned_tx, last_valid_block_height, expires_at)
         VALUES ($1, $2::uuid, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(&body.wallet)
    .bind(body.guard_rule_id.as_deref())
    .bind(&body.protocol)
    .bind(&body.obligation_or_account)
    .bind(&body.reserve_or_bank)
    .bind(&body.mint)
    .bind(body.amount_native as i64)
    .bind(&unsigned.tx_base64)
    .bind(unsigned.last_valid_block_height as i64)
    .bind(expires_at)
    .fetch_one(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let intent_id = row
        .try_get::<uuid::Uuid, _>("id")
        .map_err(internal_error)?
        .to_string();

    Ok(Json(BuildRepayResponse {
        intent_id,
        unsigned,
    }))
}

#[derive(serde::Deserialize)]
pub struct UpdateIntentBody {
    pub status: String,
    pub signature: Option<String>,
    pub error: Option<String>,
}

pub async fn update_intent(
    State(state): State<Arc<AppState>>,
    Path(intent_id): Path<String>,
    Json(body): Json<UpdateIntentBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    const ALLOWED: &[&str] = &["signed", "submitted", "confirmed", "expired", "cancelled"];
    if !ALLOWED.contains(&body.status.as_str()) {
        return Err((StatusCode::BAD_REQUEST, format!("bad status: {}", body.status)));
    }

    sqlx::query(
        "UPDATE execution_intents
         SET status = $2, signature = COALESCE($3, signature),
             error = COALESCE($4, error), updated_at = NOW()
         WHERE id = $1::uuid",
    )
    .bind(&intent_id)
    .bind(&body.status)
    .bind(body.signature.as_deref())
    .bind(body.error.as_deref())
    .execute(&state.db_pool)
    .await
    .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct IntentRow {
    pub id: String,
    pub protocol: String,
    pub mint: String,
    pub amount_native: i64,
    pub status: String,
    pub signature: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_intents(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<Json<Vec<IntentRow>>, (StatusCode, String)> {
    let rows = sqlx::query(
        "SELECT id, protocol, mint, amount_native, status, signature, created_at, expires_at
         FROM execution_intents
         WHERE wallet_pubkey = $1
         ORDER BY created_at DESC
         LIMIT 50",
    )
    .bind(&wallet)
    .fetch_all(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let items = rows
        .into_iter()
        .map(|r| {
            Ok::<_, sqlx::Error>(IntentRow {
                id: r.try_get::<uuid::Uuid, _>("id")?.to_string(),
                protocol: r.try_get("protocol")?,
                mint: r.try_get("mint")?,
                amount_native: r.try_get("amount_native")?,
                status: r.try_get("status")?,
                signature: r.try_get("signature")?,
                created_at: r.try_get("created_at")?,
                expires_at: r.try_get("expires_at")?,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;

    Ok(Json(items))
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
