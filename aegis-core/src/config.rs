//! Configuration loader — reads environment variables and initializes AegisConfig.
//! Used by aegis-server at startup to configure gRPC, RPC, database, and alert thresholds.
//!
//! Override defaults by setting environment variables; unset variables fall back to sensible defaults.

use std::env;

/// Application configuration loaded from environment variables.
/// Initializes all runtime parameters: endpoints, thresholds, intervals, and credentials.
pub struct AegisConfig {
    /// ParaFi Yellowstone gRPC endpoint
    pub grpc_endpoint: String,
    /// Standard RPC endpoint (fallback for one-off reads)
    pub rpc_endpoint: String,
    /// Postgres connection string
    pub database_url: String,
    /// Telegram bot token from @BotFather
    pub telegram_bot_token: String,
    /// LLM API key
    pub llm_api_key: String,
    /// Health score below this -> Fire alert
    pub alert_threshold: f64,
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Heartbeat digest interval in hours. 0 disables the heartbeat dispatcher.
    pub heartbeat_interval_hours: u64,
}

impl AegisConfig {
    /// Load configuration from environment variables with sensible defaults.
    /// DATABASE_URL is required; all others have fallbacks or are optional.
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();

        Self {
            grpc_endpoint: env::var("GRPC_ENDPOINT")
                .unwrap_or_else(|_| "https://solana-rpc.parafi.tech:10443".to_string()),
            rpc_endpoint: env::var("RPC_ENDPOINT")
                .unwrap_or_else(|_| "https://solana-rpc.parafi.tech".to_string()),
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default(),
            llm_api_key: env::var("LLM_API_KEY").unwrap_or_default(),
            alert_threshold: env::var("ALERT_THRESHOLD")
                .unwrap_or_else(|_| "30.0".into())
                .parse()
                .expect("ALERT_THRESHOLD must be a number"),
            poll_interval_secs: env::var("POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "60".into())
                .parse()
                .expect("POLL_INTERVAL_SECS must be a number"),
            heartbeat_interval_hours: env::var("HEARTBEAT_INTERVAL_HOURS")
                .unwrap_or_else(|_| "4".into())
                .parse()
                .expect("HEARTBEAT_INTERVAL_HOURS must be a number"),
        }
    }
}
