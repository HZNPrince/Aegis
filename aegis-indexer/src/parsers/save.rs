use solana_sdk::bs58;

use crate::{
    grpc::SAVE_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
};

/// Save (Solend) uses 10^18 (WAD) fixed-point scaling
const SAVE_WAD: u128 = 1_000_000_000_000_000_000;

pub struct SaveParser;

impl ProtocolParser for SaveParser {
    fn program_id(&self) -> &str {
        SAVE_PROGRAM_ID
    }
    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        match data.len() {
            1300 => {
                let ob_owner = bs58::encode(&data[42..74]).into_string();

                let deposited_value_sf = u128::from_le_bytes(data[74..90].try_into().unwrap());

                let borrowed_value_sf = u128::from_le_bytes(data[90..106].try_into().unwrap());

                Some(PositionUpdate {
                    pubkey: pubkey.to_string(),
                    owner: ob_owner,
                    protocol: "SAVE".to_string(),
                    collateral_usd: (deposited_value_sf / SAVE_WAD) as f64,
                    debt_usd: (borrowed_value_sf / SAVE_WAD) as f64,
                    slot,
                })
            }
            619 => None,
            _ => None,
        }
    }
}
