//! Aegis Telegram bot.
//!
//! Commands:
//!   /start          — Welcome + usage
//!   /link <pubkey>  — Link a Solana wallet to this chat
//!   /status         — Current wallet health + positions + AI analysis

use teloxide::{prelude::*, utils::command::BotCommands};
use tracing::info;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Aegis bot commands:")]
enum Command {
    #[command(description = "Show usage instructions")]
    Start,
    #[command(description = "Link your Solana wallet: /link <wallet_pubkey>")]
    Link(String),
    #[command(rename = "bot_status", description = "Show your current positions and health score")]
    Status,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    info!("╔════════════════════════════════╗");
    info!("║  Aegis Alert Bot  — starting   ║");
    info!("╚════════════════════════════════╝");

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set");
    let bot = Bot::new(token);

    Command::repl(bot, handle_command).await;
}

async fn handle_command(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Start => {
            bot.send_message(
                msg.chat.id,
                "👋 *Welcome to Aegis!*\n\n\
                 Aegis monitors your Solana lending positions on Kamino, Save, and Marginfi \
                 and alerts you when your health drops or liquidation risk increases\\.\n\n\
                 *Commands:*\n\
                 `/link <wallet_pubkey>` — Connect your wallet\n\
                 `/status` — Check current positions and health\n\n\
                 Connect your wallet in the Aegis dashboard first, then run `/link`\\.",
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }

        Command::Link(wallet_pubkey) => {
            let wallet_pubkey = wallet_pubkey.trim().to_string();

            if wallet_pubkey.len() < 32 || wallet_pubkey.len() > 44 {
                bot.send_message(
                    msg.chat.id,
                    "❌ That doesn't look like a valid Solana wallet address. Please copy your full pubkey from the Aegis dashboard.",
                )
                .await?;
                return Ok(());
            }

            let chat_id = msg.chat.id.0;
            match link_wallet_to_telegram(&wallet_pubkey, chat_id).await {
                Ok(()) => {
                    let short = short_wallet(&wallet_pubkey);
                    // Fetch health to confirm link worked and show current state.
                    match fetch_health(&wallet_pubkey).await {
                        Ok(Some(health)) => {
                            let text = format_link_success(&short, &health);
                            bot.send_message(msg.chat.id, text)
                                .parse_mode(teloxide::types::ParseMode::Markdown)
                                .await?;
                        }
                        _ => {
                            bot.send_message(
                                msg.chat.id,
                                format!(
                                    "✅ *Linked!* Wallet `{}` is now connected.\n\nSend /status at any time to check your positions.",
                                    short
                                ),
                            )
                            .parse_mode(teloxide::types::ParseMode::Markdown)
                            .await?;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("link failed for {}: {}", wallet_pubkey, e);
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "❌ Couldn't link wallet: {}\n\nMake sure you've connected this wallet in the Aegis dashboard first.",
                            e
                        ),
                    )
                    .await?;
                }
            }
        }

        Command::Status => {
            let chat_id = msg.chat.id.0;

            // Resolve chat_id → wallet pubkey.
            let wallet = match lookup_wallet_by_chat(chat_id).await {
                Ok(w) => w,
                Err(_) => {
                    bot.send_message(
                        msg.chat.id,
                        "⚠️ No wallet linked to this chat yet.\n\nUse `/link <wallet_pubkey>` to connect your Solana wallet.",
                    )
                    .await?;
                    return Ok(());
                }
            };

            // Show a "loading" indicator first since the health call may take a moment.
            bot.send_message(msg.chat.id, "⏳ Fetching your positions…").await?;

            match fetch_health(&wallet).await {
                Ok(Some(health)) => {
                    let text = format_status_message(&wallet, &health);
                    bot.send_message(msg.chat.id, text)
                        .parse_mode(teloxide::types::ParseMode::Markdown)
                        .await?;
                }
                _ => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "⚠️ Wallet `{}` is linked but has no tracked positions yet.\n\nMake sure the Aegis server is running and your wallet is monitored.",
                            short_wallet(&wallet)
                        ),
                    )
                    .parse_mode(teloxide::types::ParseMode::Markdown)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

// ─── API helpers ────────────────────────────────────────────────────────────

fn api_base() -> String {
    std::env::var("AEGIS_API_URL").unwrap_or_else(|_| "http://localhost:7878".to_string())
}

fn short_wallet(w: &str) -> String {
    if w.len() > 12 {
        format!("{}…{}", &w[..4], &w[w.len() - 4..])
    } else {
        w.to_string()
    }
}

async fn link_wallet_to_telegram(wallet: &str, chat_id: i64) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    // Register wallet first (idempotent).
    let _ = client
        .post(format!("{}/api/wallets/{}", api_base(), wallet))
        .send()
        .await?;

    // Link the chat ID.
    let resp = client
        .patch(format!("{}/api/wallets/{}/telegram", api_base(), wallet))
        .json(&serde_json::json!({ "chat_id": chat_id }))
        .send()
        .await?;

    let status = resp.status();
    if status.is_success() || status == reqwest::StatusCode::NO_CONTENT {
        return Ok(());
    }
    anyhow::bail!("{}: {}", status, resp.text().await.unwrap_or_default())
}

/// Resolve a Telegram chat_id to the linked wallet pubkey.
async fn lookup_wallet_by_chat(chat_id: i64) -> anyhow::Result<String> {
    let url = format!("{}/api/wallets/by-chat/{}", api_base(), chat_id);
    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        anyhow::bail!("not found");
    }
    let json: serde_json::Value = resp.json().await?;
    json["wallet"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing wallet field"))
}

/// Full WalletRisk response from the health endpoint.
#[derive(Debug, serde::Deserialize)]
struct WalletHealth {
    health_score: f64,
    ltv: f64,
    liquidation_threshold: f64,
    liquidation_buffer_usd: f64,
    total_collateral_usd: f64,
    total_debt_usd: f64,
    positions: Vec<PositionData>,
}

#[derive(Debug, serde::Deserialize)]
struct PositionData {
    protocol: String,
    collateral_usd: f64,
    debt_usd: f64,
    legs: Vec<LegData>,
}

#[derive(Debug, serde::Deserialize)]
struct LegData {
    asset_symbol: String,
    side: String,
    amount_ui: f64,
    value_usd: f64,
}

async fn fetch_health(wallet: &str) -> anyhow::Result<Option<WalletHealth>> {
    let url = format!("{}/api/health/{}", api_base(), wallet);
    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let health: WalletHealth = resp.json().await?;
    if health.total_collateral_usd <= 0.0 && health.total_debt_usd <= 0.0 {
        return Ok(None);
    }
    Ok(Some(health))
}

// ─── Message formatters ──────────────────────────────────────────────────────

fn format_link_success(short: &str, h: &WalletHealth) -> String {
    let mut lines = vec![
        format!("✅ *Linked!* Wallet `{}` is now connected to Aegis.", short),
        String::new(),
    ];
    lines.extend(position_lines(h));
    lines.push(String::new());
    lines.push(health_summary_line(h));
    lines.push(String::new());
    lines.push("You'll receive alerts here for health drops, LTV changes, and liquidation risk events.".to_string());
    lines.join("\n")
}

fn format_status_message(wallet: &str, h: &WalletHealth) -> String {
    let short = short_wallet(wallet);
    let mut lines = vec![
        "🛡️ *Aegis Status*".to_string(),
        String::new(),
        format!("📍 *Wallet:* `{}`", short),
        String::new(),
        "📊 *Positions:*".to_string(),
    ];

    lines.extend(position_lines(h));
    lines.push(String::new());
    lines.push(health_summary_line(h));
    lines.push(String::new());

    // AI-style narrative analysis based on the numbers.
    lines.push("🤖 *Analysis:*".to_string());
    lines.push(generate_analysis(h));

    // Actionable suggestions.
    let suggestions = generate_suggestions(h);
    if !suggestions.is_empty() {
        lines.push(String::new());
        lines.push("💡 *Suggested actions:*".to_string());
        for s in suggestions {
            lines.push(format!("• {}", s));
        }
    }

    lines.join("\n")
}

fn position_lines(h: &WalletHealth) -> Vec<String> {
    let mut out = Vec::new();
    for pos in &h.positions {
        if !pos.legs.is_empty() {
            for leg in &pos.legs {
                let usd = if leg.value_usd > 0.0 {
                    format!("*${:.2}*", leg.value_usd)
                } else {
                    format!("{:.4} {}", leg.amount_ui, leg.asset_symbol)
                };
                out.push(format!("• {} · {} ({}) — {}", pos.protocol, leg.asset_symbol, leg.side, usd));
            }
        } else {
            if pos.collateral_usd > 0.0 {
                out.push(format!("• {} · (Collateral) — *${:.2}*", pos.protocol, pos.collateral_usd));
            }
            if pos.debt_usd > 0.0 {
                out.push(format!("• {} · (Borrow) — *${:.2}*", pos.protocol, pos.debt_usd));
            }
        }
    }
    if out.is_empty() {
        out.push("  _No open positions_".to_string());
    }
    out
}

fn health_summary_line(h: &WalletHealth) -> String {
    let emoji = if h.health_score >= 65.0 { "💚" } else if h.health_score >= 40.0 { "⚠️" } else { "🔴" };
    format!(
        "{} *Health:* {:.0}/100   *LTV:* {:.1}%   *Buffer:* ${:.2}   *Threshold:* {:.0}%",
        emoji,
        h.health_score,
        h.ltv * 100.0,
        h.liquidation_buffer_usd,
        h.liquidation_threshold * 100.0,
    )
}

fn generate_analysis(h: &WalletHealth) -> String {
    let ltv_pct = h.ltv * 100.0;
    let threshold_pct = h.liquidation_threshold * 100.0;
    let margin_pct = (h.liquidation_threshold - h.ltv) * 100.0;

    // How much collateral can fall before liquidation?
    let collateral_drop_to_liq = if h.total_collateral_usd > 0.0 && h.ltv > 0.0 {
        let liq_collateral = h.total_debt_usd / h.liquidation_threshold;
        ((h.total_collateral_usd - liq_collateral) / h.total_collateral_usd * 100.0).max(0.0)
    } else {
        0.0
    };

    if h.health_score >= 65.0 {
        format!(
            "Your position is healthy with {:.1}% LTV against a {:.0}% liquidation threshold — \
             {:.1}% margin remaining. Collateral would need to fall {:.0}% from here to trigger \
             liquidation.",
            ltv_pct, threshold_pct, margin_pct, collateral_drop_to_liq
        )
    } else if h.health_score >= 40.0 {
        format!(
            "Your position is under moderate stress. LTV is {:.1}% with {:.0}% margin before \
             the {:.0}% liquidation threshold. A {:.0}% drop in collateral value would put you at \
             risk — consider reducing debt or adding collateral.",
            ltv_pct, margin_pct, threshold_pct, collateral_drop_to_liq
        )
    } else {
        format!(
            "⚠️ High liquidation risk. LTV is {:.1}%, only {:.1}% from the {:.0}% threshold. \
             You have ${:.2} buffer left — act now by repaying debt or adding collateral. \
             Any adverse price move could trigger liquidation.",
            ltv_pct, margin_pct, threshold_pct, h.liquidation_buffer_usd
        )
    }
}

fn generate_suggestions(h: &WalletHealth) -> Vec<String> {
    let mut out = Vec::new();
    let margin = h.liquidation_threshold - h.ltv;

    if h.health_score < 40.0 {
        out.push("Repay debt on the highest-LTV protocol immediately.".to_string());
        out.push(format!(
            "You need to reduce debt by ${:.2} to reach a safer 60% LTV.",
            (h.total_debt_usd - h.total_collateral_usd * 0.60).max(0.0)
        ));
    } else if margin < 0.15 {
        out.push("Add collateral or partially repay debt — margin is thin.".to_string());
        out.push("Set a guard rule to auto-alert if health drops below 40.".to_string());
    } else if h.health_score >= 65.0 {
        out.push("Position looks healthy. Keep monitoring price movements.".to_string());
        if h.total_debt_usd > 0.0 {
            out.push("Consider setting a health guard rule below 50 for peace of mind.".to_string());
        }
    }

    out
}
