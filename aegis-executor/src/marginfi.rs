//! Marginfi v2 repay instruction builder — constructs LendingAccountRepay transaction.
//! Signer must be the MarginfiAccount.authority (not delegate-capable for Aegis).
//! Parses Bank layout at fixed offsets to extract mint, group, liquidity_vault, and token_program.
//! Used by: executor's build_repay_tx() for Marginfi protocol repayments.

use crate::{derive_ata, ExecutorError};
use borsh::BorshSerialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

// Bank layout after the 8-byte anchor discriminator (carbon-marginfi-v2-decoder
// 0.12 confirms the field order; we only need the first three fixed-size
// fields, so we don't deserialize BankConfig — it has on-chain enums
// (OracleSetup / RiskTier / OperationalState) whose newer variants are not
// recognized by the 0.12 schema and cause "Unexpected variant index" errors).
//
//   [0..32]    mint
//   [32]       mint_decimals
//   [33..65]   group
//   [65..72]   pad (7)
//   [72..88]   asset_share_value (i128)
//   [88..104]  liability_share_value (i128)
//   [104..136] liquidity_vault
const DISC_LEN: usize = 8;
const BANK_MINT_OFFSET: usize = DISC_LEN; // 8
const BANK_GROUP_OFFSET: usize = DISC_LEN + 33; // 41
const BANK_LIQUIDITY_VAULT_OFFSET: usize = DISC_LEN + 104; // 112
const BANK_MIN_LEN: usize = BANK_LIQUIDITY_VAULT_OFFSET + 32;

/// From carbon-marginfi-v2-decoder::instructions::LendingAccountRepay.
/// Verified at research time against the decoder's `#[carbon(discriminator)]`
/// attribute (hex `0x4fd1acb1de33ad97`, LE interpretation = these 8 bytes).
const DISCRIMINATOR: [u8; 8] = [0x4f, 0xd1, 0xac, 0xb1, 0xde, 0x33, 0xad, 0x97];

const MARGINFI_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("MFv2hWf31Z9kbCa1snEPYctwafyhdvnV7FZnsebVacA");

/// Marginfi banks may be either legacy SPL Token or Token-2022 — the bank's
/// mint owner is authoritative. We fetch the mint and use its `owner`
/// (which is the program that controls the mint) for both ATA derivation
/// and accounts[6]. Hardcoding either program ID breaks any bank using
/// the other.

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
    expected_mint: Pubkey,
    amount: u64,
    repay_all: bool,
) -> Result<Vec<Instruction>, ExecutorError> {
    let account = rpc
        .get_account(&bank_pk)
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("bank {bank_pk}: {e}")))?;

    if account.data.len() < BANK_MIN_LEN {
        return Err(ExecutorError::Decode(format!(
            "marginfi bank: expected ≥{BANK_MIN_LEN} bytes, got {}",
            account.data.len()
        )));
    }
    let mint = read_pubkey(&account.data, BANK_MINT_OFFSET)?;
    let marginfi_group = read_pubkey(&account.data, BANK_GROUP_OFFSET)?;
    let liquidity_vault = read_pubkey(&account.data, BANK_LIQUIDITY_VAULT_OFFSET)?;

    if mint != expected_mint {
        return Err(ExecutorError::Guardrail(format!(
            "marginfi bank mint {} != expected {}",
            mint, expected_mint
        )));
    }

    // Fetch the mint to learn which token program (legacy vs Token-2022)
    // owns it. SPL Token's Transfer CPI fails with InvalidAccountData if we
    // route a Token-2022 account through legacy SPL Token (or vice versa).
    let mint_account = rpc
        .get_account(&mint)
        .await
        .map_err(|e| ExecutorError::RpcFetch(format!("mint {mint}: {e}")))?;
    let token_program_id = mint_account.owner;

    tracing::info!(
        "[marginfi repay] group={} liquidity_vault={} mint={} token_program={}",
        marginfi_group, liquidity_vault, mint, token_program_id
    );

    let signer_token_account = derive_ata(&wallet, &mint, &token_program_id);

    let mut data = Vec::with_capacity(8 + 9);
    data.extend_from_slice(&DISCRIMINATOR);
    let args = RepayArgs {
        amount,
        repay_all: if repay_all { Some(true) } else { None },
    };
    args.serialize(&mut data)
        .map_err(|e| ExecutorError::Decode(format!("serialize repay args: {e}")))?;

    let mut accounts = vec![
        AccountMeta::new_readonly(marginfi_group, false),
        AccountMeta::new(marginfi_account, false),
        AccountMeta::new_readonly(wallet, true),
        AccountMeta::new(bank_pk, false),
        AccountMeta::new(signer_token_account, false),
        AccountMeta::new(liquidity_vault, false),
        AccountMeta::new_readonly(token_program_id, false),
    ];

    // Token-2022 banks require the mint as the first "remaining account"
    // (anchor error 6044 / T22MintRequired). Legacy SPL Token banks must NOT
    // receive it. Detect by token program owner of the mint.
    const TOKEN_2022_PROGRAM_ID: Pubkey =
        solana_sdk::pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
    if token_program_id == TOKEN_2022_PROGRAM_ID {
        accounts.push(AccountMeta::new_readonly(mint, false));
    }

    Ok(vec![Instruction {
        program_id: MARGINFI_PROGRAM_ID,
        accounts,
        data,
    }])
}

fn read_pubkey(data: &[u8], offset: usize) -> Result<Pubkey, ExecutorError> {
    let slice: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or_else(|| ExecutorError::Decode(format!("marginfi bank: truncated at {offset}")))?
        .try_into()
        .map_err(|_| ExecutorError::Decode("marginfi bank: 32-byte pubkey slice".into()))?;
    Ok(Pubkey::new_from_array(slice))
}
