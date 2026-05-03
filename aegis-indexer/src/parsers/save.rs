//! Save (formerly Solend) parser.
//!
//! Save obligations store pre-computed USD values on-chain using WAD (10^18) scaling,
//! plus per-asset deposit/borrow arrays from which we extract `PositionLeg`s.
//!
//! No official Rust SDK exists — all parsing is via raw byte offsets verified
//! against the on-chain program layout (RESERVE_LEN = 619, OBLIGATION_LEN = 1300).

use std::sync::Arc;

use aegis_core::{
    symbols::symbol_or_short,
    types::{PositionLeg, PositionSide, default_liquidation_threshold},
};
use solana_sdk::bs58;

use crate::{
    grpc::SAVE_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
    state::AppState,
};

/// Save uses 10^18 (WAD) fixed-point scaling for USD values and borrow amounts.
const WAD: u128 = 1_000_000_000_000_000_000;
const WAD_F64: f64 = WAD as f64;

/// Obligation account size (user positions).
const OBLIGATION_LEN: usize = 1300;

// Byte offsets within the Obligation account data:
const OWNER_START: usize = 42;
const OWNER_END: usize = 74;
const DEPOSITED_VALUE_START: usize = 74;
const DEPOSITED_VALUE_END: usize = 90;
const BORROWED_VALUE_START: usize = 90;
const BORROWED_VALUE_END: usize = 106;
/// Offset of the `deposits_len` u8 field (number of populated deposit slots).
const DEPOSITS_LEN_OFFSET: usize = 202;
/// Offset of the `borrows_len` u8 field.
const BORROWS_LEN_OFFSET: usize = 203;
/// Offset where the flat deposit/borrow array data begins.
const ARRAY_DATA_START: usize = 204;

/// Size of one ObligationCollateral entry (bytes).
/// Layout: reserve(32) + deposited_amount(8) + market_value_wad(16) + padding(32) = 88
const DEPOSIT_ENTRY_SIZE: usize = 88;
/// Size of one ObligationLiquidity entry (bytes).
/// Layout: reserve(32) + cumulative_rate_wad(16) + borrowed_amount_wads(16) + market_value_wad(16) + padding(32) = 112
const BORROW_ENTRY_SIZE: usize = 112;

/// Hard caps — Save's maximum reserve slots per obligation.
const MAX_DEPOSITS: usize = 10;
const MAX_BORROWS: usize = 10;

pub struct SaveParser {
    pub state: Arc<AppState>,
}

impl ProtocolParser for SaveParser {
    fn program_id(&self) -> &str {
        SAVE_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        match data.len() {
            OBLIGATION_LEN => parse_obligation(pubkey, data, slot, &self.state),
            // Update the reserve cache for any reserve-shaped account that streams
            // through gRPC. This self-heals a missing cache entry caused by the
            // initial RPC seed using DataSize(619) — any reserve version that is
            // at least 389 bytes (covers all fields we need) but is not an
            // obligation gets cached here the first time it is touched on-chain.
            len if len >= 389 && len != OBLIGATION_LEN => {
                cache_save_reserve(pubkey, data, &self.state);
                None
            }
            _ => None,
        }
    }
}

/// Parse a Save reserve account and upsert it into the reserve cache.
/// Only requires data.len() >= 389 — safe to call for any reserve size.
fn cache_save_reserve(pubkey: &str, data: &[u8], state: &Arc<AppState>) {
    use solana_sdk::pubkey::Pubkey;

    let Ok(liquidity_mint_pk) = Pubkey::try_from(&data[42..74]) else { return };
    let liquidity_mint = liquidity_mint_pk.to_string();
    if liquidity_mint == "11111111111111111111111111111111" {
        return;
    }

    let mint_decimals = data[74];
    let available_amount = u64::from_le_bytes(data[171..179].try_into().unwrap_or([0u8; 8]));
    let borrowed_amount_wads = u128::from_le_bytes(data[179..195].try_into().unwrap_or([0u8; 16]));

    let Ok(collateral_mint_pk) = Pubkey::try_from(&data[227..259]) else { return };
    let collateral_mint = collateral_mint_pk.to_string();
    let collateral_mint_total_supply =
        u64::from_le_bytes(data[259..267].try_into().unwrap_or([0u8; 8]));
    let liquidation_threshold_pct = data[302];
    let accumulated_fees_wads =
        u128::from_le_bytes(data[373..389].try_into().unwrap_or([0u8; 16]));

    state.save_reserve_cache.insert(
        pubkey.to_string(),
        crate::state::SaveReserveData {
            liquidity_mint: liquidity_mint.clone(),
            mint_decimals,
            collateral_mint,
            collateral_mint_total_supply,
            available_amount,
            borrowed_amount_wads,
            accumulated_fees_wads,
            liquidation_threshold_pct,
        },
    );
    // Ensure the mint is also in token_mints for price polling.
    state.token_mints.insert(pubkey.to_string(), liquidity_mint);
}

fn parse_obligation(
    pubkey: &str,
    data: &[u8],
    slot: u64,
    state: &Arc<AppState>,
) -> Option<PositionUpdate> {
    let owner = bs58::encode(&data[OWNER_START..OWNER_END]).into_string();

    let deposited_sf =
        u128::from_le_bytes(data[DEPOSITED_VALUE_START..DEPOSITED_VALUE_END].try_into().ok()?);
    let borrowed_sf =
        u128::from_le_bytes(data[BORROWED_VALUE_START..BORROWED_VALUE_END].try_into().ok()?);

    // Aggregate USD from obligation header (WAD-scaled).
    let collateral_usd = deposited_sf as f64 / WAD_F64;
    let debt_usd = borrowed_sf as f64 / WAD_F64;

    if collateral_usd == 0.0 && debt_usd == 0.0 {
        return None;
    }

    let deposits_len = data[DEPOSITS_LEN_OFFSET] as usize;
    let borrows_len = data[BORROWS_LEN_OFFSET] as usize;
    // Weighted threshold accumulators (by deposit USD value).
    let mut weighted_threshold_sum = 0.0_f64;
    let mut total_collateral_weight = 0.0_f64;

    // Untrusted on-chain counts: clamp to max to avoid maliciously large values.
    if deposits_len > MAX_DEPOSITS || borrows_len > MAX_BORROWS {
        return Some(PositionUpdate {
            pubkey: pubkey.to_string(),
            owner,
            protocol: "SAVE".to_string(),
            collateral_usd,
            debt_usd,
            slot,
            legs: Vec::new(),
            liquidation_threshold: default_liquidation_threshold(),
        });
    }

    // Bounds check: ensure the array data fits within OBLIGATION_LEN.
    let required = ARRAY_DATA_START
        + deposits_len * DEPOSIT_ENTRY_SIZE
        + borrows_len * BORROW_ENTRY_SIZE;
    if required > data.len() {
        return Some(PositionUpdate {
            pubkey: pubkey.to_string(),
            owner,
            protocol: "SAVE".to_string(),
            collateral_usd,
            debt_usd,
            slot,
            legs: Vec::new(),
            liquidation_threshold: default_liquidation_threshold(),
        });
    }

    let mut legs: Vec<PositionLeg> = Vec::with_capacity(deposits_len + borrows_len);

    // --- Deposit legs ---
    for i in 0..deposits_len {
        let base = ARRAY_DATA_START + i * DEPOSIT_ENTRY_SIZE;

        let reserve_pk = bs58::encode(&data[base..base + 32]).into_string();
        let deposited_amount =
            u64::from_le_bytes(data[base + 32..base + 40].try_into().unwrap_or([0u8; 8]));
        let market_value_wad =
            u128::from_le_bytes(data[base + 40..base + 56].try_into().unwrap_or([0u8; 16]));

        if deposited_amount == 0 && market_value_wad == 0 {
            continue; // empty slot
        }

        let oracle_usd = market_value_wad as f64 / WAD_F64;

        let Some(reserve) = state.save_reserve_cache.get(&reserve_pk) else {
            tracing::warn!("[save] deposit reserve {} not in cache (slot {})", &reserve_pk[..8], slot);
            // Fall back to on-chain oracle USD so collateral stays visible.
            if oracle_usd > 0.0 {
                weighted_threshold_sum += oracle_usd * default_liquidation_threshold();
                total_collateral_weight += oracle_usd;
                legs.push(PositionLeg {
                    side: PositionSide::Collateral,
                    asset_mint: String::new(),
                    asset_symbol: "?".to_string(),
                    amount_native: 0,
                    amount_ui: 0.0,
                    value_usd: oracle_usd,
                    reserve_or_bank: reserve_pk,
                });
            }
            continue;
        };

        let underlying_native = ctokens_to_underlying(deposited_amount, &reserve);
        let decimals_scale = 10f64.powi(reserve.mint_decimals as i32);
        let amount_ui = underlying_native as f64 / decimals_scale;

        // Prefer live Jupiter price — Save's on-chain oracle can be stale.
        let jupiter_usd = state.token_prices.get(&reserve.liquidity_mint)
            .map(|p| amount_ui * *p)
            .unwrap_or(0.0);
        let value_usd = if jupiter_usd > 0.0 { jupiter_usd } else { oracle_usd };

        let threshold = reserve.liquidation_threshold_pct as f64 / 100.0;
        weighted_threshold_sum += value_usd * threshold;
        total_collateral_weight += value_usd;

        legs.push(PositionLeg {
            side: PositionSide::Collateral,
            asset_mint: reserve.liquidity_mint.clone(),
            asset_symbol: symbol_or_short(&reserve.liquidity_mint),
            amount_native: underlying_native,
            amount_ui,
            value_usd,
            reserve_or_bank: reserve_pk,
        });
    }

    // --- Borrow legs ---
    let borrow_base = ARRAY_DATA_START + deposits_len * DEPOSIT_ENTRY_SIZE;
    for j in 0..borrows_len {
        let base = borrow_base + j * BORROW_ENTRY_SIZE;

        let reserve_pk = bs58::encode(&data[base..base + 32]).into_string();
        // +32..+48: cumulative_borrow_rate_wads (skip)
        let borrowed_amount_wads =
            u128::from_le_bytes(data[base + 48..base + 64].try_into().unwrap_or([0u8; 16]));
        let market_value_wad =
            u128::from_le_bytes(data[base + 64..base + 80].try_into().unwrap_or([0u8; 16]));

        if borrowed_amount_wads == 0 && market_value_wad == 0 {
            continue;
        }

        let Some(reserve) = state.save_reserve_cache.get(&reserve_pk) else {
            tracing::warn!("[save] borrow reserve {} not in cache (slot {})", &reserve_pk[..8], slot);
            // Fall back to on-chain oracle USD so the borrow stays visible in the
            // dashboard even while the reserve cache is still warming up.
            let oracle_usd = if market_value_wad > 0 { market_value_wad as f64 / WAD_F64 } else { 0.0 };
            if oracle_usd > 0.0 {
                legs.push(PositionLeg {
                    side: PositionSide::Borrow,
                    asset_mint: String::new(),
                    asset_symbol: "?".to_string(),
                    amount_native: 0,
                    amount_ui: 0.0,
                    value_usd: oracle_usd,
                    reserve_or_bank: reserve_pk,
                });
            }
            continue;
        };

        // WAD integer division: truncates sub-token amounts (correct, matches protocol).
        let amount_native = (borrowed_amount_wads / WAD) as u64;
        // Sub-lamport dust after full repay — skip to avoid a zero-amount ghost leg.
        if amount_native == 0 {
            continue;
        }
        let decimals_scale = 10f64.powi(reserve.mint_decimals as i32);
        let amount_ui = amount_native as f64 / decimals_scale;

        // Always prefer live Jupiter price — Save's on-chain market_value is computed
        // at the time of the last user interaction and can be significantly stale.
        let oracle_usd = if market_value_wad > 0 { market_value_wad as f64 / WAD_F64 } else { 0.0 };
        let jupiter_usd = state.token_prices.get(&reserve.liquidity_mint)
            .map(|p| amount_ui * *p)
            .unwrap_or(0.0);
        let value_usd = if jupiter_usd > 0.0 { jupiter_usd } else { oracle_usd };

        legs.push(PositionLeg {
            side: PositionSide::Borrow,
            asset_mint: reserve.liquidity_mint.clone(),
            asset_symbol: symbol_or_short(&reserve.liquidity_mint),
            amount_native,
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

    // Leg sums are authoritative over the stale obligation header.
    // The header fields (deposited_sf / borrowed_sf) are only updated lazily by
    // Save's oracle — after a full repay the header can still show non-zero debt
    // while every borrow entry has amount_native=0 (dust). Use leg sums when we
    // had entries to parse; fall back to header only when entry counts were zero.
    let final_collateral_usd = if deposits_len > 0 {
        legs.iter()
            .filter(|l| matches!(l.side, PositionSide::Collateral))
            .map(|l| l.value_usd)
            .sum()
    } else {
        collateral_usd
    };
    let final_debt_usd = if borrows_len > 0 {
        legs.iter()
            .filter(|l| matches!(l.side, PositionSide::Borrow))
            .map(|l| l.value_usd)
            .sum()
    } else {
        debt_usd
    };

    if final_collateral_usd == 0.0 && final_debt_usd == 0.0 {
        return None;
    }

    Some(PositionUpdate {
        pubkey: pubkey.to_string(),
        owner,
        protocol: "SAVE".to_string(),
        collateral_usd: final_collateral_usd,
        debt_usd: final_debt_usd,
        slot,
        legs,
        liquidation_threshold,
    })
}

/// Convert cToken lamports → underlying native token amount using the reserve's
/// current exchange rate: total_liquidity / collateral_mint_total_supply.
///
/// If supply is 0 (new empty reserve) returns the cToken amount as-is (1:1).
fn ctokens_to_underlying(deposited_ctokens: u64, reserve: &crate::state::SaveReserveData) -> u64 {
    if reserve.collateral_mint_total_supply == 0 {
        return deposited_ctokens;
    }
    let borrowed_native = (reserve.borrowed_amount_wads / WAD).min(u64::MAX as u128) as u64;
    let fees_native = (reserve.accumulated_fees_wads / WAD).min(u64::MAX as u128) as u64;
    let total_liquidity = reserve
        .available_amount
        .saturating_add(borrowed_native)
        .saturating_sub(fees_native);

    // Use u128 for the multiplication to prevent u64 overflow.
    ((deposited_ctokens as u128 * total_liquidity as u128)
        / reserve.collateral_mint_total_supply as u128) as u64
}
