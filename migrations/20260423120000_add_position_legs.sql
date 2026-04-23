CREATE TABLE IF NOT EXISTS position_legs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    obligation_pubkey VARCHAR(44) NOT NULL REFERENCES positions(obligation_pubkey) ON DELETE CASCADE,
    side VARCHAR(16) NOT NULL, -- 'Collateral' | 'Borrow'
    asset_mint VARCHAR(44) NOT NULL,
    asset_symbol VARCHAR(16) NOT NULL,
    amount_native BIGINT NOT NULL, -- native token units; i64 easily covers per-position amounts across all supported assets
    amount_ui DOUBLE PRECISION NOT NULL,
    value_usd DOUBLE PRECISION NOT NULL,
    reserve_or_bank VARCHAR(44) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_position_legs_obligation ON position_legs(obligation_pubkey);
CREATE INDEX IF NOT EXISTS idx_position_legs_mint ON position_legs(asset_mint);
