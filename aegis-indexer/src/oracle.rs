//! Oracle Engine — discovers token mints from lending pools and polls Jupiter for USD prices.
//!
//! At startup, fetches all Marginfi Banks and Kamino Reserves via RPC to build
//! a map of (bank/reserve pubkey → token mint). Then spawns a background task
//! that polls Jupiter Price API every 10 seconds to keep USD prices fresh.

use klend_sdk::accounts::Reserve;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::info;

use crate::grpc::{KAMINO_PROGRAM_ID, MARGINFI_V2_PROGRAM_ID};

/// 8-byte Anchor discriminator for Marginfi Bank accounts.
const MARGINFI_BANK_DISCRIMINATOR: [u8; 8] = [142, 49, 166, 242, 50, 66, 97, 188];

/// Fetches all Marginfi Banks and Kamino Reserves, extracts their token mints,
/// and stores the (bank/reserve pubkey → mint) mapping in `state.token_mints`.
///
/// Returns the list of unique mint addresses for price polling.
#[allow(deprecated)]
pub async fn discover_mints(
    rpc_url: &str,
    state: &Arc<crate::state::AppState>,
) -> anyhow::Result<Vec<String>> {
    let client = RpcClient::new(rpc_url.to_string());
    let mut mint_pubkeys = HashSet::new();

    // --- Marginfi Banks ---
    // Bank layout: [8-byte discriminator][32-byte mint][...rest]
    // We read the mint directly from raw bytes (immune to SDK version mismatches).
    let marginfi_config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            MARGINFI_BANK_DISCRIMINATOR.to_vec(),
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

    let mut marginfi_mapped = 0;
    for (pubkey, account) in &marginfi_accounts {
        let data = &account.data;
        if data.len() < 112 {
            continue;
        }

        // Bank layout after 8-byte discriminator:
        //   +0:  mint (32 bytes)
        //   +32: mint_decimals (1 byte)
        //   +33: group (32 bytes)
        //   +65: auto_padding_0 (7 bytes)
        //   +72: asset_share_value (i128, 16 bytes) — I80F48 fixed-point
        //   +88: liability_share_value (i128, 16 bytes)
        let mint = Pubkey::try_from(&data[8..40]).unwrap();
        let mint_str = mint.to_string();
        if mint_str == "11111111111111111111111111111111" {
            continue;
        }

        let mint_decimals = data[40];
        let asset_share_raw = i128::from_le_bytes(data[80..96].try_into().unwrap());
        let liability_share_raw = i128::from_le_bytes(data[96..112].try_into().unwrap());

        // I80F48 → f64: divide by 2^48
        let i80f48_scale = (1u128 << 48) as f64;
        let asset_share_value = asset_share_raw as f64 / i80f48_scale;
        let liability_share_value = liability_share_raw as f64 / i80f48_scale;

        marginfi_mapped += 1;
        let pubkey_str = pubkey.to_string();

        state
            .token_mints
            .insert(pubkey_str.clone(), mint_str.clone());
        state.bank_cache.insert(
            pubkey_str,
            crate::state::BankData {
                mint: mint_str.clone(),
                mint_decimals,
                asset_share_value,
                liability_share_value,
            },
        );
        mint_pubkeys.insert(mint_str);
    }

    info!(
        "Marginfi: {}/{} banks mapped to mints",
        marginfi_mapped,
        marginfi_accounts.len()
    );

    // --- Kamino Reserves ---
    // Filter by data size (Reserve::LEN = 8624). Deserialize with klend-sdk.
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

    let mut kamino_mapped = 0;
    for (pubkey, account) in &kamino_accounts {
        if let Ok(reserve) = Reserve::from_bytes(&account.data) {
            let mint_str = reserve.liquidity.mint_pubkey.to_string();
            if mint_str == "11111111111111111111111111111111" {
                continue;
            }
            kamino_mapped += 1;
            let pubkey_str = pubkey.to_string();
            state
                .token_mints
                .insert(pubkey_str.clone(), mint_str.clone());
            state.reserve_cache.insert(
                pubkey_str,
                crate::state::ReserveData {
                    mint: mint_str.clone(),
                    mint_decimals: reserve.liquidity.mint_decimals as u8,
                },
            );
            mint_pubkeys.insert(mint_str);
        }
    }

    info!(
        "Kamino: {}/{} reserves mapped to mints",
        kamino_mapped,
        kamino_accounts.len()
    );

    info!(
        "Oracle discovery complete: {} unique token mints from {} accounts",
        mint_pubkeys.len(),
        marginfi_mapped + kamino_mapped
    );

    Ok(mint_pubkeys.into_iter().collect())
}


/// Background task: polls Jupiter Price API v3 every 10 seconds for all known token mints.
/// Writes prices into `state.token_prices` (DashMap<mint, f64>).
pub async fn start_jupiter_poller(state: Arc<crate::state::AppState>, mints: Vec<String>) {
    let client = reqwest::Client::new();
    // Jupiter allows up to 100 token IDs per request
    let chunks: Vec<Vec<String>> = mints.chunks(100).map(|c| c.to_vec()).collect();

    info!("Jupiter poller started for {} tokens", mints.len());

    loop {
        let mut prices_updated = 0;

        for chunk in &chunks {
            let ids = chunk.join(",");
            let url = format!("https://api.jup.ag/price/v3?ids={}", ids);

            match client.get(&url).send().await {
                Ok(resp) => {
                    // Parse as generic JSON to handle tokens with null/missing usdPrice
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            if let Some(map) = json.as_object() {
                                for (mint, token_data) in map {
                                    if let Some(price) = token_data.get("usdPrice").and_then(|v| v.as_f64()) {
                                        state.token_prices.insert(mint.clone(), price);
                                        prices_updated += 1;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Jupiter: failed to parse response: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Jupiter API error: {}", e);
                }
            }
        }

        if prices_updated > 0 {
            tracing::debug!("Jupiter: updated {} token prices", prices_updated);
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
