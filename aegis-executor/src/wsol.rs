use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

pub const WSOL_MINT: Pubkey = solana_sdk::pubkey!("So11111111111111111111111111111111111111112");
pub const SPL_TOKEN_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const SPL_ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

/// Wrap native SOL by building the instructions to:
/// 1. Create the WSOL ATA idempotently.
/// 2. Transfer SOL from wallet to the WSOL ATA.
/// 3. SyncNative to wrap the transferred SOL.
pub fn build_wsol_wrap_ixs(
    wallet: Pubkey,
    wsol_ata: Pubkey,
    amount_native: u64,
) -> Vec<Instruction> {
    let create_ata_ix = Instruction {
        program_id: SPL_ASSOCIATED_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(wallet, true),
            AccountMeta::new(wsol_ata, false),
            AccountMeta::new_readonly(wallet, false),
            AccountMeta::new_readonly(WSOL_MINT, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(SPL_TOKEN_PROGRAM_ID, false),
        ],
        // create_idempotent instruction data for spl_associated_token_account
        data: vec![1],
    };

    let mut transfer_data = vec![2, 0, 0, 0]; // system_instruction::Transfer discriminator
    transfer_data.extend_from_slice(&amount_native.to_le_bytes());

    let transfer_ix = Instruction {
        program_id: SYSTEM_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(wallet, true),
            AccountMeta::new(wsol_ata, false),
        ],
        data: transfer_data,
    };

    let sync_native_ix = Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![AccountMeta::new(wsol_ata, false)],
        // sync_native is instruction index 17
        data: vec![17],
    };

    vec![create_ata_ix, transfer_ix, sync_native_ix]
}

/// Close the WSOL account to return rent and any unspent dust to the wallet.
pub fn build_wsol_close_ix(wallet: Pubkey, wsol_ata: Pubkey) -> Instruction {
    Instruction {
        program_id: SPL_TOKEN_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(wsol_ata, false),
            AccountMeta::new(wallet, false),
            AccountMeta::new_readonly(wallet, true),
        ],
        // close_account is instruction index 9
        data: vec![9],
    }
}
