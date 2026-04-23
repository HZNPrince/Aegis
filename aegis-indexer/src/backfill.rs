//! On-demand backfill — when a wallet is linked, fetch its current positions
//! from all three protocols via RPC and push them through the same parser →
//! DashMap → DB writer pipeline the gRPC stream uses.
//!
//! The trick: the parser is a pure function of (pubkey, bytes, slot). Backfill
//! is just a different byte source.
//!
//! Owner/authority offsets (after 8-byte Anchor discriminator where applicable):
//!   - Save Obligation:  offset 42 (owner)                — confirmed by parser.
//!   - Marginfi Account: offset 40 (authority, 8 + 32)    — group then authority.
//!   - Kamino Obligation: offset 64 (owner, 8+8+16+32)    — tag, last_update, lending_market, owner.
//!
//! We filter server-side by (dataSize + memcmp owner) and re-verify client-side
//! via the parser — if an offset is wrong we'll see a 0-result backfill and
//! catch it fast.

use std::{str::FromStr, sync::Arc};

use klend_sdk::accounts::Obligation as KaminoObligation;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;
use tracing::{info, warn};

use crate::{
    grpc::{KAMINO_PROGRAM_ID, MARGINFI_V2_PROGRAM_ID, SAVE_PROGRAM_ID, process_update},
    parsers::{ProtocolParser, kamino::KaminoParser, marginfi::MarginfiParser, save::SaveParser},
    state::AppState,
};

const SAVE_OBLIGATION_LEN: u64 = 1300;
const MARGINFI_ACCOUNT_LEN: u64 = 2312;

const SAVE_OWNER_OFFSET: usize = 42;
const MARGINFI_AUTHORITY_OFFSET: usize = 40;
const KAMINO_OWNER_OFFSET: usize = 64;

/// Fetches all positions for `wallet` across Kamino/Save/Marginfi and pushes
/// them through the normal cache+DB pipeline. Returns the number of positions
/// parsed and forwarded.
#[allow(deprecated)]
pub async fn backfill_wallet(
    rpc_url: &str,
    state: Arc<AppState>,
    wallet: &str,
) -> anyhow::Result<usize> {
    let wallet_pk = Pubkey::from_str(wallet)
        .map_err(|e| anyhow::anyhow!("invalid wallet pubkey: {}", e))?;
    let wallet_bytes = wallet_pk.to_bytes().to_vec();

    // Ensure the wallet is marked monitored *before* we push updates through
    // `process_update` — otherwise the monitored-wallet guard drops them.
    state.monitored_wallets.insert(wallet.to_string(), true);

    let client = RpcClient::new(rpc_url.to_string());
    let mut parsers: std::collections::HashMap<String, Box<dyn ProtocolParser>> =
        std::collections::HashMap::new();
    parsers.insert(
        KAMINO_PROGRAM_ID.to_string(),
        Box::new(KaminoParser { state: state.clone() }),
    );
    parsers.insert(SAVE_PROGRAM_ID.to_string(), Box::new(SaveParser));
    parsers.insert(
        MARGINFI_V2_PROGRAM_ID.to_string(),
        Box::new(MarginfiParser { state: state.clone() }),
    );

    let db_tx = state.db_writer_tx.clone();
    // Slot 0 ensures any live stream update later overwrites the backfilled row
    // (positions table upsert has WHERE last_slot < $6).
    let slot: u64 = 0;
    let mut total = 0;

    total += fetch_and_dispatch(
        &client,
        KAMINO_PROGRAM_ID,
        KaminoObligation::LEN as u64,
        KAMINO_OWNER_OFFSET,
        &wallet_bytes,
        slot,
        &parsers,
        &db_tx,
        &state,
    )
    .await?;

    total += fetch_and_dispatch(
        &client,
        SAVE_PROGRAM_ID,
        SAVE_OBLIGATION_LEN,
        SAVE_OWNER_OFFSET,
        &wallet_bytes,
        slot,
        &parsers,
        &db_tx,
        &state,
    )
    .await?;

    total += fetch_and_dispatch(
        &client,
        MARGINFI_V2_PROGRAM_ID,
        MARGINFI_ACCOUNT_LEN,
        MARGINFI_AUTHORITY_OFFSET,
        &wallet_bytes,
        slot,
        &parsers,
        &db_tx,
        &state,
    )
    .await?;

    info!("backfill complete for {}: {} positions", wallet, total);
    Ok(total)
}

#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
async fn fetch_and_dispatch(
    client: &RpcClient,
    program_id: &str,
    data_size: u64,
    owner_offset: usize,
    wallet_bytes: &[u8],
    slot: u64,
    parsers: &std::collections::HashMap<String, Box<dyn ProtocolParser>>,
    db_tx: &tokio::sync::mpsc::Sender<crate::parsers::PositionUpdate>,
    state: &Arc<AppState>,
) -> anyhow::Result<usize> {
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            RpcFilterType::DataSize(data_size),
            RpcFilterType::Memcmp(Memcmp::new_raw_bytes(owner_offset, wallet_bytes.to_vec())),
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
            ..Default::default()
        },
        ..Default::default()
    };

    let program_pk = Pubkey::from_str(program_id)
        .map_err(|e| anyhow::anyhow!("bad program id: {}", e))?;

    let accounts = match client
        .get_program_accounts_with_config(&program_pk, config)
        .await
    {
        Ok(a) => a,
        Err(e) => {
            warn!("backfill RPC failed for {}: {}", &program_id[..8], e);
            return Ok(0);
        }
    };

    let mut count = 0;
    for (pubkey, account) in accounts {
        let pubkey_str = pubkey.to_string();
        process_update(
            program_id,
            &pubkey_str,
            &account.data,
            slot,
            parsers,
            db_tx,
            state,
        );
        count += 1;
    }
    info!(
        "backfill {}: {} matching accounts",
        &program_id[..8],
        count
    );
    Ok(count)
}
