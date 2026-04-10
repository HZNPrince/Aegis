//! Marginfi v2 parser.
//!
//! Marginfi stores deposit/borrow "shares" per bank, not USD values.
//! Converting to USD: shares × bank.share_value / 10^decimals × jupiter_price

use std::sync::Arc;

use borsh::BorshDeserialize;
use carbon_marginfi_v2_decoder::accounts::marginfi_account::MarginfiAccount;

use crate::{
    grpc::MARGINFI_V2_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
    state::AppState,
};

/// MarginfiAccount size (user lending account with up to 16 balance slots).
const MARGINFI_ACCOUNT_LEN: usize = 2312;

pub struct MarginfiParser {
    pub state: Arc<AppState>,
}

impl ProtocolParser for MarginfiParser {
    fn program_id(&self) -> &str {
        MARGINFI_V2_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        if data.len() != MARGINFI_ACCOUNT_LEN {
            return None;
        }

        let account = MarginfiAccount::deserialize(&mut &data[8..]).ok()?;

        let active_balances: Vec<_> = account
            .lending_account
            .balances
            .iter()
            .filter(|b| b.active)
            .collect();

        if active_balances.is_empty() {
            return None;
        }

        let mut total_collateral_usd = 0.0;
        let mut total_debt_usd = 0.0;

        for balance in &active_balances {
            let bank_pk = balance.bank_pk.to_string();

            let Some(bank) = self.state.bank_cache.get(&bank_pk) else {
                continue;
            };

            let decimals_scale = 10f64.powi(bank.mint_decimals as i32);
            let i80f48_scale = (1u128 << 48) as f64;

            let deposit_shares = balance.asset_shares.value as f64 / i80f48_scale;
            let borrow_shares = balance.liability_shares.value as f64 / i80f48_scale;

            // shares × share_value / 10^decimals = human-readable token amount
            let deposit_tokens = deposit_shares * bank.asset_share_value / decimals_scale;
            let borrow_tokens = borrow_shares * bank.liability_share_value / decimals_scale;

            // token_amount × jupiter_price = USD
            let price = self
                .state
                .token_prices
                .get(&bank.mint)
                .map(|p| *p)
                .unwrap_or(0.0);

            total_collateral_usd += deposit_tokens * price;
            total_debt_usd += borrow_tokens * price;
        }

        Some(PositionUpdate {
            pubkey: pubkey.to_string(),
            owner: account.authority.to_string(),
            protocol: "Marginfi".to_string(),
            collateral_usd: total_collateral_usd,
            debt_usd: total_debt_usd,
            slot,
        })
    }
}
