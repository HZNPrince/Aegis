//! Alert delivery — the contract any notification channel must satisfy.
//!
//! The engine doesn't know or care where an alert goes. It hands every
//! fired alert to each registered `Dispatcher`. A dispatcher that errors
//! out gets its failure logged; the rest still run.
//!
//! Adding a new channel (Telegram, Slack, webhook) = one new type that
//! impls this trait. No engine changes required.

use aegis_core::types::{AlertRecord, AlertSeverity};
use async_trait::async_trait;
use tracing::{info, warn};

#[async_trait]
pub trait Dispatcher: Send + Sync {
    /// Human-readable channel name for logs ("log", "telegram", ...).
    fn name(&self) -> &'static str;

    /// Deliver one alert. Errors are logged by the caller — the engine
    /// should never crash because a channel is down.
    async fn send(&self, alert: &AlertRecord) -> anyhow::Result<()>;
}

/// The default, always-on dispatcher. Prints a structured log line so
/// operators and the Loom demo can see alerts firing in real time.
pub struct LogDispatcher;

#[async_trait]
impl Dispatcher for LogDispatcher {
    fn name(&self) -> &'static str {
        "log"
    }

    async fn send(&self, alert: &AlertRecord) -> anyhow::Result<()> {
        info!(
            "🚨 [alert] {:?} wallet={} health={:.1} ltv={:.3} — {}",
            alert.severity, alert.wallet, alert.health_score, alert.ltv, alert.title,
        );
        for action in &alert.suggested_actions {
            info!("         ↳ suggest: {}", action);
        }
        Ok(())
    }
}

/// Sends alerts to a single Telegram chat via the Bot API. For MVP this
/// is one global admin chat — every alert for every monitored wallet lands
/// there. Multi-user fan-out becomes a `wallets.telegram_chat_id` column
/// later without touching this dispatcher.
pub struct TelegramDispatcher {
    http: reqwest::Client,
    token: String,
    chat_id: String,
}

impl TelegramDispatcher {
    /// Returns `None` when either env var is missing so the server can
    /// register it conditionally without panicking in local dev.
    pub fn from_env() -> Option<Self> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN").ok().filter(|v| !v.is_empty())?;
        let chat_id = std::env::var("TG_CHAT_ID")
            .ok()
            .or_else(|| std::env::var("TELEGRAM_ADMIN_CHAT_ID").ok())
            .filter(|v| !v.is_empty())?;
        Some(Self {
            http: reqwest::Client::new(),
            token,
            chat_id,
        })
    }
}

#[async_trait]
impl Dispatcher for TelegramDispatcher {
    fn name(&self) -> &'static str {
        "telegram"
    }

    async fn send(&self, alert: &AlertRecord) -> anyhow::Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let text = format_alert_markdown(alert);

        let resp = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": text,
                "parse_mode": "Markdown",
                "disable_web_page_preview": true,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("telegram sendMessage {}: {}", status, body);
        }
        Ok(())
    }
}

fn format_alert_markdown(alert: &AlertRecord) -> String {
    let emoji = match alert.severity {
        AlertSeverity::Critical => "🚨",
        AlertSeverity::Warning => "⚠️",
        AlertSeverity::Info => "ℹ️",
    };
    let short_wallet = if alert.wallet.len() > 12 {
        format!("{}…{}", &alert.wallet[..4], &alert.wallet[alert.wallet.len() - 4..])
    } else {
        alert.wallet.clone()
    };

    let mut out = format!(
        "{} *{:?}* — {}\n\n*Wallet:* `{}`\n*Health:* {:.1}/100   *LTV:* {:.3}\n\n{}",
        emoji, alert.severity, alert.title, short_wallet, alert.health_score, alert.ltv, alert.message,
    );
    if !alert.suggested_actions.is_empty() {
        out.push_str("\n\n*Suggested:*");
        for a in &alert.suggested_actions {
            out.push_str(&format!("\n• {}", a));
        }
    }
    out
}

/// Fan one alert out to every dispatcher. Called by the engine after the
/// alert is persisted. A failure in one channel must not block the others,
/// so we log and continue.
pub async fn broadcast(dispatchers: &[std::sync::Arc<dyn Dispatcher>], alert: &AlertRecord) {
    for d in dispatchers {
        if let Err(e) = d.send(alert).await {
            warn!("dispatcher '{}' failed: {}", d.name(), e);
        }
    }
}
