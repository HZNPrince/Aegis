//! gRPC stream — subscribes to all lending protocol account updates via Yellowstone
//! and dispatches them to protocol-specific parsers.
//!
//! The public entrypoint `start_account_stream` is a supervisor: it runs
//! `run_session` in a loop and reconnects with jittered exponential backoff
//! when the session errors or ends cleanly. Durable resources (state, parsers,
//! the DB writer channel) live across reconnects; only the gRPC client and
//! stream are re-created per session.

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::{SinkExt, StreamExt};
use solana_sdk::bs58;
use tokio::time::Instant;
use tracing::{error, info, warn};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts,
    subscribe_update::UpdateOneof,
};

use crate::{
    parsers::{
        PositionUpdate, ProtocolParser, kamino::KaminoParser, marginfi::MarginfiParser,
        save::SaveParser,
    },
    state::AppState,
};

pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const SAVE_PROGRAM_ID: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo";
pub const MARGINFI_V2_PROGRAM_ID: &str = "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA";

/// A "stable" session is one that stayed up long enough to reset the backoff.
const STABLE_SESSION_THRESHOLD: Duration = Duration::from_secs(30);
const BACKOFF_BASE_MS: u64 = 500;
const BACKOFF_CAP_MS: u64 = 30_000;

/// Supervisor loop: reconnects forever with jittered exponential backoff.
pub async fn start_account_stream(
    grpc_endpoint: &str,
    state: Arc<AppState>,
) -> anyhow::Result<()> {
    let parsers = build_parsers(state.clone());
    let mut attempt: u32 = 0;

    loop {
        let started = Instant::now();
        let result = run_session(grpc_endpoint, state.clone(), parsers.clone()).await;
        let stable = started.elapsed() >= STABLE_SESSION_THRESHOLD;

        match result {
            Ok(()) => warn!("gRPC session ended cleanly after {:?}", started.elapsed()),
            Err(e) => error!("gRPC session error after {:?}: {}", started.elapsed(), e),
        }

        if stable {
            attempt = 0;
        }
        attempt = attempt.saturating_add(1);

        let backoff = compute_backoff(attempt);
        warn!("reconnecting in {:?} (attempt {})", backoff, attempt);
        tokio::time::sleep(backoff).await;
    }
}

fn build_parsers(state: Arc<AppState>) -> Arc<HashMap<String, Box<dyn ProtocolParser>>> {
    let mut parsers: HashMap<String, Box<dyn ProtocolParser>> = HashMap::new();
    parsers.insert(KAMINO_PROGRAM_ID.to_string(), Box::new(KaminoParser));
    parsers.insert(SAVE_PROGRAM_ID.to_string(), Box::new(SaveParser));
    parsers.insert(
        MARGINFI_V2_PROGRAM_ID.to_string(),
        Box::new(MarginfiParser { state }),
    );
    Arc::new(parsers)
}

/// Full jitter exponential backoff: sleep = rand(0, min(cap, base * 2^attempt)).
///
/// "Full jitter" prevents a thundering herd: if N clients drop at the same time
/// deterministic backoff has them all reconnect on the same tick. Spreading
/// uniformly across the window decorrelates them.
fn compute_backoff(attempt: u32) -> Duration {
    let shift = attempt.min(10);
    let exp_ms = BACKOFF_BASE_MS.saturating_mul(1u64 << shift);
    let max_ms = exp_ms.min(BACKOFF_CAP_MS);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let jittered = nanos % max_ms.max(1);
    Duration::from_millis(jittered)
}

/// One gRPC session: connect, subscribe, consume until error or EOF.
async fn run_session(
    grpc_endpoint: &str,
    state: Arc<AppState>,
    parsers: Arc<HashMap<String, Box<dyn ProtocolParser>>>,
) -> anyhow::Result<()> {
    info!("Connecting to Yellowstone gRPC at {}", grpc_endpoint);

    let mut client = GeyserGrpcClient::build_from_shared(grpc_endpoint.to_string())?
        .x_token::<String>(None)?
        .connect_timeout(Duration::from_secs(10))
        .max_decoding_message_size(64 * 1024 * 1024)
        .connect()
        .await
        .map_err(|e| anyhow::anyhow!("gRPC connect failed: {}", e))?;

    info!("Connected to gRPC endpoint");

    let mut accounts_filter: HashMap<String, SubscribeRequestFilterAccounts> = HashMap::new();
    accounts_filter.insert(
        "lending_positions".to_string(),
        SubscribeRequestFilterAccounts {
            account: vec![],
            owner: vec![
                KAMINO_PROGRAM_ID.to_string(),
                SAVE_PROGRAM_ID.to_string(),
                MARGINFI_V2_PROGRAM_ID.to_string(),
            ],
            filters: vec![],
            nonempty_txn_signature: None,
        },
    );

    let subscribe_request = SubscribeRequest {
        accounts: accounts_filter,
        commitment: Some(CommitmentLevel::Confirmed as i32),
        slots: HashMap::new(),
        transactions: HashMap::new(),
        transactions_status: HashMap::new(),
        blocks: HashMap::new(),
        blocks_meta: HashMap::new(),
        entry: HashMap::new(),
        accounts_data_slice: Vec::new(),
        ping: None,
        from_slot: None,
    };

    let (mut subscribe_tx, mut stream) = client
        .subscribe()
        .await
        .map_err(|e| anyhow::anyhow!("Subscribe failed: {}", e))?;

    subscribe_tx
        .send(subscribe_request)
        .await
        .map_err(|e| anyhow::anyhow!("Send subscribe request failed: {}", e))?;

    info!("Subscribed to lending protocol accounts");

    let db_tx = state.db_writer_tx.clone();
    let mut update_count: u64 = 0;
    let mut dispatched: u64 = 0;
    let mut last_stats = Instant::now();

    while let Some(message) = stream.next().await {
        let update = message.map_err(|e| anyhow::anyhow!("stream error: {:?}", e))?;
        update_count += 1;

        let Some(UpdateOneof::Account(account_update)) = update.update_oneof else {
            continue;
        };
        let Some(account_info) = account_update.account else {
            continue;
        };

        let owner = bs58::encode(&account_info.owner).into_string();
        if !parsers.contains_key(&owner) {
            continue;
        }

        dispatched += 1;
        let pubkey = bs58::encode(&account_info.pubkey).into_string();
        let slot = account_update.slot;

        process_update(&owner, &pubkey, &account_info.data, slot, &parsers, &db_tx, &state);

        if last_stats.elapsed() >= Duration::from_secs(30) {
            info!(
                "STATS: {} messages | {} dispatched | {} cached positions",
                update_count,
                dispatched,
                state.positions.len()
            );
            last_stats = Instant::now();
        }
    }

    Ok(())
}

/// Parses + caches + forwards to the DB writer. Pure function on the hot path —
/// no spawning, no awaits. Keep it cheap.
pub(crate) fn process_update(
    owner: &str,
    pubkey: &str,
    data: &[u8],
    slot: u64,
    parsers: &HashMap<String, Box<dyn ProtocolParser>>,
    db_tx: &tokio::sync::mpsc::Sender<PositionUpdate>,
    state: &Arc<AppState>,
) {
    let Some(parser) = parsers.get(owner) else {
        return;
    };

    let Some(pos) = parser.try_parse(pubkey, data, slot) else {
        return;
    };

    if !state.monitored_wallets.contains_key(&pos.owner) {
        return;
    }

    state.positions.insert(pos.pubkey.clone(), pos.clone());

    if let Err(e) = db_tx.try_send(pos) {
        warn!("DB channel full, dropping update: {}", e);
    }
}
