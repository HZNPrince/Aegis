use std::{
    collections::HashMap,
    io::{Write, stdout},
    sync::Arc,
};

use futures::StreamExt;
use solana_sdk::{bs58, hash::Hash};
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

/// Known lending protocol program IDs on mainnet.
pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const SAVE_PROGRAM_ID: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo";
pub const MARGINFI_V2_PROGRAM_ID: &str = "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA";

pub async fn start_account_stream(grpc_endpoint: &str, state: Arc<AppState>) -> anyhow::Result<()> {
    info!("Connecting to Yellowstone gRPC at {}", grpc_endpoint);

    // Bounded Channel for Backpressure
    let (tx, mut rx) = mpsc::channel::<PositionUpdate>(1_000);

    // Spawn background dababase writer
    tokio::spawn(run_db_writer(rx, state.clone()));

    let mut client = GeyserGrpcClient::build_from_shared(grpc_endpoint.to_string())?
        .x_token::<String>(None)?
        .connect_timeout(std::time::Duration::from_secs(10))
        .max_decoding_message_size(64 * 1024 * 1024)
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to gRPC: {}", e))?;

    info!("Connected to gRPC endpoint");

    // Filter for ALL three lending protocols in one subscription
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
    let (mut _subscribe_tx, mut stream) = client
        .subscribe()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to subscribe: {}", e))?;

    use futures::SinkExt;
    _subscribe_tx
        .send(subscribe_request)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send subscribe request: {}", e))?;

    info!("Subscribed to lending protocol accounts. Waiting for updates ...");

    let mut update_count: u64 = 0;

    let mut parser_map: HashMap<&str, Box<dyn ProtocolParser>> = HashMap::new();
    parser_map.insert(KAMINO_PROGRAM_ID, Box::new(KaminoParser));
    parser_map.insert(SAVE_PROGRAM_ID, Box::new(SaveParser));
    parser_map.insert(MARGINFI_V2_PROGRAM_ID, Box::new(MarginfiParser));

    while let Some(message) = stream.next().await {
        match message {
            Ok(update) => {
                update_count += 1;
                process_update(&update, update_count, &parser_map, &tx, &state);
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

fn process_update(
    update: &SubscribeUpdate,
    count: u64,
    parsers: &HashMap<&str, Box<dyn ProtocolParser>>,
    tx: &mpsc::Sender<PositionUpdate>,
    state: &Arc<AppState>,
) {
    match &update.update_oneof {
        Some(UpdateOneof::Account(account_update)) => {
            if let Some(account_info) = &account_update.account {
                let pubkey = bs58::encode(&account_info.pubkey).into_string();
                let owner = bs58::encode(&account_info.owner).into_string();
                let slot = account_update.slot;

                if let Some(parser) = parsers.get(owner.as_str()) {
                    if let Some(pos) = parser.try_parse(&pubkey, &account_info.data, slot) {
                        info!(
                            "{} #{}: owner={} collateral_usd={:.2} debt_usd={:.2}",
                            pos.protocol, count, pos.owner, pos.collateral_usd, pos.debt_usd
                        );
                        if state.monitored_wallets.contains_key(&pos.owner) {
                            // Cache in memory for instant API lookups
                            state.positions.insert(pos.pubkey.clone(), pos.clone());

                            if let Err(e) = tx.try_send(pos) {
                                tracing::warn!("Channel full ! Dropping update: {}", e);
                            }
                        }
                    }
                }
            }
        }
        Some(UpdateOneof::Ping(_)) => {}
        _ => {}
    }
}

async fn run_db_writer(mut rx: mpsc::Receiver<PositionUpdate>, state: Arc<AppState>) {
    tracing::info!("Database writer spawned!");

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
