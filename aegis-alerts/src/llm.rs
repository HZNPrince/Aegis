//! LLM integration — converts wallet risk snapshots into human-readable alert payloads.
//! Uses OpenAI or OpenRouter APIs to generate alert titles, messages, and suggested actions.
//! Used by: alert engine, which calls explain_risk() on every fired alert before dispatching.

use aegis_core::types::{AlertRecord, AlertSeverity, WalletRisk};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tracing::warn;

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
        let api_key = std::env::var("OPENAI_API_KEY")
            .ok()
            .or_else(|| std::env::var("LLM_API_KEY").ok())
            .filter(|v| !v.is_empty());

        Self {
            http: reqwest::Client::new(),
            api_key,
            base_url: std::env::var("LLM_BASE_URL").unwrap_or_else(|_| {
                "https://openrouter.ai/api/v1/chat/completions".to_string()
            }),
            model: std::env::var("LLM_MODEL")
                .unwrap_or_else(|_| "openai/gpt-oss-120b:free".to_string()),
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
            .header("HTTP-Referer", "https://github.com/HZNPrince/Aegis")
            .header("X-Title", "Aegis")
            .json(&request)
            .send()
            .await
            .context("failed to call LLM")?;

        let body: serde_json::Value = match response.json().await {
            Ok(b) => b,
            Err(e) => {
                warn!("[llm] non-JSON response, using fallback: {e}");
                return Ok(fallback_explanation(risk));
            }
        };
        let Some(content) = body["choices"][0]["message"]["content"].as_str() else {
            warn!("[llm] missing content in response, using fallback");
            return Ok(fallback_explanation(risk));
        };

        match serde_json::from_str(content) {
            Ok(payload) => Ok(payload),
            Err(e) => {
                warn!("[llm] failed to parse alert payload, using fallback: {e}");
                Ok(fallback_explanation(risk))
            }
        }
    }
}

pub fn build_prompt(risk: &WalletRisk) -> String {
    let mut pos_lines = Vec::new();
    for pos in &risk.positions {
        for leg in &pos.legs {
            let side = match leg.side {
                aegis_core::types::PositionSide::Collateral => "Collateral",
                aegis_core::types::PositionSide::Borrow => "Borrow",
            };
            pos_lines.push(format!(
                "{} {} {} ${:.2}",
                pos.protocol, leg.asset_symbol, side, leg.value_usd
            ));
        }
        if pos.legs.is_empty() {
            if pos.collateral_usd > 0.0 {
                pos_lines.push(format!("{} (Collateral) ${:.2}", pos.protocol, pos.collateral_usd));
            }
            if pos.debt_usd > 0.0 {
                pos_lines.push(format!("{} (Borrow) ${:.2}", pos.protocol, pos.debt_usd));
            }
        }
    }

    format!(
        "Solana DeFi wallet risk summary:\nWallet: {}\nHealth: {:.1}/100  LTV: {:.1}%  Buffer: ${:.2}\nCollateral: ${:.2}  Debt: ${:.2}\nPositions:\n{}\n\nReturn JSON {{\"severity\": \"Info|Warning|Critical\", \"title\": string (≤60 chars), \"message\": string (1-2 sentences, mention specific assets), \"suggested_actions\": string[] (≤3 items)}}.",
        risk.wallet,
        risk.health_score,
        risk.ltv * 100.0,
        risk.liquidation_buffer_usd,
        risk.total_collateral_usd,
        risk.total_debt_usd,
        if pos_lines.is_empty() { "none".to_string() } else { pos_lines.join("\n") }
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
        telegram_chat_id: None,
    }
}
