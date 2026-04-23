//! Minimal mint → symbol lookup for the common Solana lending assets.
//!
//! Kept deliberately small: the executor and the frontend only need to
//! display something human-readable for the tokens guard-rule actions can
//! target. Unknown mints fall back to a truncated pubkey.
//!
//! Extend this when adding support for a new asset.

/// Return a short symbol for a known mint, or `None` if unknown.
pub fn symbol_for_mint(mint: &str) -> Option<&'static str> {
    match mint {
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => Some("USDC"),
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => Some("USDT"),
        "So11111111111111111111111111111111111111112" => Some("SOL"),
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => Some("mSOL"),
        "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1" => Some("bSOL"),
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => Some("JitoSOL"),
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN" => Some("JUP"),
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => Some("BONK"),
        _ => None,
    }
}

/// Human-readable label: known symbol or the first 4 chars of the mint.
pub fn symbol_or_short(mint: &str) -> String {
    if let Some(sym) = symbol_for_mint(mint) {
        return sym.to_string();
    }
    if mint.len() >= 4 {
        mint[..4].to_string()
    } else {
        mint.to_string()
    }
}
