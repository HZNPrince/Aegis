use std::collections::HashMap;

use futures::StreamExt;
use solana_sdk::bs58;
use tonic::transport::ClientTlsConfig;
use tracing::{error, info, warn};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{
    CommitmentLevel, SubscribeRequest, SubscribeRequestFilterAccounts, SubscribeUpdate,
    subscribe_update::UpdateOneof,
};

/// Known lending protocol program IDs on mainnet.
pub const KAMINO_PROGRAM_ID: &str = "KLend2g3cP87fffoy8q1mQqGKjrxjC8boSyAYavgmjD";
pub const SAVE_PROGRAM_ID: &str = "So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo";
pub const MARGINFI_V2_PROGRAM_ID: &str = "MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA";

pub async fn start_account_stream(grpc_endpoint: &str) -> anyhow::Result<()> {
    info!("Connecting to Yellowstone gRPC at {}", grpc_endpoint);

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

    while let Some(message) = stream.next().await {
        match message {
            Ok(update) => {
                update_count += 1;
                process_update(&update, update_count);
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

fn process_update(update: &SubscribeUpdate, count: u64) {
    match &update.update_oneof {
        Some(UpdateOneof::Account(account_update)) => {
            if let Some(account_info) = &account_update.account {
                let pubkey = bs58::encode(&account_info.pubkey).into_string();
                let owner = bs58::encode(&account_info.owner).into_string();
                let data_len = account_info.data.len();
                let slot = account_update.slot;

                info!(
                    "Update #{}: account={} owner={} data_len={} slot={}",
                    count, &pubkey, &owner, data_len, slot
                );
            }
        }
        Some(UpdateOneof::Ping(_)) => {}
        _ => {}
    }
}
