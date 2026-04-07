use klend_sdk::accounts::{Obligation, Reserve};
use solana_sdk::bs58;

use crate::{
    grpc::KAMINO_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
};

/// Kamino uses 2^60 fixed-point scaling for USD values (FRACTION_ONE_SCALED)
const FRACTION_ONE_SCALED: u128 = 1u128 << 60;

pub struct KaminoParser;
impl ProtocolParser for KaminoParser {
    fn program_id(&self) -> &str {
        KAMINO_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        match data.len() {
            Obligation::LEN => {
                let obligation = Obligation::from_bytes(data).ok()?;
                let collateral = obligation.deposited_value_sf / FRACTION_ONE_SCALED;
                let debt = obligation.borrow_factor_adjusted_debt_value_sf / FRACTION_ONE_SCALED;

                if collateral == 0 && debt == 0 {
                    return None;
                }

                Some(PositionUpdate {
                    pubkey: pubkey.to_string(),
                    owner: bs58::encode(&obligation.owner).into_string(),
                    protocol: "Kamino".to_string(),
                    collateral_usd: collateral as f64,
                    debt_usd: debt as f64,
                    slot,
                })
            }
            Reserve::LEN => None, // Todo
            _ => None,
        }
    }
}
