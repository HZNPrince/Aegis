//! aegis-executor — builds unsigned transactions for user signatures to act on guard rules.
//! Produces Versioned Transactions for Kamino, Save, and Marginfi repay instructions.
//! Each protocol's handler verifies the signer is the obligation/account authority.
//! Used by: aegis-api endpoint POST /api/intents/repay, which calls build_repay_tx().

use aegis_core::types::{GuardRule, PositionLeg, PositionSide};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::CommitmentConfig};
use solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    message::{v0::Message as MessageV0, VersionedMessage},
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use std::str::FromStr;
use std::sync::Arc;

pub mod guardrails;
pub mod kamino;
pub mod marginfi;
pub mod save;
pub mod wsol;

/// Input to the executor: identifies which debt leg to repay, under which
/// rule, and for how much. Amount is in native token units (pre-decimals).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRepayRequest {
    pub wallet: String,
    pub obligation_or_account: String,
    pub protocol: String,
    pub reserve_or_bank: String,
    pub mint: String,
    pub amount_native: u64,
    /// When true, build a "repay everything" tx (Save/Kamino: u64::MAX sentinel,
    /// Marginfi: repay_all=Some(true)) so dust + accrued-interest races between
    /// the indexer cache and on-chain state can't strand a leg.
    #[serde(default)]
    pub repay_all: bool,
    pub rule: Option<GuardRule>,
}

impl BuildRepayRequest {
    pub fn from_leg(
        wallet: &str,
        obligation_or_account: &str,
        protocol: &str,
        leg: &PositionLeg,
        amount_native: u64,
        rule: Option<GuardRule>,
    ) -> Self {
        debug_assert_eq!(leg.side, PositionSide::Borrow);
        Self {
            wallet: wallet.to_string(),
            obligation_or_account: obligation_or_account.to_string(),
            protocol: protocol.to_string(),
            reserve_or_bank: leg.reserve_or_bank.clone(),
            mint: leg.asset_mint.clone(),
            amount_native,
            repay_all: false,
            rule,
        }
    }
}

/// What the executor returns: a base64 serialized VersionedTransaction ready
/// for `wallet.signAndSend(bs58.decode(...))` in the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedTx {
    pub protocol: String,
    pub wallet: String,
    pub amount_native: u64,
    pub mint: String,
    pub tx_base64: String,
    pub last_valid_block_height: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("invalid pubkey: {0}")]
    InvalidPubkey(String),
    #[error("unknown protocol: {0}")]
    UnknownProtocol(String),
    #[error("guardrail violation: {0}")]
    Guardrail(String),
    #[error("on-chain account fetch failed: {0}")]
    RpcFetch(String),
    #[error("account decode failed: {0}")]
    Decode(String),
    #[error("rpc error: {0}")]
    Rpc(#[from] solana_client::client_error::ClientError),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Everything the executor needs at runtime. Keep it clone-cheap.
#[derive(Clone)]
pub struct ExecutorContext {
    pub rpc: Arc<RpcClient>,
    pub intent_ttl_secs: i64,
}

impl ExecutorContext {
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc: Arc::new(RpcClient::new(rpc_url.to_string())),
            intent_ttl_secs: 120,
        }
    }
}

/// Top-level entry point: validate, build IX per protocol, wrap in a
/// VersionedTransaction with the user as fee payer.
pub async fn build_repay_tx(
    ctx: &ExecutorContext,
    req: &BuildRepayRequest,
) -> Result<UnsignedTx, ExecutorError> {
    // Distinctive banner: if you don't see this in `cargo run -p aegis-server`
    // stdout when clicking Repay, the *old* binary is running.
    tracing::info!(
        "█ [executor v3 — chain-fetched token_program + raw-offset bank decode] build_repay_tx: protocol={} wallet={} mint={} amount_native={} repay_all={}",
        req.protocol, req.wallet, req.mint, req.amount_native, req.repay_all
    );

    guardrails::validate(req)?;

    // Save & Kamino use u64::MAX as the canonical "repay everything" sentinel.
    // Marginfi has its own repay_all bool wired in marginfi::build_repay_ix.
    let amount_for_proto = match req.protocol.as_str() {
        "SAVE" | "Save" | "Kamino" if req.repay_all => u64::MAX,
        _ => req.amount_native,
    };

    let wallet = parse_pubkey(&req.wallet, "wallet")?;
    let mint = parse_pubkey(&req.mint, "mint")?;
    let reserve_or_bank = parse_pubkey(&req.reserve_or_bank, "reserve_or_bank")?;
    let obligation_or_account =
        parse_pubkey(&req.obligation_or_account, "obligation_or_account")?;

    let mut ixs = Vec::new();
    let is_wsol = req.mint == wsol::WSOL_MINT.to_string();
    let user_token_account = if is_wsol {
        // Derive WSOL ATA
        derive_ata(&wallet, &wsol::WSOL_MINT, &wsol::SPL_TOKEN_PROGRAM_ID)
    } else {
        derive_ata(&wallet, &mint, &wsol::SPL_TOKEN_PROGRAM_ID)
    };

    if is_wsol {
        ixs.extend(wsol::build_wsol_wrap_ixs(wallet, user_token_account, req.amount_native));
    }

    let proto_ixs: Vec<Instruction> = match req.protocol.as_str() {
        "Kamino" => {
            kamino::build_repay_ix(
                &ctx.rpc,
                wallet,
                obligation_or_account,
                reserve_or_bank,
                mint,
                amount_for_proto,
            )
            .await?
        }
        "SAVE" | "Save" => {
            save::build_repay_ix(
                &ctx.rpc,
                wallet,
                obligation_or_account,
                reserve_or_bank,
                mint,
                amount_for_proto,
            )
            .await?
        }
        "Marginfi" => {
            marginfi::build_repay_ix(
                &ctx.rpc,
                wallet,
                obligation_or_account,
                reserve_or_bank,
                mint,
                req.amount_native,
                req.repay_all,
            )
            .await?
        }
        other => return Err(ExecutorError::UnknownProtocol(other.to_string())),
    };

    ixs.extend(proto_ixs);

    if is_wsol {
        ixs.push(wsol::build_wsol_close_ix(wallet, user_token_account));
    }

    let (recent_blockhash, last_valid_block_height) = ctx
        .rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("blockhash: {e}")))?;

    let tx = wrap_unsigned(wallet, &ixs, recent_blockhash)?;
    let bytes = bincode::serialize(&tx).map_err(|e| ExecutorError::Decode(e.to_string()))?;

    Ok(UnsignedTx {
        protocol: req.protocol.clone(),
        wallet: req.wallet.clone(),
        amount_native: req.amount_native,
        mint: req.mint.clone(),
        tx_base64: B64.encode(bytes),
        last_valid_block_height,
    })
}

fn wrap_unsigned(
    payer: Pubkey,
    ixs: &[Instruction],
    blockhash: Hash,
) -> Result<VersionedTransaction, ExecutorError> {
    let msg = MessageV0::try_compile(&payer, ixs, &[], blockhash)
        .map_err(|e| ExecutorError::Decode(format!("compile: {e}")))?;
    // Placeholder signatures: one empty sig per required signer. The wallet
    // replaces them on signAndSend. This keeps the serialized tx bytes
    // layout identical to a signed tx so Phantom/base58 flows work.
    let sig_count = msg.header.num_required_signatures as usize;
    let signatures = vec![solana_sdk::signature::Signature::default(); sig_count];
    Ok(VersionedTransaction {
        signatures,
        message: VersionedMessage::V0(msg),
    })
}

pub(crate) fn parse_pubkey(s: &str, field: &str) -> Result<Pubkey, ExecutorError> {
    Pubkey::from_str(s).map_err(|_| ExecutorError::InvalidPubkey(format!("{field}={s}")))
}

/// Derive the Associated Token Account for (wallet, mint, token_program).
pub(crate) fn derive_ata(wallet: &Pubkey, mint: &Pubkey, token_program: &Pubkey) -> Pubkey {
    const ATA_PROGRAM_ID: Pubkey =
        solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
    let (ata, _) = Pubkey::find_program_address(
        &[wallet.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ATA_PROGRAM_ID,
    );
    ata
}
