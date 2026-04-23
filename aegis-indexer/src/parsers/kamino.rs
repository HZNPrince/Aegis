//! Kamino (KLend) parser.
//!
//! Kamino Obligations store per-asset detail in `deposits: [ObligationCollateral; 8]`
//! and `borrows: [ObligationLiquidity; 5]`, plus pre-computed USD totals on
//! the obligation itself (both use 2^60 fixed-point scaling).
//!
//! We keep the aggregates for the existing risk engine path, and additionally
//! emit one `PositionLeg` per populated slot so the executor can identify the
//! specific reserve + mint + amount to act on.

use std::sync::Arc;

use aegis_core::{
    symbols::symbol_or_short,
    types::{PositionLeg, PositionSide},
};
use klend_sdk::accounts::{Obligation, Reserve};
use solana_sdk::bs58;

use crate::{
    grpc::KAMINO_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
    state::AppState,
};

/// Kamino uses 2^60 fixed-point scaling for all USD values (FRACTION_ONE_SCALED).
const FRACTION_ONE_SCALED: u128 = 1u128 << 60;

pub struct KaminoParser {
    pub state: Arc<AppState>,
}

impl ProtocolParser for KaminoParser {
    fn program_id(&self) -> &str {
        KAMINO_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        match data.len() {
            // Obligation = user position. Contains deposited/borrowed USD values.
            Obligation::LEN => {
                let obligation = Obligation::from_bytes(data).ok()?;
                let collateral_total = obligation.deposited_value_sf / FRACTION_ONE_SCALED;
                let debt_total = obligation.borrow_factor_adjusted_debt_value_sf / FRACTION_ONE_SCALED;

                if collateral_total == 0 && debt_total == 0 {
                    return None;
                }

                // Aggregate totals come from the obligation header (already
                // borrow-factor-adjusted on the debt side). Legs carry
                // per-asset market value straight from each slot.
                let mut legs: Vec<PositionLeg> =
                    Vec::with_capacity(obligation.deposits.len() + obligation.borrows.len());

                for dep in &obligation.deposits {
                    if dep.deposited_amount == 0 && dep.market_value_sf == 0 {
                        continue;
                    }
                    let reserve_pk = dep.deposit_reserve.to_string();
                    // Skip if we don't yet know this reserve — it will be
                    // discovered on the next oracle refresh.
                    let Some(reserve) = self.state.reserve_cache.get(&reserve_pk) else {
                        continue;
                    };

                    let value_usd = dep.market_value_sf as f64 / FRACTION_ONE_SCALED as f64;
                    let price = self
                        .state
                        .token_prices
                        .get(&reserve.mint)
                        .map(|p| *p)
                        .unwrap_or(0.0);
                    let decimals_scale = 10f64.powi(reserve.mint_decimals as i32);

                    // `deposited_amount` is cToken units, not liquidity-token
                    // units. For display we show the liquidity-equivalent
                    // derived from USD / price, which is accurate enough for
                    // the dashboard. The executor re-derives exact native
                    // amounts from the Reserve at ix-build time.
                    let amount_ui = if price > 0.0 { value_usd / price } else { 0.0 };
                    let amount_native = native_to_u64(amount_ui * decimals_scale);

                    legs.push(PositionLeg {
                        side: PositionSide::Collateral,
                        asset_mint: reserve.mint.clone(),
                        asset_symbol: symbol_or_short(&reserve.mint),
                        amount_native,
                        amount_ui,
                        value_usd,
                        reserve_or_bank: reserve_pk,
                    });
                }

                for bor in &obligation.borrows {
                    if bor.borrowed_amount_sf == 0 && bor.market_value_sf == 0 {
                        continue;
                    }
                    let reserve_pk = bor.borrow_reserve.to_string();
                    let Some(reserve) = self.state.reserve_cache.get(&reserve_pk) else {
                        continue;
                    };

                    // `borrowed_amount_sf` is native liquidity units scaled
                    // by 2^60 — descale once to get true native amount.
                    let borrowed_native_f = bor.borrowed_amount_sf as f64 / FRACTION_ONE_SCALED as f64;
                    let decimals_scale = 10f64.powi(reserve.mint_decimals as i32);
                    let amount_ui = borrowed_native_f / decimals_scale;
                    let value_usd = bor.market_value_sf as f64 / FRACTION_ONE_SCALED as f64;

                    legs.push(PositionLeg {
                        side: PositionSide::Borrow,
                        asset_mint: reserve.mint.clone(),
                        asset_symbol: symbol_or_short(&reserve.mint),
                        amount_native: native_to_u64(borrowed_native_f),
                        amount_ui,
                        value_usd,
                        reserve_or_bank: reserve_pk,
                    });
                }

                Some(PositionUpdate {
                    pubkey: pubkey.to_string(),
                    owner: bs58::encode(&obligation.owner).into_string(),
                    protocol: "Kamino".to_string(),
                    collateral_usd: collateral_total as f64,
                    debt_usd: debt_total as f64,
                    slot,
                    legs,
                })
            }
            // Reserve = lending pool config. Not a user position.
            Reserve::LEN => None,
            _ => None,
        }
    }
}

fn native_to_u64(x: f64) -> u64 {
    if !x.is_finite() || x <= 0.0 {
        return 0;
    }
    if x >= u64::MAX as f64 {
        return u64::MAX;
    }
    x.round() as u64
}
