//! Executor-side guardrails. These run after the alerts engine has decided
//! to emit a RepayDebt intent — they defend the tx-build path against
//! obviously-bad requests (zero amount, mismatched protocol, budget cap).
//!
//! Rule-level cooldowns and last-fired tracking live in the alerts engine;
//! we don't re-check them here.

use crate::{BuildRepayRequest, ExecutorError};
use aegis_core::types::ActionKind;

pub fn validate(req: &BuildRepayRequest) -> Result<(), ExecutorError> {
    if req.amount_native == 0 {
        return Err(ExecutorError::Guardrail("amount must be > 0".into()));
    }

    // If a rule was attached, sanity-check that it's a repay action and
    // (when provided) its per-action USD cap hasn't been obviously bypassed.
    if let Some(rule) = &req.rule {
        if rule.action_kind != ActionKind::RepayDebt {
            return Err(ExecutorError::Guardrail(format!(
                "rule action_kind={:?} is not RepayDebt",
                rule.action_kind
            )));
        }
        if !rule.is_active {
            return Err(ExecutorError::Guardrail("rule is inactive".into()));
        }
        if rule.max_usd_per_action <= 0.0 {
            return Err(ExecutorError::Guardrail(
                "rule.max_usd_per_action must be positive".into(),
            ));
        }
        if let Some(protocol_filter) = &rule.protocol {
            if !protocol_filter.eq_ignore_ascii_case(&req.protocol) {
                return Err(ExecutorError::Guardrail(format!(
                    "rule targets protocol={} but request is {}",
                    protocol_filter, req.protocol
                )));
            }
        }
        if let Some(token) = &rule.action_token {
            if token != &req.mint {
                return Err(ExecutorError::Guardrail(format!(
                    "rule targets token={} but request mint={}",
                    token, req.mint
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aegis_core::types::{ActionKind, GuardRule, TriggerKind};

    fn base_rule() -> GuardRule {
        GuardRule {
            id: None,
            wallet: "w".into(),
            protocol: None,
            trigger_kind: TriggerKind::LtvAbove,
            trigger_value: 0.8,
            action_kind: ActionKind::RepayDebt,
            action_token: None,
            action_amount_usd: None,
            max_usd_per_action: 1000.0,
            daily_limit_usd: 5000.0,
            cooldown_seconds: 600,
            is_active: true,
            created_at: None,
            updated_at: None,
            last_fired_at: None,
        }
    }

    fn base_req() -> BuildRepayRequest {
        BuildRepayRequest {
            wallet: "W".into(),
            obligation_or_account: "O".into(),
            protocol: "Kamino".into(),
            reserve_or_bank: "R".into(),
            mint: "M".into(),
            amount_native: 100,
            rule: None,
        }
    }

    #[test]
    fn rejects_zero_amount() {
        let mut r = base_req();
        r.amount_native = 0;
        assert!(validate(&r).is_err());
    }

    #[test]
    fn rejects_wrong_action_kind() {
        let mut rule = base_rule();
        rule.action_kind = ActionKind::NotifyOnly;
        let mut r = base_req();
        r.rule = Some(rule);
        assert!(validate(&r).is_err());
    }

    #[test]
    fn rejects_inactive_rule() {
        let mut rule = base_rule();
        rule.is_active = false;
        let mut r = base_req();
        r.rule = Some(rule);
        assert!(validate(&r).is_err());
    }

    #[test]
    fn rejects_protocol_mismatch() {
        let mut rule = base_rule();
        rule.protocol = Some("Marginfi".into());
        let mut r = base_req();
        r.rule = Some(rule);
        assert!(validate(&r).is_err());
    }

    #[test]
    fn rejects_token_mismatch() {
        let mut rule = base_rule();
        rule.action_token = Some("OTHER".into());
        let mut r = base_req();
        r.rule = Some(rule);
        assert!(validate(&r).is_err());
    }

    #[test]
    fn accepts_valid() {
        let mut r = base_req();
        r.rule = Some(base_rule());
        assert!(validate(&r).is_ok());
    }
}
