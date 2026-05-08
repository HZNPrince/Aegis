//! Kamino (KLend) repay instruction builder — constructs RepayObligationLiquidity transaction.
//! Fetches Reserve account from RPC to extract lending_market, supply_vault, and token_program.
//! Used by: executor's build_repay_tx() for Kamino protocol repayments.

use crate::{derive_ata, ExecutorError};
use klend_sdk::{
    accounts::Reserve,
    instructions::RepayObligationLiquidityBuilder,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

pub async fn build_repay_ix(
    rpc: &RpcClient,
    wallet: Pubkey,
    obligation: Pubkey,
    repay_reserve: Pubkey,
    _expected_mint: Pubkey,
    liquidity_amount: u64,
) -> Result<Vec<Instruction>, ExecutorError> {
    let account = rpc
        .get_account(&repay_reserve)
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("reserve {repay_reserve}: {e}")))?;

    let reserve = Reserve::from_bytes(&account.data)
        .map_err(|e| ExecutorError::Decode(format!("kamino reserve: {e}")))?;

    let lending_market = reserve.lending_market;
    let liquidity_mint = reserve.liquidity.mint_pubkey;
    let destination_liquidity = reserve.liquidity.supply_vault;
    let token_program = reserve.liquidity.token_program;

    let user_source_liquidity = derive_ata(&wallet, &liquidity_mint, &token_program);

    let ix = RepayObligationLiquidityBuilder::new()
        .owner(wallet)
        .obligation(obligation)
        .lending_market(lending_market)
        .repay_reserve(repay_reserve)
        .reserve_liquidity_mint(liquidity_mint)
        .reserve_destination_liquidity(destination_liquidity)
        .user_source_liquidity(user_source_liquidity)
        .token_program(token_program)
        .liquidity_amount(liquidity_amount)
        .instruction();

    Ok(vec![ix])
}
