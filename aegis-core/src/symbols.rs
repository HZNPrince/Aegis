//! Token symbol lookup — maps Solana mints to human-readable symbols.
//! Used by all three protocol parsers (Kamino, Marginfi, Save) in aegis-indexer to label positions,
//! and by the frontend for display. Unknown mints fall back to the first 4 characters of the mint address.
//!
//! Extend the symbol_for_mint match when adding support for new assets.

/// Lookup a known Solana token mint and return its ticker symbol, or None if unknown.
/// Returns a static string reference for zero-copy performance.
pub fn symbol_for_mint(mint: &str) -> Option<&'static str> {
    match mint {
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" => Some("USDC"),
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" => Some("USDT"),
        "So11111111111111111111111111111111111111112" => Some("SOL"),
        "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So" => Some("mSOL"),
        "bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1" => Some("bSOL"),
        "J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn" => Some("JitoSOL"),
        "jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v" => Some("jupSOL"),
        "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN" => Some("JUP"),
        "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263" => Some("BONK"),
        "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm" => Some("WIF"),
        "HZ1JovNiVvGrGNiiYvEozEVgZ58xaU3RKwX8eACQBCt3" => Some("PYTH"),
        "2u1tszSeqZ3qBWF3uNGPFc8TzMk2tdiwknnRMWGWjGWH" => Some("USDG"),
        _ => None,
    }
}

/// Human-readable label: known symbol or the first 4 chars of the mint if unknown.
/// Falls back gracefully to truncated mint addresses when a symbol is not known.
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
