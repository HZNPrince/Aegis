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

/// Canonical mainnet mints we always want priced — regardless of whether a
/// lending-protocol reserve happens to reference them. Keeps the ticker rail
/// and any future UI lookup (e.g. "what's USDC worth?") reliable even when
/// Jupiter skips a mint on partial-response chunks.
const SEED_MINTS: &[&str] = &[
    "So11111111111111111111111111111111111111112", // SOL (wSOL)
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
    "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB", // USDT
    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", // BONK
    "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm", // WIF
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",  // JUP
    "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3", // PYTH
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn",  // JitoSOL
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",   // mSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1",   // bSOL
];

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

    // Union in seed mints so canonical stables/LSTs always get polled, even
    // if for some reason they didn't show up in any discovered bank/reserve.
    let discovered = mint_pubkeys.len();
    for m in SEED_MINTS {
        mint_pubkeys.insert((*m).to_string());
    }

    info!(
        "Oracle discovery complete: {} unique token mints ({} discovered + {} seeded) from {} accounts",
        mint_pubkeys.len(),
        discovered,
        mint_pubkeys.len() - discovered,
        marginfi_mapped + kamino_mapped
    );

    Ok(mint_pubkeys.into_iter().collect())
}


/// Background task: polls Jupiter Price API v3 every 10 seconds for all known token mints.
/// Writes prices into `state.token_prices` (DashMap<mint, f64>).
pub async fn start_jupiter_poller(state: Arc<crate::state::AppState>, mints: Vec<String>) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("reqwest client");

    // Jupiter /price/v3 nominally allows 100 mints per GET, but at that
    // size the URL crosses 4KB and some chunks return partial responses
    // (observed USDC + SOL dropping out repeatedly). 50 is the sweet spot.
    const CHUNK_SIZE: usize = 50;
    let chunks: Vec<Vec<String>> = mints.chunks(CHUNK_SIZE).map(|c| c.to_vec()).collect();

    info!(
        "Jupiter poller started for {} tokens across {} chunks of {}",
        mints.len(),
        chunks.len(),
        CHUNK_SIZE
    );

    loop {
        let mut prices_updated = 0;
        let mut chunks_failed = 0;

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            let ids = chunk.join(",");
            let url = format!("https://api.jup.ag/price/v3?ids={}", ids);

            let result = async {
                let resp = client.get(&url).send().await?;
                let status = resp.status();
                if !status.is_success() {
                    return Err(anyhow::anyhow!("HTTP {} on chunk {}", status, chunk_idx));
                }
                let json: serde_json::Value = resp.json().await?;
                let map = json
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("non-object response on chunk {}", chunk_idx))?;
                let mut updated = 0;
                for (mint, token_data) in map {
                    if let Some(price) =
                        token_data.get("usdPrice").and_then(|v| v.as_f64())
                    {
                        state.token_prices.insert(mint.clone(), price);
                        updated += 1;
                    }
                    if let Some(ch) =
                        token_data.get("priceChange24h").and_then(|v| v.as_f64())
                    {
                        state.token_price_changes.insert(mint.clone(), ch);
                    }
                }
                Ok::<_, anyhow::Error>(updated)
            }
            .await;

            match result {
                Ok(n) => prices_updated += n,
                Err(e) => {
                    chunks_failed += 1;
                    tracing::warn!("jupiter chunk {} failed: {}", chunk_idx, e);
                }
            }

            // Polite pacing between chunks — Jupiter's public tier rate-limits.
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

        tracing::info!(
            "jupiter cycle: {} prices, {} chunks failed, cache={}",
            prices_updated,
            chunks_failed,
            state.token_prices.len()
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    }
}
