//! HTTP request handlers — implements all API endpoints for the REST server.
//! Provides status, prices, health, guard rules, alerts, intents, and wallet management.
//! All handlers receive AppState and return JSON responses; errors are logged but don't crash the server.

use aegis_alerts::engine::load_guard_rules;
use aegis_core::{
    state::AppState,
    types::{ActionKind, AlertRecord, AlertSeverity, GuardRule, TriggerKind},
};
use aegis_executor::ExecutorContext;
use aegis_risk::{
    health::wallet_risk,
    scenario::{ScenarioRequest, ScenarioResponse, simulate},
};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use tracing::error;

/// Lazy-init shared ExecutorContext — keeps one RpcClient pool alive across requests.
fn executor_ctx() -> &'static ExecutorContext {
    static CTX: OnceLock<ExecutorContext> = OnceLock::new();
    CTX.get_or_init(|| {
        let url = std::env::var("RPC_ENDPOINT")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        ExecutorContext::new(&url)
    })
}

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

    state.telegram_chat_ids.insert(wallet.clone(), body.chat_id);

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

// ─── Telegram one-time-code linking ─────────────────────────────────────────
//
// Flow:
//   1. Frontend POSTs /api/wallets/:wallet/telegram/code → stores a 10-min
//      code and returns it + a Telegram deep link.
//   2. User opens the deep link → Telegram sends `/start <code>` to the bot.
//   3. Bot POSTs /api/telegram/redeem {code, chat_id} → server looks up the
//      wallet by code, links chat_id, deletes the code. Idempotent.
//   4. Frontend polls GET /api/wallets/:wallet (existing) to flip the status.

const LINK_CODE_TTL_SECS: i64 = 600; // 10 minutes

#[derive(Serialize)]
pub struct CreateLinkCodeResponse {
    pub code: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub deep_link: String,
}

/// Generate `AEG-XXXX-XXXX` from a v4 uuid's hex digits. ~4B keyspace per
/// 10-min window — collisions are vanishingly unlikely for a single user.
fn generate_link_code() -> String {
    let hex = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
    format!("AEG-{}-{}", &hex[..4], &hex[4..8])
}

pub async fn create_telegram_link_code(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<Json<CreateLinkCodeResponse>, (StatusCode, String)> {
    // Wallet must exist — link_wallet creates the row, so the frontend should
    // call POST /api/wallets/:wallet first. We don't auto-create here because
    // this endpoint is about confirming an existing connection, not registering.
    let exists = sqlx::query("SELECT 1 FROM wallets WHERE pubkey = $1")
        .bind(&wallet)
        .fetch_optional(&state.db_pool)
        .await
        .map_err(internal_error)?;
    if exists.is_none() {
        return Err((StatusCode::NOT_FOUND, "wallet not found".to_string()));
    }

    // Sweep expired codes for this wallet so we never accumulate dead rows.
    let _ = sqlx::query(
        "DELETE FROM telegram_link_codes WHERE wallet_pubkey = $1 AND expires_at < NOW()",
    )
    .bind(&wallet)
    .execute(&state.db_pool)
    .await;

    let code = generate_link_code();
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(LINK_CODE_TTL_SECS);

    sqlx::query(
        "INSERT INTO telegram_link_codes (code, wallet_pubkey, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(&code)
    .bind(&wallet)
    .bind(expires_at)
    .execute(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let bot_username =
        std::env::var("TELEGRAM_BOT_USERNAME").unwrap_or_else(|_| "AegisBot".to_string());
    let deep_link = format!("https://t.me/{}?start={}", bot_username, code);

    Ok(Json(CreateLinkCodeResponse {
        code,
        expires_at,
        deep_link,
    }))
}

#[derive(Deserialize)]
pub struct RedeemLinkCodeBody {
    pub code: String,
    pub chat_id: i64,
}

#[derive(Serialize)]
pub struct RedeemLinkCodeResponse {
    pub wallet: String,
}

/// Bot-only: exchange a one-time code for a wallet binding. On success the
/// wallet's telegram_chat_id is set and the code is consumed.
pub async fn redeem_telegram_link_code(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RedeemLinkCodeBody>,
) -> Result<Json<RedeemLinkCodeResponse>, (StatusCode, String)> {
    let code = body.code.trim().to_uppercase();

    let row = sqlx::query(
        "SELECT wallet_pubkey, expires_at FROM telegram_link_codes WHERE code = $1",
    )
    .bind(&code)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "invalid or already-used code".to_string()));
    };

    let wallet: String = row.try_get("wallet_pubkey").map_err(internal_error)?;
    let expires_at: chrono::DateTime<chrono::Utc> =
        row.try_get("expires_at").map_err(internal_error)?;

    if expires_at < chrono::Utc::now() {
        let _ = sqlx::query("DELETE FROM telegram_link_codes WHERE code = $1")
            .bind(&code)
            .execute(&state.db_pool)
            .await;
        return Err((StatusCode::GONE, "code expired — request a fresh one".to_string()));
    }

    // Spoof guard: if the wallet is already bound to a different chat_id, reject.
    // A re-link to the same chat is idempotent (allowed).
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT telegram_chat_id FROM wallets WHERE pubkey = $1",
    )
    .bind(&wallet)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(internal_error)?
    .flatten();

    if let Some(current) = existing {
        if current != body.chat_id {
            return Err((
                StatusCode::CONFLICT,
                "wallet already linked to a different Telegram chat".to_string(),
            ));
        }
    } else {
        sqlx::query("UPDATE wallets SET telegram_chat_id = $1 WHERE pubkey = $2")
            .bind(body.chat_id)
            .bind(&wallet)
            .execute(&state.db_pool)
            .await
            .map_err(internal_error)?;
        state.telegram_chat_ids.insert(wallet.clone(), body.chat_id);
    }

    // Consume the code so it can't be replayed.
    let _ = sqlx::query("DELETE FROM telegram_link_codes WHERE code = $1")
        .bind(&code)
        .execute(&state.db_pool)
        .await;

    Ok(Json(RedeemLinkCodeResponse { wallet }))
}

/// Detach the Telegram chat from a wallet. Used by the bot's /unlink command.
pub async fn unlink_telegram(
    State(state): State<Arc<AppState>>,
    Path(wallet): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let result = sqlx::query(
        "UPDATE wallets SET telegram_chat_id = NULL WHERE pubkey = $1",
    )
    .bind(&wallet)
    .execute(&state.db_pool)
    .await
    .map_err(internal_error)?;

    if result.rows_affected() == 0 {
        return Err((StatusCode::NOT_FOUND, "wallet not found".to_string()));
    }

    state.telegram_chat_ids.remove(&wallet);
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

// ─── Repay intents ───────────────────────────────────────────────────────────

/// Simulate an unsigned transaction against the RPC to catch program errors
/// at build time (before the user is asked to sign). Returns Err with the
/// program logs if the simulation rejects the tx.
async fn simulate_unsigned_tx(tx_base64: &str) -> Result<(), (StatusCode, String)> {
    let rpc_url = std::env::var("RPC_ENDPOINT")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": [
            tx_base64,
            {
                "encoding": "base64",
                "sigVerify": false,
                "replaceRecentBlockhash": true,
                "commitment": "confirmed"
            }
        ]
    });

    let resp: serde_json::Value = reqwest::Client::new()
        .post(&rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("simulate rpc: {e}")))?
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("simulate decode: {e}")))?;

    // If the RPC itself returned an error envelope, treat it as transport.
    if let Some(err) = resp.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("rpc envelope error")
            .to_string();
        return Err((StatusCode::BAD_GATEWAY, format!("simulate envelope: {msg}")));
    }

    // Inspect the simulation result.
    let value = resp.pointer("/result/value").cloned().unwrap_or_default();
    let logs: Vec<String> = value
        .get("logs")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| l.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let sim_err = value.get("err");
    let is_err = sim_err.map(|v| !v.is_null()).unwrap_or(false);

    if is_err {
        let err_str = sim_err
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown sim error".to_string());

        tracing::warn!(
            "█ [preflight FAILED] err={}\n{}",
            err_str,
            logs.join("\n")
        );

        let body = format!(
            "Preflight simulation rejected the tx.\n\nError: {}\n\nProgram logs:\n{}",
            err_str,
            logs.join("\n")
        );
        return Err((StatusCode::UNPROCESSABLE_ENTITY, body));
    }

    tracing::info!(
        "█ [preflight OK] units_consumed={}",
        value
            .get("unitsConsumed")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    );
    Ok(())
}


#[derive(Deserialize)]
pub struct CreateRepayIntentBody {
    pub wallet: String,
    /// Obligation pubkey (Kamino/Save) or marginfi account pubkey.
    pub position_pubkey: String,
    pub protocol: String,
    pub reserve_or_bank: String,
    pub mint: String,
    /// Native units (pre-decimals). String to dodge JS number precision on u64.
    pub amount_native: String,
    pub rule_id: Option<String>,
}

#[derive(Serialize)]
pub struct CreateRepayIntentResponse {
    pub intent_id: String,
    pub tx_base64: String,
    pub last_valid_block_height: i64,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn create_repay_intent(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateRepayIntentBody>,
) -> Result<Json<CreateRepayIntentResponse>, (StatusCode, String)> {
    let amount: u64 = body
        .amount_native
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "amount_native must be a u64 string".to_string()))?;
    if amount == 0 {
        return Err((StatusCode::BAD_REQUEST, "amount_native must be > 0".to_string()));
    }

    let req = aegis_executor::BuildRepayRequest {
        wallet: body.wallet.clone(),
        obligation_or_account: body.position_pubkey.clone(),
        protocol: body.protocol.clone(),
        reserve_or_bank: body.reserve_or_bank.clone(),
        mint: body.mint.clone(),
        amount_native: amount,
        rule: None,
    };

    let unsigned = aegis_executor::build_repay_tx(executor_ctx(), &req)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("build tx: {e}")))?;

    // Preflight simulation BEFORE the user signs. Uses sigVerify=false +
    // replaceRecentBlockhash=true so we can simulate the unsigned tx as-is.
    // If on-chain rejects the IX (wrong account, missing refresh, etc.), this
    // catches it now and surfaces the program logs to both stdout and the response.
    if let Err((code, msg)) = simulate_unsigned_tx(&unsigned.tx_base64).await {
        return Err((code, msg));
    }

    let expires_at =
        chrono::Utc::now() + chrono::Duration::seconds(executor_ctx().intent_ttl_secs);

    let rule_uuid: Option<uuid::Uuid> = body
        .rule_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    let row = sqlx::query(
        "INSERT INTO execution_intents
         (wallet_pubkey, guard_rule_id, protocol, obligation_or_account, reserve_or_bank, mint, amount_native, unsigned_tx, last_valid_block_height, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING id",
    )
    .bind(&body.wallet)
    .bind(rule_uuid)
    .bind(&body.protocol)
    .bind(&body.position_pubkey)
    .bind(&body.reserve_or_bank)
    .bind(&body.mint)
    .bind(amount as i64)
    .bind(&unsigned.tx_base64)
    .bind(unsigned.last_valid_block_height as i64)
    .bind(expires_at)
    .fetch_one(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let intent_id: uuid::Uuid = row.try_get("id").map_err(internal_error)?;

    Ok(Json(CreateRepayIntentResponse {
        intent_id: intent_id.to_string(),
        tx_base64: unsigned.tx_base64,
        last_valid_block_height: unsigned.last_valid_block_height as i64,
        expires_at,
    }))
}

#[derive(Deserialize)]
pub struct SubmitIntentBody {
    pub signed_tx_base64: String,
}

#[derive(Serialize)]
pub struct SubmitIntentResponse {
    pub signature: String,
}

pub async fn submit_intent(
    State(state): State<Arc<AppState>>,
    Path(intent_id): Path<String>,
    Json(body): Json<SubmitIntentBody>,
) -> Result<Json<SubmitIntentResponse>, (StatusCode, String)> {
    let intent_uuid: uuid::Uuid = uuid::Uuid::parse_str(&intent_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid intent id".to_string()))?;

    let row = sqlx::query(
        "SELECT status, expires_at FROM execution_intents WHERE id = $1",
    )
    .bind(intent_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "intent not found".to_string()));
    };

    let status: String = row.try_get("status").map_err(internal_error)?;
    let expires_at: chrono::DateTime<chrono::Utc> =
        row.try_get("expires_at").map_err(internal_error)?;

    if status != "pending" {
        return Err((
            StatusCode::CONFLICT,
            format!("intent is {status}, cannot resubmit"),
        ));
    }
    if expires_at < chrono::Utc::now() {
        let _ = sqlx::query(
            "UPDATE execution_intents SET status = 'expired', updated_at = NOW() WHERE id = $1",
        )
        .bind(intent_uuid)
        .execute(&state.db_pool)
        .await;
        return Err((
            StatusCode::GONE,
            "intent expired — request a fresh repay".to_string(),
        ));
    }

    // Validate base64 decodes — but ship the original string to the RPC since
    // sendTransaction with `encoding: "base64"` expects the encoded form.
    B64.decode(&body.signed_tx_base64)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid base64: {e}")))?;

    // Call JSON-RPC sendTransaction directly — dodges the type-mismatch dance
    // between solana-sdk's re-exported VersionedTransaction and solana-rpc-client's
    // SerializableTransaction trait bound (different crate versions in our tree).
    let rpc_url = std::env::var("RPC_ENDPOINT")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": [
            body.signed_tx_base64,
            { "encoding": "base64", "skipPreflight": false, "preflightCommitment": "confirmed" }
        ]
    });

    let rpc_resp: serde_json::Value = reqwest::Client::new()
        .post(&rpc_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("rpc send: {e}")))?
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("rpc decode: {e}")))?;

    if let Some(err) = rpc_resp.get("error") {
        // Build a rich error: top-level message + program msg!() logs from preflight.
        // The logs are what tell us which on-chain check actually rejected the tx.
        let top_msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown rpc error")
            .to_string();

        let logs: Vec<String> = err
            .pointer("/data/logs")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Also surface preflight return-data, which sometimes carries the Anchor error.
        let preflight_err = err
            .pointer("/data/err")
            .map(|v| v.to_string())
            .unwrap_or_default();

        tracing::warn!(
            "[intent {}] tx rejected: {} | err={} | logs:\n{}",
            intent_uuid,
            top_msg,
            preflight_err,
            logs.join("\n")
        );

        let combined = if logs.is_empty() {
            top_msg.clone()
        } else {
            format!("{}\n\nProgram logs:\n{}", top_msg, logs.join("\n"))
        };

        let pool = state.db_pool.clone();
        let err_msg = combined.clone();
        tokio::spawn(async move {
            let _ = sqlx::query(
                "UPDATE execution_intents SET status = 'cancelled', error = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(err_msg)
            .bind(intent_uuid)
            .execute(&pool)
            .await;
        });
        return Err((StatusCode::BAD_GATEWAY, combined));
    }

    let sig_str = rpc_resp
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_GATEWAY, "rpc returned no signature".to_string()))?
        .to_string();

    sqlx::query(
        "UPDATE execution_intents SET status = 'submitted', signature = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(&sig_str)
    .bind(intent_uuid)
    .execute(&state.db_pool)
    .await
    .map_err(internal_error)?;

    Ok(Json(SubmitIntentResponse { signature: sig_str }))
}

#[derive(Serialize)]
pub struct IntentResponse {
    pub id: String,
    pub status: String,
    pub signature: Option<String>,
    pub error: Option<String>,
    pub protocol: String,
    pub mint: String,
    pub amount_native: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_intent(
    State(state): State<Arc<AppState>>,
    Path(intent_id): Path<String>,
) -> Result<Json<IntentResponse>, (StatusCode, String)> {
    let intent_uuid: uuid::Uuid = uuid::Uuid::parse_str(&intent_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "invalid intent id".to_string()))?;

    let row = sqlx::query(
        "SELECT id, status, signature, error, protocol, mint, amount_native, created_at, expires_at
         FROM execution_intents WHERE id = $1",
    )
    .bind(intent_uuid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(internal_error)?;

    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "intent not found".to_string()));
    };

    Ok(Json(IntentResponse {
        id: row
            .try_get::<uuid::Uuid, _>("id")
            .map_err(internal_error)?
            .to_string(),
        status: row.try_get("status").map_err(internal_error)?,
        signature: row.try_get("signature").ok(),
        error: row.try_get("error").ok(),
        protocol: row.try_get("protocol").map_err(internal_error)?,
        mint: row.try_get("mint").map_err(internal_error)?,
        amount_native: row.try_get("amount_native").map_err(internal_error)?,
        created_at: row.try_get("created_at").map_err(internal_error)?,
        expires_at: row.try_get("expires_at").map_err(internal_error)?,
    }))
}
