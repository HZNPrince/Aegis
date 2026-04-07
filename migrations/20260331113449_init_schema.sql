CREATE TABLE IF NOT EXISTS wallets (
    pubkey VARCHAR(44) PRIMARY KEY,
    health_score NUMERIC(5,2), -- Global health score (0.00 to 100.00)
    is_monitored BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS positions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_pubkey VARCHAR(44) NOT NULL REFERENCES wallets(pubkey) ON DELETE CASCADE,
    obligation_pubkey VARCHAR(44) UNIQUE NOT NULL,
    protocol VARCHAR(20) NOT NULL, --'Kamino', 'Save', 'Marginfi'
    collateral_usd NUMERIC NOT NULL DEFAULT 0,
    debt_usd NUMERIC NOT NULL DEFAULT 0,
    last_slot BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_positions_wallet ON positions(wallet_pubkey);