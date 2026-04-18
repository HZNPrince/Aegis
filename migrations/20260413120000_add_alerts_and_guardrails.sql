CREATE TABLE IF NOT EXISTS guard_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_pubkey VARCHAR(44) NOT NULL REFERENCES wallets(pubkey) ON DELETE CASCADE,
    protocol VARCHAR(20),
    trigger_kind VARCHAR(32) NOT NULL,
    trigger_value DOUBLE PRECISION NOT NULL,
    action_kind VARCHAR(32) NOT NULL,
    action_token VARCHAR(64),
    action_amount_usd DOUBLE PRECISION,
    max_usd_per_action DOUBLE PRECISION NOT NULL,
    daily_limit_usd DOUBLE PRECISION NOT NULL,
    cooldown_seconds BIGINT NOT NULL DEFAULT 3600,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_guard_rules_wallet ON guard_rules(wallet_pubkey);

CREATE TABLE IF NOT EXISTS alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_pubkey VARCHAR(44) NOT NULL REFERENCES wallets(pubkey) ON DELETE CASCADE,
    severity VARCHAR(16) NOT NULL,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    health_score DOUBLE PRECISION NOT NULL,
    ltv DOUBLE PRECISION NOT NULL,
    suggested_actions JSONB NOT NULL DEFAULT '[]'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_alerts_wallet_created_at ON alerts(wallet_pubkey, created_at DESC);

CREATE TABLE IF NOT EXISTS executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_pubkey VARCHAR(44) NOT NULL REFERENCES wallets(pubkey) ON DELETE CASCADE,
    guard_rule_id UUID REFERENCES guard_rules(id) ON DELETE SET NULL,
    action_kind VARCHAR(32) NOT NULL,
    action_token VARCHAR(64),
    action_amount_usd DOUBLE PRECISION,
    status VARCHAR(24) NOT NULL,
    tx_hash VARCHAR(128),
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_executions_wallet_created_at ON executions(wallet_pubkey, created_at DESC);
