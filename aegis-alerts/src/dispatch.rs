//! Alert delivery — the contract any notification channel must satisfy.
//!
//! The engine doesn't know or care where an alert goes. It hands every
//! fired alert to each registered `Dispatcher`. A dispatcher that errors
//! out gets its failure logged; the rest still run.
//!
//! Adding a new channel (Telegram, Slack, webhook) = one new type that
//! impls this trait. No engine changes required.

use aegis_core::types::AlertRecord;
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
