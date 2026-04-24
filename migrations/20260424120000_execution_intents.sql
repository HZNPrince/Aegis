-- execution_intents: one row per tripped RepayDebt rule (or explicit user
-- click) that produced an unsigned tx. Lifecycle:
--   pending   -> unsigned tx minted, shown to the user
--   signed    -> wallet returned a signed tx (frontend recorded the sig)
--   submitted -> relayed to a cluster, awaiting confirmation
--   confirmed -> landed on-chain
--   expired   -> blockhash TTL elapsed before the user signed
--   cancelled -> user dismissed or a newer intent supersedes it
CREATE TABLE IF NOT EXISTS execution_intents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_pubkey VARCHAR(44) NOT NULL REFERENCES wallets(pubkey) ON DELETE CASCADE,
    guard_rule_id UUID REFERENCES guard_rules(id) ON DELETE SET NULL,
    protocol VARCHAR(20) NOT NULL,
    obligation_or_account VARCHAR(44) NOT NULL,
    reserve_or_bank VARCHAR(44) NOT NULL,
    mint VARCHAR(44) NOT NULL,
    amount_native BIGINT NOT NULL,
    unsigned_tx TEXT NOT NULL,
    last_valid_block_height BIGINT NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'pending',
    signature VARCHAR(128),
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_execution_intents_wallet_status
    ON execution_intents(wallet_pubkey, status, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_execution_intents_pending_expiry
    ON execution_intents(expires_at)
    WHERE status = 'pending';
