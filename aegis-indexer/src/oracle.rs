//! Oracle engine — discovers token mints from lending pools, polls Jupiter for USD prices.
//! Maintains cached bank/reserve data for all three protocols, refreshed periodically from RPC.
//! Used by: aegis-server at startup to discover mints, then runs background price/bank refresh loops.

use klend_sdk::accounts::Reserve;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

use crate::grpc::{KAMINO_PROGRAM_ID, MARGINFI_V2_PROGRAM_ID, SAVE_PROGRAM_ID};

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
    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", // JUP
    "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3", // PYTH
    "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn", // JitoSOL
    "jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v", // jupSOL (Sanctum)
    "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So", // mSOL
    "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1", // bSOL
    "LSoLi2stepNoznKTYxiHD9L91FQhYQdkPiQGFRfaYh8", // lSOL (Sanctum)
    "edge86g9cVz87xcpKpy3J77vbp4wYd9idEV562CCntt",  // edgeSOL (Sanctum)
];

/// Sanctum LST mints that are not reliably priced by Jupiter's price API.
/// These are fetched via Sanctum's sol-value API and multiplied by SOL price.
const SANCTUM_LST_MINTS: &[&str] = &[
    "jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v", // jupSOL
    "LSoLi2stepNoznKTYxiHD9L91FQhYQdkPiQGFRfaYh8", // lSOL
    "edge86g9cVz87xcpKpy3J77vbp4wYd9idEV562CCntt",  // edgeSOL
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
        // Minimum size covers up to liability_weight_maint at offset 160..176.
        if data.len() < 176 {
            continue;
        }

        // Bank layout after 8-byte discriminator:
        //   +0:  mint (32 bytes)
        //   +32: mint_decimals (1 byte)
        //   +33: group (32 bytes)
        //   +65: auto_padding_0 (7 bytes)
        //   +72: asset_share_value (i128, 16 bytes) — I80F48 fixed-point
        //   +88: liability_share_value (i128, 16 bytes)
        //   +104: asset_weight_init (i128, 16 bytes)
        //   +120: asset_weight_maint (i128, 16 bytes)
        //   +136: liability_weight_init (i128, 16 bytes)
        //   +152: liability_weight_maint (i128, 16 bytes)
        let mint = Pubkey::try_from(&data[8..40]).unwrap();
        let mint_str = mint.to_string();
        if mint_str == "11111111111111111111111111111111" {
            continue;
        }

        let mint_decimals = data[40];
        let i80f48_scale = (1u128 << 48) as f64;

        let asset_share_raw = i128::from_le_bytes(data[80..96].try_into().unwrap());
        let liability_share_raw = i128::from_le_bytes(data[96..112].try_into().unwrap());
        let asset_share_value = asset_share_raw as f64 / i80f48_scale;
        let liability_share_value = liability_share_raw as f64 / i80f48_scale;

        let asset_weight_maint_raw = i128::from_le_bytes(data[128..144].try_into().unwrap());
        let liability_weight_maint_raw = i128::from_le_bytes(data[160..176].try_into().unwrap());
        let asset_weight_maint = asset_weight_maint_raw as f64 / i80f48_scale;
        let liability_weight_maint = liability_weight_maint_raw as f64 / i80f48_scale;

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
                asset_weight_maint,
                liability_weight_maint,
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
                    exchange_rate: compute_kamino_exchange_rate(&reserve),
                    liquidation_threshold_pct: reserve.config.liquidation_threshold_pct,
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

    // --- Save Reserves ---
    let save_mints = discover_save_reserves_inner(&client, state).await?;
    info!("Save: {} reserves mapped to mints", save_mints.len());
    for m in save_mints {
        mint_pubkeys.insert(m);
    }

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

/// Fetch all Save reserves and upsert them into `state.save_reserve_cache`.
/// Returns the set of liquidity mints found (for price polling).
///
/// Byte offsets verified against the Save (Solend-fork) on-chain reserve layout.
/// RESERVE_LEN = 619; all reads are within bounds.
#[allow(deprecated)]
async fn discover_save_reserves_inner(
    client: &RpcClient,
    state: &Arc<crate::state::AppState>,
) -> anyhow::Result<HashSet<String>> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(619)]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            ..RpcAccountInfoConfig::default()
        },
        ..RpcProgramAccountsConfig::default()
    };

    let accounts = client
        .get_program_accounts_with_config(&SAVE_PROGRAM_ID.parse().unwrap(), config)
        .await?;

    let mut mints = HashSet::new();

    for (pubkey, account) in &accounts {
        let data = &account.data;
        if data.len() < 389 {
            continue;
        }

        let Ok(liquidity_mint_pk) = Pubkey::try_from(&data[42..74]) else {
            continue;
        };
        let liquidity_mint = liquidity_mint_pk.to_string();
        if liquidity_mint == "11111111111111111111111111111111" {
            continue;
        }
        let mint_decimals = data[74];

        let available_amount = u64::from_le_bytes(data[171..179].try_into().unwrap());
        let borrowed_amount_wads = u128::from_le_bytes(data[179..195].try_into().unwrap());

        let Ok(collateral_mint_pk) = Pubkey::try_from(&data[227..259]) else {
            continue;
        };
        let collateral_mint = collateral_mint_pk.to_string();

        let collateral_mint_total_supply = u64::from_le_bytes(data[259..267].try_into().unwrap());
        let liquidation_threshold_pct = data[302];
        let accumulated_fees_wads = u128::from_le_bytes(data[373..389].try_into().unwrap());

        let pubkey_str = pubkey.to_string();
        state
            .token_mints
            .insert(pubkey_str.clone(), liquidity_mint.clone());
        state.save_reserve_cache.insert(
            pubkey_str,
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
        mints.insert(liquidity_mint);
    }

    Ok(mints)
}

/// Background task: refreshes Marginfi bank share values every 30 s.
///
/// Share values increase every slot as interest accrues. Without refresh,
/// cached values drift and health scores underreport both collateral and debt.
/// Uses in-place upsert — never clears the cache.
#[allow(deprecated)]
pub async fn start_bank_refresh_loop(rpc_url: String, state: Arc<crate::state::AppState>) {
    let client = RpcClient::new(rpc_url);
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let config = RpcProgramAccountsConfig {
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

        match client
            .get_program_accounts_with_config(&MARGINFI_V2_PROGRAM_ID.parse().unwrap(), config)
            .await
        {
            Ok(accounts) => {
                let mut updated = 0usize;
                for (pubkey, account) in &accounts {
                    let data = &account.data;
                    if data.len() < 176 {
                        continue;
                    }
                    let mint = Pubkey::try_from(&data[8..40]).unwrap();
                    let mint_str = mint.to_string();
                    if mint_str == "11111111111111111111111111111111" {
                        continue;
                    }
                    let mint_decimals = data[40];
                    let i80f48_scale = (1u128 << 48) as f64;
                    let asset_share_value =
                        i128::from_le_bytes(data[80..96].try_into().unwrap()) as f64 / i80f48_scale;
                    let liability_share_value =
                        i128::from_le_bytes(data[96..112].try_into().unwrap()) as f64
                            / i80f48_scale;
                    let asset_weight_maint = i128::from_le_bytes(data[128..144].try_into().unwrap())
                        as f64
                        / i80f48_scale;
                    let liability_weight_maint =
                        i128::from_le_bytes(data[160..176].try_into().unwrap()) as f64
                            / i80f48_scale;

                    state.bank_cache.insert(
                        pubkey.to_string(),
                        crate::state::BankData {
                            mint: mint_str,
                            mint_decimals,
                            asset_share_value,
                            liability_share_value,
                            asset_weight_maint,
                            liability_weight_maint,
                        },
                    );
                    updated += 1;
                }
                info!("[oracle] bank refresh: {} banks updated", updated);
            }
            Err(e) => warn!("[oracle] bank refresh failed: {}", e),
        }
    }
}

/// Compute the cToken→underlying exchange rate for a Kamino reserve.
/// exchange_rate = total_liquidity / collateral_mint_total_supply
fn compute_kamino_exchange_rate(reserve: &Reserve) -> f64 {
    let supply = reserve.collateral.mint_total_supply;
    if supply == 0 {
        return 1.0;
    }
    let borrowed = reserve.liquidity.borrowed_amount_sf as u128 / (1u128 << 60);
    let fees = reserve.liquidity.accumulated_protocol_fees_sf as u128 / (1u128 << 60);
    let available = reserve.liquidity.available_amount as u128;
    let total_liquidity = available.saturating_add(borrowed).saturating_sub(fees);
    total_liquidity as f64 / supply as f64
}

/// Background task: refreshes Kamino reserve exchange rates and liquidation thresholds every 30 s.
#[allow(deprecated)]
pub async fn start_kamino_reserve_refresh_loop(
    rpc_url: String,
    state: Arc<crate::state::AppState>,
) {
    let client = RpcClient::new(rpc_url);
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        let config = RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::DataSize(8624)]),
            account_config: RpcAccountInfoConfig {
                encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                ..RpcAccountInfoConfig::default()
            },
            ..RpcProgramAccountsConfig::default()
        };

        match client
            .get_program_accounts_with_config(&KAMINO_PROGRAM_ID.parse().unwrap(), config)
            .await
        {
            Ok(accounts) => {
                let mut updated = 0usize;
                for (pubkey, account) in &accounts {
                    if let Ok(reserve) = Reserve::from_bytes(&account.data) {
                        let mint_str = reserve.liquidity.mint_pubkey.to_string();
                        if mint_str == "11111111111111111111111111111111" {
                            continue;
                        }
                        state.reserve_cache.insert(
                            pubkey.to_string(),
                            crate::state::ReserveData {
                                mint: mint_str,
                                mint_decimals: reserve.liquidity.mint_decimals as u8,
                                exchange_rate: compute_kamino_exchange_rate(&reserve),
                                liquidation_threshold_pct: reserve.config.liquidation_threshold_pct,
                            },
                        );
                        updated += 1;
                    }
                }
                info!(
                    "[oracle] kamino reserve refresh: {} reserves updated",
                    updated
                );
            }
            Err(e) => warn!("[oracle] kamino reserve refresh failed: {}", e),
        }
    }
}

/// Background task: refreshes Save reserve exchange rates every 30 s.
///
/// cToken→underlying exchange rates change with every interest accrual.
/// Without refresh, `amount_native` values fed to the executor drift from reality.
pub async fn start_save_reserve_refresh_loop(rpc_url: String, state: Arc<crate::state::AppState>) {
    let client = RpcClient::new(rpc_url);
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;

        match discover_save_reserves_inner(&client, &state).await {
            Ok(n) => info!(
                "[oracle] save reserve refresh: {} reserves updated",
                n.len()
            ),
            Err(e) => warn!("[oracle] save reserve refresh failed: {}", e),
        }
    }
}

/// Background task: polls Jupiter Price API v3 every 10 seconds for all known token mints.
/// Writes prices into `state.token_prices` (DashMap<mint, f64>).
///
/// **Critical fix:** SEED_MINTS (SOL, USDC, USDT, etc.) are polled in a
/// dedicated priority chunk before all other mints. This ensures the core
/// prices used for position repricing are never dropped due to rate-limiting
/// on larger discovery chunks.
pub async fn start_jupiter_poller(state: Arc<crate::state::AppState>, _initial_mints: Vec<String>) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("reqwest client");

    let seed_set: std::collections::HashSet<&str> = SEED_MINTS.iter().copied().collect();
    let seed_chunk: Vec<String> = SEED_MINTS.iter().map(|s| s.to_string()).collect();

    // Smaller chunks (50) avoid URL length issues and reduce the blast radius
    // of a single 429 — losing 50 obscure mints is better than losing SOL+USDC.
    const CHUNK_SIZE: usize = 50;

    info!("Jupiter poller started: dynamic mint list driven by state.token_mints");

    loop {
        // Pull the live mint set from state each cycle. Newly-discovered
        // banks/reserves (added by the refresh loops) automatically join the
        // poll list — without this, the initial boot snapshot would be frozen
        // forever and tokens like USDG (added post-boot) never get priced.
        let mut live_mints: std::collections::HashSet<String> = state
            .token_mints
            .iter()
            .map(|kv| kv.value().clone())
            .collect();
        for m in SEED_MINTS {
            live_mints.insert((*m).to_string());
        }
        let remaining: Vec<String> = live_mints
            .into_iter()
            .filter(|m| !seed_set.contains(m.as_str()))
            .collect();
        let other_chunks: Vec<Vec<String>> =
            remaining.chunks(CHUNK_SIZE).map(|c| c.to_vec()).collect();

        let mut prices_updated = 0;
        let mut chunks_failed = 0;

        // --- Priority: poll seed mints first with retry ---
        let seed_result = poll_jupiter_chunk(&client, &state, &seed_chunk, "seed").await;
        match seed_result {
            Ok(n) => prices_updated += n,
            Err(_) => {
                // Retry once after a 3s backoff — seed prices are critical.
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                match poll_jupiter_chunk(&client, &state, &seed_chunk, "seed-retry").await {
                    Ok(n) => prices_updated += n,
                    Err(e) => {
                        chunks_failed += 1;
                        tracing::warn!("jupiter seed chunk failed even after retry: {}", e);
                    }
                }
            }
        }

        // Pace before starting the remaining chunks.
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        // --- Remaining protocol mints ---
        for (chunk_idx, chunk) in other_chunks.iter().enumerate() {
            let label = format!("chunk-{}", chunk_idx);
            match poll_jupiter_chunk(&client, &state, chunk, &label).await {
                Ok(n) => prices_updated += n,
                Err(e) => {
                    chunks_failed += 1;
                    tracing::warn!("jupiter {} failed: {}", label, e);
                }
            }

            // 2s pacing between chunks to stay well under rate limits.
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        }

        tracing::info!(
            "jupiter cycle: {} updated, {} chunks failed, polled={} mints, cache={}",
            prices_updated,
            chunks_failed,
            seed_chunk.len() + remaining.len(),
            state.token_prices.len()
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
    }
}

/// Poll a single chunk of mints from Jupiter and insert prices into state.
async fn poll_jupiter_chunk(
    client: &reqwest::Client,
    state: &Arc<crate::state::AppState>,
    mints: &[String],
    label: &str,
) -> anyhow::Result<usize> {
    if mints.is_empty() {
        return Ok(0);
    }
    let ids = mints.join(",");
    let url = format!("https://api.jup.ag/price/v3?ids={}", ids);

    let resp = client.get(&url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("HTTP {} on {}", status, label));
    }
    let json: serde_json::Value = resp.json().await?;
    let map = json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("non-object response on {}", label))?;
    let mut updated = 0;
    for (mint, token_data) in map {
        if let Some(price) = token_data.get("usdPrice").and_then(|v| v.as_f64()) {
            state.token_prices.insert(mint.clone(), price);
            updated += 1;
        }
        if let Some(ch) = token_data.get("priceChange24h").and_then(|v| v.as_f64()) {
            state.token_price_changes.insert(mint.clone(), ch);
        }
    }
    Ok(updated)
}
/// Background task: prices Sanctum LSTs not covered by Jupiter's price API.
///
/// Sanctum's sol-value API returns the SOL NAV (net asset value) for each LST.
/// We multiply by the current SOL USD price to get USD price.
/// Runs every 60 s; silently skips the cycle if SOL price is not yet loaded.
pub async fn start_sanctum_lst_poller(state: Arc<crate::state::AppState>) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("reqwest client");

    let sol_mint = "So11111111111111111111111111111111111111112";
    let lst_param = SANCTUM_LST_MINTS.join(",");

    info!(
        "Sanctum LST poller started for {} mints",
        SANCTUM_LST_MINTS.len()
    );

    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;

        let Some(sol_price) = state.token_prices.get(sol_mint).map(|p| *p) else {
            continue;
        };
        if sol_price <= 0.0 {
            continue;
        }

        let url = format!(
            "https://extra.jup.ag/v1/sol-value/current?lst={}",
            lst_param
        );

        let result: anyhow::Result<()> = async {
            let resp = client.get(&url).send().await?;
            if !resp.status().is_success() {
                return Err(anyhow::anyhow!("HTTP {}", resp.status()));
            }
            let json: serde_json::Value = resp.json().await?;
            let map = json.as_object().ok_or_else(|| anyhow::anyhow!("non-object"))?;
            let mut updated = 0;
            for (mint, sol_val) in map {
                // sol_val is a string decimal (e.g. "1.0432")
                let nav: f64 = sol_val
                    .as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| sol_val.as_f64())
                    .unwrap_or(0.0);
                if nav > 0.0 {
                    state.token_prices.insert(mint.clone(), nav * sol_price);
                    updated += 1;
                }
            }
            info!("[oracle] sanctum LST cycle: {} prices updated", updated);
            Ok(())
        }
        .await;

        if let Err(e) = result {
            warn!("[oracle] sanctum LST fetch failed: {}", e);
        }
    }
}
