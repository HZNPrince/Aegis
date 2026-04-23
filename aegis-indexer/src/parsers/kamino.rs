//! Kamino (KLend) parser.
//!
//! Kamino Obligations store pre-computed USD values on-chain using 2^60 fixed-point scaling.
//! We just read and descale them — no external price lookups needed.

use klend_sdk::accounts::{Obligation, Reserve};
use solana_sdk::bs58;

use crate::{
    grpc::KAMINO_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
};

/// Kamino uses 2^60 fixed-point scaling for all USD values (FRACTION_ONE_SCALED).
const FRACTION_ONE_SCALED: u128 = 1u128 << 60;

pub struct KaminoParser;

impl ProtocolParser for KaminoParser {
    fn program_id(&self) -> &str {
        KAMINO_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        match data.len() {
            // Obligation = user position. Contains deposited/borrowed USD values.
            Obligation::LEN => {
                let obligation = Obligation::from_bytes(data).ok()?;
                let collateral = obligation.deposited_value_sf / FRACTION_ONE_SCALED;
                let debt = obligation.borrow_factor_adjusted_debt_value_sf / FRACTION_ONE_SCALED;

                if collateral == 0 && debt == 0 {
                    return None;
                }

                // TODO(autonomous-execution): populate `legs` from
                // `obligation.deposits[]` / `obligation.borrows[]`. Each
                // entry carries a reserve pubkey and a share amount; we need
                // a reserve→mint lookup (Reserve.liquidity.mint_pubkey) plus
                // collateral_exchange_rate to derive native token amounts.
                // Left aggregate-only for now — risk engine is unaffected.
                Some(PositionUpdate {
                    pubkey: pubkey.to_string(),
                    owner: bs58::encode(&obligation.owner).into_string(),
                    protocol: "Kamino".to_string(),
                    collateral_usd: collateral as f64,
                    debt_usd: debt as f64,
                    slot,
                    legs: Vec::new(),
                })
            }
            // Reserve = lending pool config. Not a user position.
            Reserve::LEN => None,
            _ => None,
        }
    }
}
