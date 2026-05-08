# Aegis — Runbook

How to boot the stack and verify the demo end-to-end. Read `STATUS.md` first for the architecture map.

---

## 0. Prereqs (one-time)

- Postgres 14+ running locally (`postgres://yuno:yuno@localhost:5432/aegis` per `.env`).
- Rust toolchain (workspace pinned via `rust-toolchain.toml` if present, otherwise stable).
- Node 20+ and npm/pnpm for the frontend.
- A Yellowstone gRPC endpoint (the `.env` uses Parafi's; works without API key for low volume).
- A Helius RPC key for `RPC_ENDPOINT`.
- Optional: a Telegram bot token (`TELEGRAM_BOT_TOKEN`) — without it the alert engine still runs, just without TG dispatch.

Create the DB: `createdb aegis`. Migrations run automatically on `aegis-server` boot.

---

## 1. Boot the backend

```bash
# from repo root
cargo run -p aegis-server
```

Expected startup log (truncated):
```
[boot] postgres connected
[boot] migrations applied
[boot] db writer online
[boot] rehydrated N monitored wallets from DB
[boot] oracle discovery complete: M mints
[boot] jupiter price poller online
[boot] kamino reserve refresh loop online
[boot] save reserve refresh loop online
[boot] marginfi bank refresh loop online
[boot] sanctum LST price poller online
[boot] api server online
[boot] alert engine online
[boot] heartbeat dispatcher spawned (interval=3h)
[boot] all subsystems up — starting gRPC supervisor
```

If Postgres credentials don't match `DATABASE_URL`, you'll fail at `[boot] postgres connected`. If gRPC is unreachable, the supervisor will retry — the API still serves.

The bot is its own binary:
```bash
cargo run -p aegis-bot
```

---

## 2. Boot the frontend

```bash
cd frontend
npm install   # first time only
npm run dev
```

Default vite URL is `http://localhost:5173`. It reads `VITE_API_URL` (default `http://localhost:7878`) and `VITE_DEMO_MODE` (default `false` from `.env.production`; for dev, leave unset and it stays `false`).

To preview the marketing/demo path without connecting a wallet, force `VITE_DEMO_MODE=true` in a local `.env` file inside `frontend/`.

---

## 3. End-to-end verification (the smoke test)

This is what "make E2E complete" means in practice. Run after a clean boot.

| # | Action | Expected | Where to look if it fails |
|---|---|---|---|
| 1 | Open `http://localhost:5173/`, click **Connect wallet**, approve | Nav shows the truncated address with a green dot | `aegis-api` `/api/wallets/:wallet` POST |
| 2 | Wait ~10s on `/dashboard` | Stats panel populates with real numbers (not the $3.6k/$2.0k mock) | `/api/health/:wallet` should be returning legs |
| 3 | Open `/positions` | Table shows the wallet's actual Kamino/Save/Marginfi legs | `aegis-indexer` parser logs |
| 4 | On Dashboard, click a `Repay` button on a Borrow row | Modal opens, "Build transaction" succeeds | `/api/intents/repay` |
| 5 | Click **Sign & submit** | Wallet popup appears, after signing the intent goes to `submitted` | `/api/intents/:id/submit` |
| 6 | In Telegram, message the bot `/start <code>` (code from Settings → "Link Telegram") | Bot replies with a confirmation, `/status` then shows wallet health | `aegis-bot` log |

If steps 1–3 work but 4–5 fail, the bug is in `aegis-executor` (most likely a missing PDA / authority check). If 6 fails the bug is in `aegis-bot::link_wallet_to_telegram` or the redeem endpoint.

---

## 4. Common ops

**Tail server logs at debug:**
```bash
RUST_LOG=info,aegis_indexer=debug,aegis_alerts=debug cargo run -p aegis-server
```

**Re-run a single migration:**
```bash
sqlx migrate revert
sqlx migrate run
```

**Force an alert to fire (manual test):**
```sql
UPDATE wallets SET monitored = true WHERE pubkey = '...';
INSERT INTO guard_rules (...) VALUES (... 'HealthBelow', 200 ...);
-- next poll cycle (POLL_INTERVAL_SECS) the engine will evaluate
```

**Frontend type-check before commit:**
```bash
cd frontend && npx tsc --noEmit
```

**Workspace build before commit:**
```bash
cargo build --workspace
```

---

## 5. Environment variables

`.env` at repo root (loaded by `aegis-core/src/config.rs`):

| Var | Purpose | Required? |
|---|---|---|
| `DATABASE_URL` | Postgres | yes |
| `GRPC_ENDPOINT` | Yellowstone | yes |
| `RPC_ENDPOINT` | Solana RPC for executor + reserve refresh | yes |
| `OPENAI_API_KEY` | OpenRouter (gpt-oss-120b:free) for AI alert summaries | yes for live LLM, optional otherwise |
| `TELEGRAM_BOT_TOKEN` | Telegram dispatcher + bot | optional |
| `ALERT_THRESHOLD` | Health below which alerts fire | default 30 |
| `POLL_INTERVAL_SECS` | Alert engine cadence | default 30 |
| `HEARTBEAT_INTERVAL_HOURS` | Periodic TG heartbeat | default 3 |
| `PORT_ADDRESS` | API listen port | default 7878 |

`frontend/.env.example` shows the minimal frontend set: `VITE_API_URL`, `VITE_SOLANA_RPC`, `VITE_DEMO_MODE`.

---

## 6. Deploy notes (deferred — out of MVP scope)

- API + indexer must run as one process (they share `Arc<AppState>`). Deploy `aegis-server` to one box.
- Bot is independent, deploy `aegis-bot` separately.
- Frontend is static — `npm run build` produces a `dist/` you can serve from anywhere with `VITE_API_URL` pointing at the deployed API.
- No production deploy has been attempted yet. Don't ship without rotating the keys in `.env`.
