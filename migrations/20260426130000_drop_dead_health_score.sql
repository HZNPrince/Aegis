-- P3-1: wallets.health_score is never written to (health is always computed live).
-- Dropping it avoids confusion when introspecting the schema.
ALTER TABLE wallets DROP COLUMN IF EXISTS health_score;
