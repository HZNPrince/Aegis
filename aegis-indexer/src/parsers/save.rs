//! Save (formerly Solend) parser.
//!
//! Save Obligations store pre-computed USD values on-chain using 10^18 (WAD) scaling.
//! Parsed via raw byte offsets since there's no official Rust SDK.

use solana_sdk::bs58;

use crate::{
    grpc::SAVE_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
};

/// Save uses 10^18 (WAD) fixed-point scaling for USD values.
const WAD: u128 = 1_000_000_000_000_000_000;

/// Obligation account size (user positions).
const OBLIGATION_LEN: usize = 1300;
/// Reserve account size (lending pool config, not a user position).
const RESERVE_LEN: usize = 619;

// Byte offsets within the Obligation account data:
const OWNER_START: usize = 42;
const OWNER_END: usize = 74;
const DEPOSITED_VALUE_START: usize = 74;
const DEPOSITED_VALUE_END: usize = 90;
const BORROWED_VALUE_START: usize = 90;
const BORROWED_VALUE_END: usize = 106;

pub struct SaveParser;

impl ProtocolParser for SaveParser {
    fn program_id(&self) -> &str {
        SAVE_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        match data.len() {
            OBLIGATION_LEN => {
                let owner = bs58::encode(&data[OWNER_START..OWNER_END]).into_string();

                let deposited_sf =
                    u128::from_le_bytes(data[DEPOSITED_VALUE_START..DEPOSITED_VALUE_END].try_into().unwrap());
                let borrowed_sf =
                    u128::from_le_bytes(data[BORROWED_VALUE_START..BORROWED_VALUE_END].try_into().unwrap());

                // TODO(save-legs): populate per-asset legs.
                //
                // Save is a Solend fork with no Rust SDK. The Obligation layout
                // past byte 106 (what we read today) continues:
                //   +106 allowed_borrow_value         u128 (16)
                //   +122 unhealthy_borrow_value       u128 (16)
                //   +138 super_unhealthy_borrow_val   u128 (16)
                //   +154 borrowing_isolated_asset     u8   (1)
                //   +155 deposits_len                 u8   (1)
                //   +156 borrows_len                  u8   (1)
                //   +157 <packed deposits then borrows, element sizes TBD>
                //
                // ObligationCollateral / ObligationLiquidity element sizes
                // vary between Solend versions. Safest path: fetch one real
                // Save obligation account, verify element sizes by matching
                // known `deposits_len` + `borrows_len` against total leftover
                // bytes (1300 - 157 = 1143), then code the walk with fixtures
                // under tests/.
                //
                // Leaving aggregate-only for now so we don't emit wrong legs
                // that later feed the executor.
                Some(PositionUpdate {
                    pubkey: pubkey.to_string(),
                    owner,
                    protocol: "SAVE".to_string(),
                    collateral_usd: (deposited_sf / WAD) as f64,
                    debt_usd: (borrowed_sf / WAD) as f64,
                    slot,
                    legs: Vec::new(),
                })
            }
            RESERVE_LEN => None,
            _ => None,
        }
    }
}
