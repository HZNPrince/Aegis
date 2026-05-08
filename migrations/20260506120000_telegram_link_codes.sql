CREATE TABLE IF NOT EXISTS telegram_link_codes (
    code          TEXT        PRIMARY KEY,
    wallet_pubkey TEXT        NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS telegram_link_codes_wallet_idx
    ON telegram_link_codes (wallet_pubkey);
