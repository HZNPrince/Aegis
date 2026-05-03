//! Kamino (KLend) parser.
//!
//! Kamino Obligations store per-asset detail in `deposits: [ObligationCollateral; 8]`
//! and `borrows: [ObligationLiquidity; 5]`, plus pre-computed USD totals on
//! the obligation itself (both use 2^60 fixed-point scaling).

use std::sync::Arc;

use aegis_core::{
    symbols::symbol_or_short,
    types::{PositionLeg, PositionSide, default_liquidation_threshold},
};
use klend_sdk::accounts::{Obligation, Reserve};
use solana_sdk::bs58;
use tracing::warn;

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
            Obligation::LEN => {
                let obligation = Obligation::from_bytes(data).ok()?;
                let collateral_total = obligation.deposited_value_sf / FRACTION_ONE_SCALED;
                let debt_total =
                    obligation.borrow_factor_adjusted_debt_value_sf / FRACTION_ONE_SCALED;

                if collateral_total == 0 && debt_total == 0 {
                    return None;
                }

                let mut legs: Vec<PositionLeg> =
                    Vec::with_capacity(obligation.deposits.len() + obligation.borrows.len());

                // Accumulators for weighted-average liquidation threshold.
                let mut weighted_threshold_sum = 0.0_f64;
                let mut total_collateral_weight = 0.0_f64;

                for dep in &obligation.deposits {
                    if dep.deposited_amount == 0 && dep.market_value_sf == 0 {
                        continue;
                    }
                    let reserve_pk = dep.deposit_reserve.to_string();
                    let Some(reserve) = self.state.reserve_cache.get(&reserve_pk) else {
                        continue;
                    };

                    let value_usd = dep.market_value_sf as f64 / FRACTION_ONE_SCALED as f64;
                    let decimals_scale = 10f64.powi(reserve.mint_decimals as i32);

                    // deposited_amount is in cToken units; exchange_rate converts to underlying native.
                    let amount_native =
                        native_to_u64(dep.deposited_amount as f64 * reserve.exchange_rate);
                    let amount_ui = amount_native as f64 / decimals_scale;

                    // Accumulate weighted threshold (weight = collateral USD).
                    let threshold = reserve.liquidation_threshold_pct as f64 / 100.0;
                    weighted_threshold_sum += value_usd * threshold;
                    total_collateral_weight += value_usd;

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

                    let borrowed_native_f =
                        bor.borrowed_amount_sf as f64 / FRACTION_ONE_SCALED as f64;
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

                let liquidation_threshold = if total_collateral_weight > 0.0 {
                    weighted_threshold_sum / total_collateral_weight
                } else {
                    default_liquidation_threshold()
                };

                Some(PositionUpdate {
                    pubkey: pubkey.to_string(),
                    owner: bs58::encode(&obligation.owner).into_string(),
                    protocol: "Kamino".to_string(),
                    collateral_usd: collateral_total as f64,
                    debt_usd: debt_total as f64,
                    slot,
                    legs,
                    liquidation_threshold,
                })
            }
            // Reserve = lending pool config. Not a user position.
            Reserve::LEN => None,
            _ => {
                warn!(
                    "[kamino] unrecognised account size {} for pubkey {} — skipping",
                    data.len(),
                    pubkey
                );
                None
            }
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
