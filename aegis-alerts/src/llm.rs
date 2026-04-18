use aegis_core::types::{AlertRecord, AlertSeverity};
use aegis_risk::health::WalletRisk;
use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertPayload {
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    api_key: Option<String>,
    base_url: String,
    model: String,
}

impl LlmClient {
    pub fn from_env() -> Self {
        Self {
            http: reqwest::Client::new(),
            api_key: std::env::var("LLM_API_KEY").ok().filter(|v| !v.is_empty()),
            base_url: std::env::var("LLM_BASE_URL")
                .unwrap_or_else(|_| "https://api.groq.com/openai/v1/chat/completions".to_string()),
            model: std::env::var("LLM_MODEL").unwrap_or_else(|_| "llama-3.3-70b-versatile".to_string()),
        }
    }

    pub async fn explain_risk(&self, risk: &WalletRisk) -> anyhow::Result<AlertPayload> {
        if self.api_key.is_none() {
            return Ok(fallback_explanation(risk));
        }

        let prompt = build_prompt(risk);
        let request = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "You are Aegis, a Solana lending risk copilot. Return concise JSON only."},
                {"role": "user", "content": prompt}
            ],
            "response_format": {
                "type": "json_object"
            }
        });

        let response = self
            .http
            .post(&self.base_url)
            .bearer_auth(self.api_key.as_ref().unwrap())
            .json(&request)
            .send()
            .await
            .context("failed to call LLM")?;

        let body: serde_json::Value = response.json().await.context("invalid LLM response body")?;
        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .context("missing LLM content")?;

        serde_json::from_str(content).context("failed to parse LLM alert payload")
    }
}

pub fn build_prompt(risk: &WalletRisk) -> String {
    format!(
        "Wallet: {}\nHealth score: {:.2}\nLTV: {:.4}\nCollateral USD: {:.2}\nDebt USD: {:.2}\nLiquidation buffer USD: {:.2}\nProtocols: {}\nReturn JSON {{\"severity\": \"Info|Warning|Critical\", \"title\": string, \"message\": string, \"suggested_actions\": string[]}}.",
        risk.wallet,
        risk.health_score,
        risk.ltv,
        risk.total_collateral_usd,
        risk.total_debt_usd,
        risk.liquidation_buffer_usd,
        serde_json::to_string(&risk.protocols).unwrap_or_else(|_| "[]".to_string())
    )
}

pub fn fallback_explanation(risk: &WalletRisk) -> AlertPayload {
    let mut suggested_actions = Vec::new();

    if risk.ltv >= 0.75 {
        suggested_actions.push("Add collateral to create a wider liquidation buffer.".to_string());
    }
    if risk.total_debt_usd > 0.0 {
        suggested_actions.push("Repay part of the debt on the highest-LTV protocol first.".to_string());
    }
    if suggested_actions.is_empty() {
        suggested_actions.push("No immediate action required. Keep monitoring rate and price changes.".to_string());
    }

    let title = match risk.severity {
        AlertSeverity::Critical => "Liquidation risk is elevated",
        AlertSeverity::Warning => "Position health is weakening",
        AlertSeverity::Info => "Position health is stable",
    }
    .to_string();

    let message = format!(
        "Wallet health is {:.1}/100 with {:.2} USD collateral, {:.2} USD debt, and {:.2} USD buffer before the modeled liquidation threshold.",
        risk.health_score, risk.total_collateral_usd, risk.total_debt_usd, risk.liquidation_buffer_usd
    );

    AlertPayload {
        severity: risk.severity,
        title,
        message,
        suggested_actions,
    }
}

pub fn into_alert_record(risk: WalletRisk, payload: AlertPayload) -> AlertRecord {
    AlertRecord {
        id: None,
        wallet: risk.wallet,
        severity: payload.severity,
        title: payload.title,
        message: payload.message,
        health_score: risk.health_score,
        ltv: risk.ltv,
        suggested_actions: payload.suggested_actions,
        metadata: serde_json::json!({
            "protocols": risk.protocols,
            "liquidation_buffer_usd": risk.liquidation_buffer_usd,
        }),
        created_at: None,
    }
}
