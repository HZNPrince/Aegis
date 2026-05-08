-- P2-3: Add email column for future email notification delivery.
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS email TEXT;
