//! aegis-executor
//!
//! Builds unsigned transactions that a user's wallet (Phantom etc.) can sign
//! to act on a tripped guardrail. Kamino, Save, and Marginfi repay IXs are
//! implemented here; each protocol's handler verifies the signer is the
//! obligation/account authority, so delegate-based autonomous execution is
//! not possible — all three paths produce an unsigned tx that the user signs.

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
    guardrails::validate(req)?;

    let wallet = parse_pubkey(&req.wallet, "wallet")?;
    let mint = parse_pubkey(&req.mint, "mint")?;
    let reserve_or_bank = parse_pubkey(&req.reserve_or_bank, "reserve_or_bank")?;
    let obligation_or_account =
        parse_pubkey(&req.obligation_or_account, "obligation_or_account")?;

    let ix: Instruction = match req.protocol.as_str() {
        "Kamino" => {
            kamino::build_repay_ix(
                &ctx.rpc,
                wallet,
                obligation_or_account,
                reserve_or_bank,
                mint,
                req.amount_native,
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
                req.amount_native,
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
            )
            .await?
        }
        other => return Err(ExecutorError::UnknownProtocol(other.to_string())),
    };

    let (recent_blockhash, last_valid_block_height) = ctx
        .rpc
        .get_latest_blockhash_with_commitment(CommitmentConfig::confirmed())
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("blockhash: {e}")))?;

    let tx = wrap_unsigned(wallet, &[ix], recent_blockhash)?;
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
