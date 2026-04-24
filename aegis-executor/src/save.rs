//! Save (Solend fork) repay IX builder.
//!
//! Save has no Rust SDK. Instruction layout comes directly from Solend's
//! open-source token-lending program:
//!
//! Tag: u8 = 11
//! Args: { liquidity_amount: u64 }  (u64::MAX = repay all)
//!
//! Accounts (all unchanged in Save vs. Solend):
//!   0. source_liquidity                 [writable]   user's ATA
//!   1. destination_repay_reserve_supply [writable]   reserve.liquidity.supply_pubkey
//!   2. repay_reserve                    [writable]
//!   3. obligation                       [writable]
//!   4. lending_market                   [readonly]
//!   5. user_transfer_authority          [signer, readonly]
//!   6. clock sysvar                     [readonly]
//!   7. token_program                    [readonly]
//!
//! We parse the Reserve account at fixed byte offsets (Solend Pack layout) to
//! pull lending_market + liquidity_mint + supply_pubkey. The full Reserve
//! Pack layout is 619 bytes; the header we need lives in the first ~107.

use crate::{derive_ata, ExecutorError};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

const SAVE_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo");

const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

const REPAY_OBLIGATION_LIQUIDITY_TAG: u8 = 11;

// Reserve Pack layout (Solend/Save), relevant fields:
//   [0..1]     version u8
//   [1..10]    last_update (slot u64 + stale u8)
//   [10..42]   lending_market Pubkey
//   [42..74]   liquidity.mint_pubkey Pubkey
//   [74..75]   liquidity.mint_decimals u8
//   [75..107]  liquidity.supply_pubkey Pubkey
//
// Confirmed against Solend token-lending state layout. Reserve total size = 619.
const RESERVE_LEN: usize = 619;
const LENDING_MARKET_OFFSET: usize = 10;
const LIQUIDITY_MINT_OFFSET: usize = 42;
const LIQUIDITY_SUPPLY_OFFSET: usize = 75;

pub async fn build_repay_ix(
    rpc: &RpcClient,
    wallet: Pubkey,
    obligation: Pubkey,
    repay_reserve: Pubkey,
    expected_mint: Pubkey,
    liquidity_amount: u64,
) -> Result<Instruction, ExecutorError> {
    let account = rpc
        .get_account(&repay_reserve)
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("reserve {repay_reserve}: {e}")))?;

    if account.data.len() != RESERVE_LEN {
        return Err(ExecutorError::Decode(format!(
            "save reserve: expected {RESERVE_LEN} bytes, got {}",
            account.data.len()
        )));
    }

    let lending_market = read_pubkey(&account.data, LENDING_MARKET_OFFSET)?;
    let liquidity_mint = read_pubkey(&account.data, LIQUIDITY_MINT_OFFSET)?;
    let supply_pubkey = read_pubkey(&account.data, LIQUIDITY_SUPPLY_OFFSET)?;

    if liquidity_mint != expected_mint {
        return Err(ExecutorError::Guardrail(format!(
            "save reserve mint {} != expected {}",
            liquidity_mint, expected_mint
        )));
    }

    let source_liquidity = derive_ata(&wallet, &liquidity_mint, &SPL_TOKEN_PROGRAM_ID);

    // Instruction data: tag (1 byte) + liquidity_amount (8 bytes LE)
    let mut data = Vec::with_capacity(9);
    data.push(REPAY_OBLIGATION_LIQUIDITY_TAG);
    data.extend_from_slice(&liquidity_amount.to_le_bytes());

    let accounts = vec![
        AccountMeta::new(source_liquidity, false),
        AccountMeta::new(supply_pubkey, false),
        AccountMeta::new(repay_reserve, false),
        AccountMeta::new(obligation, false),
        AccountMeta::new_readonly(lending_market, false),
        AccountMeta::new_readonly(wallet, true),
        AccountMeta::new_readonly(sysvar::clock::ID, false),
        AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
    ];

    Ok(Instruction {
        program_id: SAVE_PROGRAM_ID,
        accounts,
        data,
    })
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey, ExecutorError> {
    let slice: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or_else(|| ExecutorError::Decode(format!("save reserve: truncated at {offset}")))?
        .try_into()
        .map_err(|_| ExecutorError::Decode("save reserve: 32-byte pubkey slice".into()))?;
    Ok(Pubkey::new_from_array(slice))
}
