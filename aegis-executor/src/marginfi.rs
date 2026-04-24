//! Marginfi v2 repay IX builder.
//!
//! `LendingAccountRepay` — discriminator + `{amount: u64, repay_all: Option<bool>}`.
//! The signer must be the MarginfiAccount.authority (or group admin /
//! liquidator, both out of scope for autonomous execution). We always set
//! `repay_all=None` — callers pass an explicit amount.
//!
//! Account order (from on-chain IDL):
//!   0. marginfi_group (readonly)
//!   1. marginfi_account (writable)
//!   2. signer (signer, readonly)
//!   3. bank (writable)
//!   4. signer_token_account (writable)
//!   5. bank_liquidity_vault (writable)
//!   6. token_program (readonly)

use crate::{derive_ata, ExecutorError};
use borsh::BorshSerialize;
use carbon_marginfi_v2_decoder::accounts::bank::Bank;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// From carbon-marginfi-v2-decoder::instructions::LendingAccountRepay.
/// Verified at research time against the decoder's `#[carbon(discriminator)]`
/// attribute (hex `0x4fd1acb1de33ad97`, LE interpretation = these 8 bytes).
const DISCRIMINATOR: [u8; 8] = [0x4f, 0xd1, 0xac, 0xb1, 0xde, 0x33, 0xad, 0x97];

const MARGINFI_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");

/// Marginfi currently uses classic SPL Token (not Token-2022) across all
/// production banks. If that ever changes we can read it off the Bank.
const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

#[derive(BorshSerialize)]
struct RepayArgs {
    amount: u64,
    repay_all: Option<bool>,
}

pub async fn build_repay_ix(
    rpc: &RpcClient,
    wallet: Pubkey,
    marginfi_account: Pubkey,
    bank_pk: Pubkey,
    _expected_mint: Pubkey,
    amount: u64,
) -> Result<Instruction, ExecutorError> {
    let account = rpc
        .get_account(&bank_pk)
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("bank {bank_pk}: {e}")))?;

    // First 8 bytes are the Bank account discriminator; skip them for Borsh.
    if account.data.len() < 8 {
        return Err(ExecutorError::Decode("bank data too short".into()));
    }
    let bank: Bank = borsh::from_slice(&account.data[8..])
        .map_err(|e| ExecutorError::Decode(format!("marginfi bank: {e}")))?;

    let marginfi_group = bank.group;
    let liquidity_vault = bank.liquidity_vault;
    let mint = bank.mint;

    let signer_token_account = derive_ata(&wallet, &mint, &SPL_TOKEN_PROGRAM_ID);

    let mut data = Vec::with_capacity(8 + 9);
    data.extend_from_slice(&DISCRIMINATOR);
    let args = RepayArgs {
        amount,
        repay_all: None,
    };
    args.serialize(&mut data)
        .map_err(|e| ExecutorError::Decode(format!("serialize repay args: {e}")))?;

    let accounts = vec![
        AccountMeta::new_readonly(marginfi_group, false),
        AccountMeta::new(marginfi_account, false),
        AccountMeta::new_readonly(wallet, true),
        AccountMeta::new(bank_pk, false),
        AccountMeta::new(signer_token_account, false),
        AccountMeta::new(liquidity_vault, false),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
    ];

    Ok(Instruction {
        program_id: MARGINFI_PROGRAM_ID,
        accounts,
        data,
    })
}
