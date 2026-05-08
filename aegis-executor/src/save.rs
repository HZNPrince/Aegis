//! Save (Solend-fork) repay instruction builder — constructs repayObligationLiquidity transaction.
//! No Rust SDK; instruction layout is from solend-sdk repayObligationLiquidity.ts.
//! Parses Reserve layout at fixed offsets for lending_market, liquidity_mint, and supply_vault.
//! Used by: executor's build_repay_tx() for Save protocol repayments.

use crate::ExecutorError;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    sysvar,
};

const SAVE_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo");

const REPAY_OBLIGATION_LIQUIDITY_TAG: u8 = 11;

// Reserve Pack layout (Solend/Save), relevant fields:
//   [0..1]     version u8
//   [1..10]    last_update (slot u64 + stale u8)
//   [10..42]   lending_market Pubkey
//   [42..74]   liquidity.mint_pubkey Pubkey
//   [74..75]   liquidity.mint_decimals u8
//   [75..107]  liquidity.supply_pubkey Pubkey
//   [107..139] liquidity.pyth_oracle_pubkey
//   [139..171] liquidity.switchboard_oracle_pubkey
//
// Confirmed against Solend token-lending state layout. Reserve total size = 619.
const RESERVE_LEN: usize = 619;
const LENDING_MARKET_OFFSET: usize = 10;
const LIQUIDITY_MINT_OFFSET: usize = 42;
const LIQUIDITY_SUPPLY_OFFSET: usize = 75;
const RESERVE_PYTH_OFFSET: usize = 107;
const RESERVE_SWITCHBOARD_OFFSET: usize = 139;

// LendingMarket Pack layout, relevant fields:
//   [0..1]     version u8
//   [1..2]     bump_seed u8
//   [2..34]    owner Pubkey
//   [34..66]   quote_currency [u8;32]
//   [66..98]   token_program_id Pubkey
//   [98..130]  oracle_program_id
//   [130..162] switchboard_oracle_program_id
//   [162..290] padding [u8;128]
const LENDING_MARKET_LEN: usize = 290;
const LM_TOKEN_PROGRAM_OFFSET: usize = 66;


pub async fn build_repay_ix(
    rpc: &RpcClient,
    wallet: Pubkey,
    obligation: Pubkey,
    repay_reserve: Pubkey,
    expected_mint: Pubkey,
    liquidity_amount: u64,
) -> Result<Vec<Instruction>, ExecutorError> {
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

    // Fetch LendingMarket and read its stored token_program_id. The on-chain
    // repay processor fails (custom error 0x9) if accounts[7] != this value.
    let lm_account = rpc
        .get_account(&lending_market)
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("lending_market {lending_market}: {e}")))?;
    if lm_account.data.len() < LENDING_MARKET_LEN {
        return Err(ExecutorError::Decode(format!(
            "save lending_market: expected ≥{LENDING_MARKET_LEN} bytes, got {}",
            lm_account.data.len()
        )));
    }
    let token_program_id = read_pubkey(&lm_account.data, LM_TOKEN_PROGRAM_OFFSET)?;

    tracing::info!(
        "[save repay v3] reserve={} lending_market={} token_program_id={} supply={} mint={}",
        repay_reserve, lending_market, token_program_id, supply_pubkey, liquidity_mint
    );

    let source_liquidity = crate::derive_ata(&wallet, &liquidity_mint, &token_program_id);
    tracing::info!(
        "[save repay v3] source_liquidity_ata={} wallet={} amount={}",
        source_liquidity, wallet, liquidity_amount
    );

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
        AccountMeta::new_readonly(token_program_id, false),
    ];

    let repay_ix = Instruction {
        program_id: SAVE_PROGRAM_ID,
        accounts,
        data,
    };

    let pyth_oracle = read_pubkey(&account.data, RESERVE_PYTH_OFFSET)?;
    let switchboard_oracle = read_pubkey(&account.data, RESERVE_SWITCHBOARD_OFFSET)?;

    let refresh_ix = Instruction {
        program_id: SAVE_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(repay_reserve, false),
            AccountMeta::new_readonly(pyth_oracle, false),
            AccountMeta::new_readonly(switchboard_oracle, false),
            AccountMeta::new_readonly(sysvar::clock::ID, false),
        ],
        data: vec![3],
    };

    Ok(vec![refresh_ix, repay_ix])
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey, ExecutorError> {
    let slice: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or_else(|| ExecutorError::Decode(format!("save reserve: truncated at {offset}")))?
        .try_into()
        .map_err(|_| ExecutorError::Decode("save reserve: 32-byte pubkey slice".into()))?;
    Ok(Pubkey::new_from_array(slice))
}
