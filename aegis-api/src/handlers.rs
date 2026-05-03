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
) -> Json<aegis_core::types::WalletRisk> {
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

/// Upsert a guard rule: UPDATE if an `id` is present, INSERT otherwise.
/// Returns the full persisted row so the frontend can sync its local id.
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
        telegram_chat_id: None,
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
        TriggerKind::HealthDropped => "health_dropped",
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

/// Reverse-lookup: given a Telegram chat_id, return the wallet pubkey linked to it.
/// Used by the bot's /status command so the user doesn't have to type their wallet.
#[derive(Serialize)]
pub struct ChatLookupResponse {
    pub wallet: String,
}

pub async fn wallet_by_chat(
    State(state): State<Arc<AppState>>,
    Path(chat_id): Path<i64>,
) -> Result<Json<ChatLookupResponse>, (StatusCode, String)> {
    let row = sqlx::query(
        "SELECT pubkey FROM wallets WHERE telegram_chat_id = $1 LIMIT 1",
    )
    .bind(chat_id)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "no wallet linked to this chat".to_string()));
    };

    let wallet: String = row.try_get("pubkey").map_err(internal_error)?;
    Ok(Json(ChatLookupResponse { wallet }))
}

/// Returns persisted wallet settings (telegram_chat_id, etc.) so the UI can pre-populate fields.
#[derive(Serialize)]
pub struct WalletSettings {
    pub telegram_chat_id: Option<i64>,
}

pub async fn get_wallet_settings(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<Json<WalletSettings>, (StatusCode, String)> {
    let row = sqlx::query("SELECT telegram_chat_id FROM wallets WHERE pubkey = $1")
        .bind(&wallet)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(internal_error)?;

    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "wallet not found".to_string()));
    };

    let telegram_chat_id: Option<i64> = row.try_get("telegram_chat_id").unwrap_or(None);
    Ok(Json(WalletSettings { telegram_chat_id }))
}

/// Link a Telegram chat ID to a wallet for per-wallet alert delivery.
/// Called by the aegis-bot after the user sends /link <wallet_pubkey>.
#[derive(serde::Deserialize)]
pub struct LinkTelegramBody {
    pub chat_id: i64,
}

pub async fn link_telegram(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
    Json(body): Json<LinkTelegramBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Only allow linking if no chat_id is set yet (prevents spoofing another wallet).
    let existing = sqlx::query(
        "SELECT telegram_chat_id FROM wallets WHERE pubkey = $1",
    )
    .bind(&wallet)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let Some(row) = existing else {
        return Err((StatusCode::NOT_FOUND, "wallet not found".to_string()));
    };

    let current: Option<i64> = row.try_get("telegram_chat_id").unwrap_or(None);
    if current.is_some() {
        // Already linked — allow re-link to the same chat (idempotent) but block others.
        if current != Some(body.chat_id) {
            return Err((StatusCode::CONFLICT, "wallet already linked to a different chat".to_string()));
        }
        return Ok(StatusCode::NO_CONTENT);
    }

    sqlx::query(
        "UPDATE wallets SET telegram_chat_id = $1 WHERE pubkey = $2",
    )
    .bind(body.chat_id)
    .bind(&wallet)
    .execute(&state.db_pool)
    .await
    .map_err(internal_error)?;

    // Send a rich welcome message with current positions — non-fatal if bot token is missing.
    if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
        let risk = wallet_risk(&state, &wallet);
        let text = format_welcome_message(&wallet, &risk);
        let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
        if let Err(e) = reqwest::Client::new()
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": body.chat_id,
                "text": text,
                "parse_mode": "Markdown",
                "disable_web_page_preview": true,
            }))
            .send()
            .await
        {
            error!("telegram welcome message failed: {}", e);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a guard rule by ID. The frontend delete button calls this.
pub async fn delete_guard_rule(
    State(state): State<Arc<AppState>>,
    Path(rule_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query("DELETE FROM guard_rules WHERE id = $1::uuid")
        .bind(&rule_id)
        .execute(&state.db_pool)
        .await
        .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "rule not found".to_string()));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Link an email address to a wallet for future email notifications.
#[derive(serde::Deserialize)]
pub struct LinkEmailBody {
    pub email: String,
}

pub async fn link_email(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
    Json(body): Json<LinkEmailBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    let exists = sqlx::query("SELECT 1 FROM wallets WHERE pubkey = $1")
        .bind(&wallet)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(internal_error)?;

    if exists.is_none() {
        return Err((StatusCode::NOT_FOUND, "wallet not found".to_string()));
    }
    sqlx::query("UPDATE wallets SET email = $1 WHERE pubkey = $2")
        .bind(&body.email)
        .bind(&wallet)
        .execute(&state.db_pool)
        .await
        .map_err(internal_error)?;

    Ok(StatusCode::NO_CONTENT)
}

fn format_welcome_message(wallet: &str, risk: &aegis_core::types::WalletRisk) -> String {
    let short = |w: &str| {
        if w.len() > 12 {
            format!("{}…{}", &w[..4], &w[w.len() - 4..])
        } else {
            w.to_string()
        }
    };

    let mut lines = vec![
        "🛡️ *Aegis connected!*".to_string(),
        String::new(),
        format!("📍 *Wallet:* `{}`", short(wallet)),
    ];

    // Build per-leg position list from all positions.
    let mut position_lines: Vec<String> = Vec::new();
    for pos in &risk.positions {
        if pos.legs.is_empty() {
            // Aggregate fallback (no per-asset breakdown).
            if pos.collateral_usd > 0.0 {
                position_lines.push(format!(
                    "• {} · (Collateral) — *${:.2}*",
                    pos.protocol, pos.collateral_usd
                ));
            }
            if pos.debt_usd > 0.0 {
                position_lines.push(format!(
                    "• {} · (Borrow) — *${:.2}*",
                    pos.protocol, pos.debt_usd
                ));
            }
        } else {
            for leg in &pos.legs {
                let side = match leg.side {
                    aegis_core::types::PositionSide::Collateral => "Collateral",
                    aegis_core::types::PositionSide::Borrow => "Borrow",
                };
                let usd = if leg.value_usd > 0.0 {
                    format!("*${:.2}*", leg.value_usd)
                } else {
                    format!("{:.4} {}", leg.amount_ui, leg.asset_symbol)
                };
                position_lines.push(format!(
                    "• {} · {} ({}) — {}",
                    pos.protocol, leg.asset_symbol, side, usd
                ));
            }
        }
    }

    let n = position_lines.len();
    lines.push(format!("📊 *{} position{} found:*", n, if n == 1 { "" } else { "s" }));
    lines.push(String::new());
    lines.extend(position_lines);
    lines.push(String::new());

    let health_emoji = if risk.health_score >= 65.0 { "💚" } else if risk.health_score >= 40.0 { "⚠️" } else { "🔴" };
    lines.push(format!(
        "{} *Health:* {:.0}/100   *LTV:* {:.1}%   *Buffer:* ${:.2}",
        health_emoji,
        risk.health_score,
        risk.ltv * 100.0,
        risk.liquidation_buffer_usd
    ));
    lines.push(String::new());
    lines.push("You'll receive alerts here for:".to_string());
    lines.push("• Sudden health drops".to_string());
    lines.push("• LTV approaching liquidation".to_string());
    lines.push("• Collateral price movements".to_string());

    lines.join("\n")
}

fn internal_error<E: std::fmt::Display>(err: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
