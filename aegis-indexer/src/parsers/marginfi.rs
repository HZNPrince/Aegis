use borsh::BorshDeserialize;
use carbon_marginfi_v2_decoder::accounts::marginfi_account::MarginfiAccount;

use crate::{
    grpc::MARGINFI_V2_PROGRAM_ID,
    parsers::{PositionUpdate, ProtocolParser},
};

pub struct MarginfiParser;

impl ProtocolParser for MarginfiParser {
    fn program_id(&self) -> &str {
        MARGINFI_V2_PROGRAM_ID
    }

    fn try_parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate> {
        if data.len() == 2312 {
            if let Ok(marginfi) = MarginfiAccount::deserialize(&mut &data[8..]) {
                let mut active_balances = 0;
                for balance in marginfi.lending_account.balances {
                    if balance.active {
                        active_balances += 1;
                    }
                }

                if active_balances > 0 {
                    return Some(PositionUpdate {
                        pubkey: pubkey.to_string(),
                        owner: marginfi.authority.to_string(),
                        protocol: "Marginfi".to_string(),
                        collateral_usd: 0.0, // Fixed in Phase 2 with Pyth Oracles
                        debt_usd: 0.0,       // Fixed in Phase 2 with Pyth Oracles
                        slot,
                    });
                }
            }
        }
        None
    }
}
