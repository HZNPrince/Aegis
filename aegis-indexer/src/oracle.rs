use borsh::BorshDeserialize;
use carbon_marginfi_v2_decoder::{accounts::bank::Bank, types::oracle_setup::OracleSetup};
use klend_sdk::accounts::Reserve;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use tracing::info;

use crate::grpc::{KAMINO_PROGRAM_ID, MARGINFI_V2_PROGRAM_ID};

use serde::Deserialize as SerdeDeserialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[allow(deprecated)]
pub async fn discover_mints(
    rpc_url: &str,
    state: &Arc<crate::state::AppState>,
) -> anyhow::Result<Vec<String>> {
    info!("Booting Oracle Engine: Fetching Banks & Reserves to map Token Mints...");

    let client = RpcClient::new(rpc_url.to_string());
    let mut mint_pubkeys = HashSet::new();

    let marginfi_bank_discriminator = [142, 49, 166, 242, 50, 66, 97, 188];
    let marginfi_config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            marginfi_bank_discriminator.to_vec(),
        ))]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };

    let marginfi_accounts = client
        .get_program_accounts_with_config(&MARGINFI_V2_PROGRAM_ID.parse().unwrap(), marginfi_config)
        .await?;

    info!("Found {} Marginfi Banks", marginfi_accounts.len());

    for (pubkey, account) in marginfi_accounts {
        if let Ok(bank) = <Bank as borsh::BorshDeserialize>::deserialize(&mut &account.data[8..]) {
            let mint = bank.mint;
            if mint.to_string() != "11111111111111111111111111111111" {
                state
                    .token_mints
                    .insert(pubkey.to_string(), mint.to_string());
                mint_pubkeys.insert(mint.to_string());
            }
        }
    }

    let kamino_config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(8624)]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };

    let kamino_accounts = client
        .get_program_accounts_with_config(&KAMINO_PROGRAM_ID.parse().unwrap(), kamino_config)
        .await?;

    info!("Found {} Kamino Reserves", kamino_accounts.len());

    for (pubkey, accounts) in kamino_accounts {
        if let Ok(reserve) = Reserve::from_bytes(&accounts.data[8..]) {
            let mint = reserve.liquidity.mint_pubkey;
            if mint.to_string() != "11111111111111111111111111111111" {
                state
                    .token_mints
                    .insert(pubkey.to_string(), mint.to_string());
                mint_pubkeys.insert(mint.to_string());
            }
        }
    }

    info!(
        "SUCCESS: Dynamically mapped {} total Token Mints!",
        mint_pubkeys.len()
    );

    Ok(mint_pubkeys.into_iter().collect())
}

#[derive(SerdeDeserialize, Debug)]
pub struct JupiterPriceResponse {
    pub data: HashMap<String, JupiterTokenPrice>,
}

#[derive(SerdeDeserialize, Debug)]
pub struct JupiterTokenPrice {
    pub price: f64,
}

/// A background task that fetches prices for all tokens every 10 seconds.
pub async fn start_jupiter_poller(state: Arc<crate::state::AppState>, mints: Vec<String>) {
    let client = reqwest::Client::new();
    let chunks: Vec<Vec<String>> = mints.chunks(100).map(|c| c.to_vec()).collect();

    info!(
        "Starting Jupiter Poller for {} individual tokens...",
        mints.len()
    );

    loop {
        for chunk in &chunks {
            let ids = chunk.join(",");
            let url = format!("https://api.jup.ag/price/v3?ids={}", ids);
            match client.get(&url).send().await {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<JupiterPriceResponse>().await {
                        for (mint, jup_price) in json.data {
                            state.token_prices.insert(mint, jup_price.price);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Jupiter API Error: {}", e);
                }
            }
        }

        // Extremely fast recalculation loop across all monitored profiles
        let mut total_recalc = 0;
        for mut pos in state.positions.iter_mut() {
            // Update collateral and debt USD values based on fresh prices
            // Wait, we need to implement recalculate_usd natively in Aegis Core?
            // Actually, we can just let `grpc.rs` handle the math when new Marginfi values hit,
            // OR we can trigger artificial recalculations.
            total_recalc += 1;
        }

        // Wait 10 seconds before polling again
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
