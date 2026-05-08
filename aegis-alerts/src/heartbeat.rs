//! Heartbeat dispatcher — sends periodic status updates to Telegram-linked wallets.
//! Distinct from alert engine: fires on a fixed cadence (default 4h), not on risk changes.
//! Provides "you're still being watched" status pulse with current prices, positions, and health.
//! Used by: aegis-server, which spawns this at startup if TELEGRAM_BOT_TOKEN is set.

use std::{sync::Arc, time::Duration};

use aegis_core::{
    state::AppState,
    types::{PositionSide, WalletRisk},
};
use aegis_risk::health::wallet_risk;
use tracing::{info, warn};

/// Spawn the heartbeat loop. `interval_hours == 0` is a no-op.
pub async fn start_heartbeat_loop(state: Arc<AppState>, interval_hours: u64) {
    if interval_hours == 0 {
        info!("[heartbeat] disabled (HEARTBEAT_INTERVAL_HOURS=0)");
        return;
    }

    let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") else {
        info!("[heartbeat] TELEGRAM_BOT_TOKEN unset — heartbeat disabled");
        return;
    };
    if token.is_empty() {
        info!("[heartbeat] TELEGRAM_BOT_TOKEN empty — heartbeat disabled");
        return;
    }

    let http = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let interval = Duration::from_secs(interval_hours * 3_600);

    info!("[heartbeat] online — interval = {}h", interval_hours);

    // Skip the first immediate tick — wait one full interval before the first
    // pulse so a server restart doesn't spam users.
    tokio::time::sleep(interval).await;

    loop {
        let chats: Vec<(String, i64)> = state
            .telegram_chat_ids
            .iter()
            .map(|e| (e.key().clone(), *e.value()))
            .collect();

        info!("[heartbeat] sending digest to {} wallet(s)", chats.len());

        for (wallet, chat_id) in chats {
            let risk = wallet_risk(&state, &wallet);
            // Skip wallets with nothing to report so we don't spam empty digests.
            if risk.total_collateral_usd <= 0.0 && risk.total_debt_usd <= 0.0 {
                continue;
            }

            let text = format_heartbeat_message(&wallet, &risk, state.as_ref());
            let body = serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "parse_mode": "Markdown",
                "disable_web_page_preview": true,
            });

            if let Err(e) = http.post(&url).json(&body).send().await {
                warn!("[heartbeat] send failed for {}: {}", short(&wallet), e);
            }
        }

        tokio::time::sleep(interval).await;
    }
}

fn format_heartbeat_message(wallet: &str, risk: &WalletRisk, state: &AppState) -> String {
    let mut out = String::new();
    let emoji = if risk.health_score >= 65.0 {
        "💚"
    } else if risk.health_score >= 40.0 {
        "⚠️"
    } else {
        "🔴"
    };

    out.push_str(&format!("🛡️ *Aegis pulse* — `{}`\n\n", short(wallet)));
    out.push_str(&format!(
        "{} *Health:* {:.0}/100   *LTV:* {:.1}%   *Buffer:* ${:.2}\n\n",
        emoji,
        risk.health_score,
        risk.ltv * 100.0,
        risk.liquidation_buffer_usd
    ));

    if !risk.positions.is_empty() {
        out.push_str("*Positions:*\n");
        for pos in &risk.positions {
            for leg in &pos.legs {
                let side = match leg.side {
                    PositionSide::Collateral => "C",
                    PositionSide::Borrow => "B",
                };
                out.push_str(&format!(
                    "• [{}] {} · {} {:.4} (≈${:.2})\n",
                    side, pos.protocol, leg.asset_symbol, leg.amount_ui, leg.value_usd,
                ));
            }
        }
        out.push('\n');
    }

    // Snapshot prices for assets the user actually holds, with 24h delta when known.
    let mut seen_mints: Vec<String> = Vec::new();
    for pos in &risk.positions {
        for leg in &pos.legs {
            if !leg.asset_mint.is_empty() && !seen_mints.contains(&leg.asset_mint) {
                seen_mints.push(leg.asset_mint.clone());
            }
        }
    }
    if !seen_mints.is_empty() {
        out.push_str("*Prices:*\n");
        for mint in &seen_mints {
            let symbol = state
                .token_mints
                .get(mint)
                .map(|s| s.clone())
                .unwrap_or_else(|| short(mint));
            let price = state.token_prices.get(mint).map(|p| *p).unwrap_or(0.0);
            let change = state.token_price_changes.get(mint).map(|c| *c);
            let change_str = match change {
                Some(c) if c >= 0.0 => format!(" (+{:.2}% 24h)", c),
                Some(c) => format!(" ({:.2}% 24h)", c),
                None => String::new(),
            };
            out.push_str(&format!("• {} — ${:.4}{}\n", symbol, price, change_str));
        }
    }

    out.push_str("\n_Aegis is watching. You'll hear from me when it matters._");
    out
}

fn short(s: &str) -> String {
    if s.len() > 12 {
        format!("{}…{}", &s[..4], &s[s.len() - 4..])
    } else {
        s.to_string()
    }
}
