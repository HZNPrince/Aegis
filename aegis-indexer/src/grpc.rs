//! gRPC stream — subscribes to all lending protocol account updates via Yellowstone
//! and dispatches them to protocol-specific parsers.
//!
//! Flow: gRPC stream → parser dispatch → DashMap cache → mpsc channel → DB writer

use std::{collections::HashMap, sync::Arc};

use futures::{SinkExt, StreamExt};
use solana_sdk::bs58;
use tokio::sync::mpsc;
use tonic::transport::ClientTlsConfig;
use tracing::{error, info, warn};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeUpdate,
    subscribe_update::UpdateOneof,
};

use crate::{
    parsers::{
        PositionUpdate, ProtocolParser, kamino::KaminoParser, marginfi::MarginfiParser,
        save::SaveParser,
    },
    state::AppState,
};

// Mainnet program IDs for the three lending protocols we index.
pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const SAVE_PROGRAM_ID: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo";
pub const MARGINFI_V2_PROGRAM_ID: &str = "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA";

/// Connects to Yellowstone gRPC, subscribes to all three lending protocol accounts,
/// and processes updates in a loop until the stream ends.
pub async fn start_account_stream(grpc_endpoint: &str, state: Arc<AppState>) -> anyhow::Result<()> {
    info!("Connecting to Yellowstone gRPC at {}", grpc_endpoint);

    let (tx, rx) = mpsc::channel::<PositionUpdate>(1_000);
    tokio::spawn(run_db_writer(rx, state.clone()));

    let mut client = GeyserGrpcClient::build_from_shared(grpc_endpoint.to_string())?
        .x_token::<String>(None)?
        .connect_timeout(std::time::Duration::from_secs(10))
        .max_decoding_message_size(64 * 1024 * 1024)
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await
        .map_err(|e| anyhow::anyhow!("gRPC connect failed: {}", e))?;

    info!("Connected to gRPC endpoint");

    // Subscribe to ALL accounts owned by the three lending programs.
    // This gives us both user positions (Obligations/MarginfiAccounts) and
    // protocol accounts (Banks/Reserves) in one subscription.
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

    info!("Subscribed to lending protocol accounts. Waiting for updates...");

    // Build parser dispatch map: program_id → parser implementation
    let mut parsers: HashMap<&str, Box<dyn ProtocolParser>> = HashMap::new();
    parsers.insert(KAMINO_PROGRAM_ID, Box::new(KaminoParser));
    parsers.insert(SAVE_PROGRAM_ID, Box::new(SaveParser));
    parsers.insert(
        MARGINFI_V2_PROGRAM_ID,
        Box::new(MarginfiParser {
            state: state.clone(),
        }),
    );

    let mut update_count: u64 = 0;

    while let Some(message) = stream.next().await {
        match message {
            Ok(update) => {
                update_count += 1;
                process_update(&update, update_count, &parsers, &tx, &state);
            }
            Err(e) => {
                error!("Stream error: {:?}", e);
                break;
            }
        }
    }

    warn!("gRPC stream ended after {} updates", update_count);
    Ok(())
}

/// Routes an account update to the appropriate protocol parser based on the
/// account's owner (program ID). If the parser returns a PositionUpdate and
/// the wallet is monitored, caches it and forwards to the DB writer.
fn process_update(
    update: &SubscribeUpdate,
    count: u64,
    parsers: &HashMap<&str, Box<dyn ProtocolParser>>,
    tx: &mpsc::Sender<PositionUpdate>,
    state: &Arc<AppState>,
) {
    let Some(UpdateOneof::Account(account_update)) = &update.update_oneof else {
        return;
    };
    let Some(account_info) = &account_update.account else {
        return;
    };

    let owner = bs58::encode(&account_info.owner).into_string();
    let Some(parser) = parsers.get(owner.as_str()) else {
        return;
    };

    let pubkey = bs58::encode(&account_info.pubkey).into_string();
    let slot = account_update.slot;

    let Some(pos) = parser.try_parse(&pubkey, &account_info.data, slot) else {
        return;
    };

    info!(
        "{} #{}: owner={} collateral_usd={:.2} debt_usd={:.2}",
        pos.protocol, count, pos.owner, pos.collateral_usd, pos.debt_usd
    );

    if state.monitored_wallets.contains_key(&pos.owner) {
        state.positions.insert(pos.pubkey.clone(), pos.clone());

        if let Err(e) = tx.try_send(pos) {
            warn!("DB channel full, dropping update: {}", e);
        }
    }
}

/// Background task that receives PositionUpdates from the mpsc channel
/// and upserts them into Postgres. Uses slot-based deduplication to
/// ignore out-of-order updates.
async fn run_db_writer(mut rx: mpsc::Receiver<PositionUpdate>, state: Arc<AppState>) {
    info!("Database writer spawned");

    while let Some(pos) = rx.recv().await {
        let _ = sqlx::query!(
            "INSERT INTO wallets (pubkey) VALUES ($1) ON CONFLICT (pubkey) DO NOTHING",
            pos.owner
        )
        .execute(&state.db_pool)
        .await;

        let _ = sqlx::query!(
            "INSERT INTO positions (wallet_pubkey, obligation_pubkey, protocol, collateral_usd, debt_usd, last_slot)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (obligation_pubkey) 
             DO UPDATE SET collateral_usd = $4, debt_usd = $5, last_slot = $6, updated_at = NOW()
             WHERE positions.last_slot < $6",
            pos.owner, pos.pubkey, pos.protocol, pos.collateral_usd, pos.debt_usd, pos.slot as i64
        )
        .execute(&state.db_pool)
        .await;
    }
}
