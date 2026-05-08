//! Aegis Telegram bot — `@aegis_alerter_bot`.
//!
//! Commands:
//!   /start [code]   — Welcome + usage. With a code payload (deep-link from
//!                     the dashboard), redeems it to link the wallet.
//!   /help           — Compact reference for every command
//!   /link <pubkey>  — Link a Solana wallet to this chat (legacy manual flow)
//!   /unlink         — Detach this chat from its wallet
//!   /status         — Wallet health + positions + AI analysis (verbose)
//!   /positions      — Per-leg position breakdown (compact)
//!   /prices         — Live prices + 24h Δ for assets you hold
//!   /rules          — Active guard rules for your wallet

use teloxide::{prelude::*, utils::command::BotCommands};
use tracing::info;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Aegis bot commands:")]
enum Command {
    #[command(description = "Show usage instructions, or redeem a link code from the dashboard")]
    Start(String),
    #[command(description = "Show every command and what it does")]
    Help,
    #[command(description = "Link your Solana wallet: /link <wallet_pubkey>")]
    Link(String),
    #[command(description = "Detach this chat from its linked wallet")]
    Unlink,
    #[command(rename = "bot_status", description = "Show your current positions and health score")]
    Status,
    #[command(description = "Per-leg position breakdown")]
    Positions,
    #[command(description = "Live prices for assets in your positions")]
    Prices,
    #[command(description = "List your active guard rules")]
    Rules,
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
        Command::Start(payload) => {
            let payload = payload.trim();
            if payload.is_empty() {
                bot.send_message(
                    msg.chat.id,
                    "👋 *Welcome to Aegis\\!*\n\n\
                     I'm `@aegis_alerter_bot` — I watch your Solana lending positions on Kamino, Save, and Marginfi \
                     and ping you when health drops or liquidation gets close\\.\n\n\
                     *To connect:*\n\
                     1\\. Open the Aegis dashboard, go to *Telegram*\n\
                     2\\. Tap *Open @aegis\\_alerter\\_bot* — it'll send you back here with a code\n\n\
                     Or manually: `/link <wallet_pubkey>`\n\n\
                     Send `/help` any time to see every command\\.",
                )
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
                return Ok(());
            }

            // Deep-link payload — redeem it for a wallet binding.
            let chat_id = msg.chat.id.0;
            match redeem_link_code(payload, chat_id).await {
                Ok(wallet) => {
                    let short = short_wallet(&wallet);
                    match fetch_health(&wallet).await {
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
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "❌ Couldn't redeem that link code: {}\n\nGenerate a fresh one from the Aegis dashboard.",
                            e
                        ),
                    )
                    .await?;
                }
            }
        }

        Command::Help => {
            bot.send_message(
                msg.chat.id,
                "🛡️ *Aegis bot — command reference*\n\n\
                 *Onboarding*\n\
                 `/start` — welcome + how to link\n\
                 `/start <code>` — redeem a link code from the dashboard \\(auto\\-deep\\-linked from the *Telegram* page\\)\n\
                 `/link <wallet_pubkey>` — manual fallback if you don't have a code\n\
                 `/unlink` — detach this chat from its wallet\n\n\
                 *Live data*\n\
                 `/bot_status` — weighted health, totals, AI risk read\n\
                 `/positions` — per\\-leg breakdown across Kamino, Save, Marginfi\n\
                 `/prices` — quotes \\+ 24h Δ for assets you hold\n\
                 `/rules` — your active guardrails\n\n\
                 *How alerts work*\n\
                 You'll get a ping when a guardrail fires \\(e\\.g\\. _Health below 60_\\) or when liquidation risk spikes\\. Critical alerts include inline *Repay* / *Mute* buttons\\.\n\n\
                 Open the dashboard at the link in your bio to set new guardrails or run a what\\-if scenario\\.",
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

        Command::Unlink => {
            let chat_id = msg.chat.id.0;
            match lookup_wallet_by_chat(chat_id).await {
                Ok(wallet) => match unlink_wallet(&wallet).await {
                    Ok(()) => {
                        bot.send_message(
                            msg.chat.id,
                            format!(
                                "🔓 Unlinked `{}`. You will not receive Aegis alerts here anymore.\n\nUse /link to re-attach.",
                                short_wallet(&wallet)
                            ),
                        )
                        .parse_mode(teloxide::types::ParseMode::Markdown)
                        .await?;
                    }
                    Err(e) => {
                        bot.send_message(msg.chat.id, format!("❌ Unlink failed: {e}"))
                            .await?;
                    }
                },
                Err(_) => {
                    bot.send_message(msg.chat.id, "ℹ️ No wallet linked to this chat.")
                        .await?;
                }
            }
        }

        Command::Positions => {
            let wallet = match resolve_wallet(msg.chat.id.0).await {
                Ok(w) => w,
                Err(text) => {
                    bot.send_message(msg.chat.id, text).await?;
                    return Ok(());
                }
            };
            match fetch_health(&wallet).await {
                Ok(Some(health)) => {
                    let mut lines = vec![
                        "📊 *Positions*".to_string(),
                        format!("Wallet `{}`", short_wallet(&wallet)),
                        String::new(),
                    ];
                    lines.extend(position_lines(&health));
                    lines.push(String::new());
                    lines.push(health_summary_line(&health));
                    bot.send_message(msg.chat.id, lines.join("\n"))
                        .parse_mode(teloxide::types::ParseMode::Markdown)
                        .await?;
                }
                _ => {
                    bot.send_message(msg.chat.id, "ℹ️ No tracked positions for this wallet.")
                        .await?;
                }
            }
        }

        Command::Prices => {
            let wallet = match resolve_wallet(msg.chat.id.0).await {
                Ok(w) => w,
                Err(text) => {
                    bot.send_message(msg.chat.id, text).await?;
                    return Ok(());
                }
            };

            let health = match fetch_health(&wallet).await {
                Ok(Some(h)) => h,
                _ => {
                    bot.send_message(msg.chat.id, "ℹ️ No positions to price.").await?;
                    return Ok(());
                }
            };

            // Collect unique mints from the user's positions.
            let mut mints: Vec<(String, String)> = Vec::new();
            for pos in &health.positions {
                for leg in &pos.legs {
                    if !leg.asset_mint.is_empty()
                        && !mints.iter().any(|(m, _)| *m == leg.asset_mint)
                    {
                        mints.push((leg.asset_mint.clone(), leg.asset_symbol.clone()));
                    }
                }
            }

            let ticker = fetch_ticker().await.unwrap_or_default();
            let mut lines = vec!["💱 *Prices* (24h)".to_string(), String::new()];
            if mints.is_empty() {
                lines.push("_No mints found in your positions._".to_string());
            } else {
                for (mint, symbol) in mints {
                    if let Some(t) = ticker.get(&mint) {
                        let arrow = match t.change_24h {
                            Some(c) if c >= 0.0 => format!(" 🟢 +{:.2}%", c),
                            Some(c) => format!(" 🔴 {:.2}%", c),
                            None => String::new(),
                        };
                        lines.push(format!("• *{}* — ${:.4}{}", symbol, t.price, arrow));
                    } else {
                        lines.push(format!("• *{}* — _no price_", symbol));
                    }
                }
            }
            bot.send_message(msg.chat.id, lines.join("\n"))
                .parse_mode(teloxide::types::ParseMode::Markdown)
                .await?;
        }

        Command::Rules => {
            let wallet = match resolve_wallet(msg.chat.id.0).await {
                Ok(w) => w,
                Err(text) => {
                    bot.send_message(msg.chat.id, text).await?;
                    return Ok(());
                }
            };

            match fetch_guard_rules(&wallet).await {
                Ok(rules) if !rules.is_empty() => {
                    let mut lines = vec![
                        format!("🛡️ *Guard Rules* ({})", rules.len()),
                        format!("Wallet `{}`", short_wallet(&wallet)),
                        String::new(),
                    ];
                    for r in rules.iter().filter(|r| r.is_active) {
                        lines.push(format_rule_line(r));
                    }
                    let inactive: Vec<_> = rules.iter().filter(|r| !r.is_active).collect();
                    if !inactive.is_empty() {
                        lines.push(String::new());
                        lines.push(format!("_{} inactive rule(s) hidden_", inactive.len()));
                    }
                    bot.send_message(msg.chat.id, lines.join("\n"))
                        .parse_mode(teloxide::types::ParseMode::Markdown)
                        .await?;
                }
                _ => {
                    bot.send_message(
                        msg.chat.id,
                        "📭 No guard rules yet. Open the Aegis dashboard to create one.",
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

/// Exchange a one-time link code (from the dashboard's deep link) for a
/// wallet binding. The server sets telegram_chat_id and consumes the code.
async fn redeem_link_code(code: &str, chat_id: i64) -> anyhow::Result<String> {
    let resp = reqwest::Client::new()
        .post(format!("{}/api/telegram/redeem", api_base()))
        .json(&serde_json::json!({ "code": code, "chat_id": chat_id }))
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        anyhow::bail!("{}: {}", status, resp.text().await.unwrap_or_default());
    }
    let json: serde_json::Value = resp.json().await?;
    json["wallet"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("missing wallet field"))
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
    #[serde(default)]
    asset_mint: String,
}

#[derive(Debug, serde::Deserialize)]
struct TickerEntry {
    price: f64,
    change_24h: Option<f64>,
}

#[derive(Debug, serde::Deserialize)]
struct GuardRuleData {
    protocol: Option<String>,
    trigger_kind: String,
    trigger_value: f64,
    action_kind: String,
    cooldown_seconds: i64,
    is_active: bool,
}

async fn unlink_wallet(wallet: &str) -> anyhow::Result<()> {
    let resp = reqwest::Client::new()
        .delete(format!("{}/api/wallets/{}/telegram", api_base(), wallet))
        .send()
        .await?;
    if resp.status().is_success() || resp.status() == reqwest::StatusCode::NO_CONTENT {
        return Ok(());
    }
    anyhow::bail!("{}: {}", resp.status(), resp.text().await.unwrap_or_default())
}

async fn fetch_ticker() -> anyhow::Result<std::collections::HashMap<String, TickerEntry>> {
    let url = format!("{}/api/ticker", api_base());
    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        anyhow::bail!("ticker fetch failed: {}", resp.status());
    }
    Ok(resp.json().await?)
}

async fn fetch_guard_rules(wallet: &str) -> anyhow::Result<Vec<GuardRuleData>> {
    let url = format!("{}/api/wallets/{}/guard-rules", api_base(), wallet);
    let resp = reqwest::get(&url).await?;
    if !resp.status().is_success() {
        anyhow::bail!("guard-rules fetch failed: {}", resp.status());
    }
    Ok(resp.json().await?)
}

/// Resolve a chat_id → linked wallet pubkey, or return a user-facing error message.
async fn resolve_wallet(chat_id: i64) -> Result<String, String> {
    lookup_wallet_by_chat(chat_id)
        .await
        .map_err(|_| "⚠️ No wallet linked to this chat. Use /link <wallet_pubkey> first.".to_string())
}

fn format_rule_line(r: &GuardRuleData) -> String {
    let trigger = match r.trigger_kind.as_str() {
        "health_below" => format!("Health < {:.0}", r.trigger_value),
        "ltv_above" => format!("LTV > {:.0}%", r.trigger_value * 100.0),
        "debt_above_usd" => format!("Debt > ${:.0}", r.trigger_value),
        "health_dropped" => format!("Health drops {:.0}%", r.trigger_value * 100.0),
        other => other.to_string(),
    };
    let action = match r.action_kind.as_str() {
        "notify_only" => "Notify",
        "add_collateral" => "Add collateral",
        "repay_debt" => "Repay debt",
        "deleverage" => "Deleverage",
        other => other,
    };
    let proto = r.protocol.as_deref().unwrap_or("All");
    let cool = if r.cooldown_seconds >= 3600 {
        format!("{}h cooldown", r.cooldown_seconds / 3600)
    } else {
        format!("{}m cooldown", r.cooldown_seconds / 60)
    };
    format!("• [{}] {} → {} · {}", proto, trigger, action, cool)
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
