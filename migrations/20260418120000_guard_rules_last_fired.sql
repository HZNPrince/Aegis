ALTER TABLE guard_rules
    ADD COLUMN IF NOT EXISTS last_fired_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_guard_rules_active_wallet
    ON guard_rules(wallet_pubkey)
    WHERE is_active = true;
