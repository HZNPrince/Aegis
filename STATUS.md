# Aegis — Status & Agent Handoff

**Single source of truth for what's done, what's left, and how agents stay aligned.**
Last updated: 2026-05-07. Update the **Status** column whenever you complete something so the next agent doesn't re-survey from scratch.

---

## 1. Architecture (1-minute orientation)

```
┌──────────────────┐     gRPC      ┌──────────────────┐
│ Yellowstone gRPC │ ────────────▶ │ aegis-indexer    │
│ (Parafi)         │   accounts    │ - parsers        │
└──────────────────┘               │ - oracle pollers │
                                   │ - db writer      │
                                   └──────┬───────────┘
                                          │ AppState (DashMap caches) + Postgres
                                          ▼
┌──────────────────┐  HTTP   ┌────────────────────────────────────┐
│ frontend (Vite)  │◀───────▶│ aegis-api (Axum, port 7878)        │
│ React + RQ       │   JSON  │ /api/health, /api/alerts, /api/... │
└──────────────────┘         └────────────┬───────────────────────┘
                                          │ shares Arc<AppState>
                                          ▼
                              ┌────────────────────────────────┐
                              │ aegis-alerts (engine + heartbeat)│
                              │ - LogDispatcher                  │
                              │ - TelegramDispatcher             │
                              └────────────┬───────────────────┘
                                          │ teloxide
                                          ▼
                                 ┌──────────────────┐
                                 │ aegis-bot (TG)   │
                                 │ /link /status…   │
                                 └──────────────────┘
```

**Crates** (cargo workspace):
- `aegis-core` — shared `AppState`, types, config.
- `aegis-indexer` — gRPC stream, account parsers (Kamino, Save, Marginfi), Jupiter/Sanctum price pollers, db writer.
- `aegis-risk` — health-score math + scenario shock simulation.
- `aegis-alerts` — alert engine (rule eval), dispatchers (log + Telegram), heartbeat loop.
- `aegis-executor` — builds **unsigned** repay txs, persists `execution_intents`, submits signed bytes.
- `aegis-api` — Axum HTTP server (CORS open, 18 routes, see §3).
- `aegis-server` — binary that boots everything in the right order. **Run target.**
- `aegis-bot` — teloxide Telegram bot. Standalone binary.
- `frontend/` — Vite + React + TanStack Query. `VITE_API_URL` + `VITE_DEMO_MODE`.

---

## 2. Status by feature

| Feature | Status | Location | Notes |
|---|---|---|---|
| Postgres schema (10 migrations) | ✅ done | `migrations/` | Latest: `20260506120000_telegram_link_codes.sql` |
| gRPC account stream + supervisor | ✅ done | `aegis-indexer/src/grpc.rs` | Parafi endpoint, auto-reconnect |
| Kamino / Save / Marginfi parsers | ✅ done | `aegis-indexer/src/parsers/` | Per-asset legs emitted |
| Oracle: Jupiter prices | ✅ done | `aegis-indexer/src/oracle.rs` | Polls discovered mints |
| Oracle: Sanctum LST | ✅ done | same | LST conversion factors |
| Oracle: bank/reserve refresh | ✅ done | same | Per-protocol loops |
| DB writer (mpsc, bounded 1000) | ✅ done | `aegis-indexer/src/writer.rs` | Drops on backpressure (intentional) |
| Risk engine (weighted health) | ✅ done | `aegis-risk/src/health.rs` | |
| Scenario simulation | ✅ done | `aegis-risk/src/scenario.rs` | Wired to `/api/scenario` |
| Alert engine (rule eval + cooldowns) | ✅ done | `aegis-alerts/src/engine.rs` | |
| Telegram dispatcher | ✅ done | `aegis-alerts/src/dispatch.rs` | Uses `TELEGRAM_BOT_TOKEN` |
| Heartbeat dispatcher | ✅ done | `aegis-alerts/src/heartbeat.rs` | Hourly summary |
| API: 18 routes | ✅ done | `aegis-api/src/lib.rs` | See §3 |
| Repay intent: build unsigned tx | ✅ done | `aegis-executor/src/save.rs` (Save), Kamino in `lib.rs` | Permissionless for Kamino+Save |
| Repay intent: submit signed | ✅ done | `aegis-executor/src/lib.rs` | Persists `execution_intents` row |
| Telegram bot: /link, /status, /health | ✅ done | `aegis-bot/src/main.rs` | Uses link-code redemption |
| Frontend: typed API client | ✅ done | `frontend/src/api.ts` | |
| Frontend: hooks (TanStack Query) | ✅ done | `frontend/src/hooks.ts` | All endpoints covered |
| Frontend: live/demo split | ✅ done | `useLive = !DEMO_MODE && wallet` | All pages respect it |
| Frontend: dashboard, positions, alerts, guardrails, telegram, settings | ✅ done | `frontend/src/pages/` | |
| Frontend: RepayModal (build → sign → submit) | ✅ done | `frontend/src/components/RepayModal.tsx` | Wallet adapter signs versioned tx |
| Frontend: dark theme + view-transition wave | ✅ done | `frontend/src/index.css` + `Nav.tsx` | |
| Frontend: real DefiLlama protocol logos | ✅ done | `frontend/src/components/sonar.tsx` | |

### Verified
- `cargo build --workspace` — ✅ clean (4 teloxide deprecation warnings only, see §5).
- `cd frontend && npx tsc --noEmit` — ✅ exit 0.
- All migrations applied automatically by `aegis-server` on boot.

### Not yet verified at runtime
- E2E demo against a real connected wallet on mainnet. The static plumbing is in place but no recorded smoke test. This is the next concrete TODO — see §4.

---

## 3. API surface (18 endpoints)

All defined in `aegis-api/src/lib.rs` and consumed by `frontend/src/api.ts`. Every route is covered by a TanStack hook in `frontend/src/hooks.ts`.

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/status` | Cache sizes (debugging only) |
| GET | `/api/prices` | Mint → USD price |
| GET | `/api/ticker` | Mint → `{price, change_24h}` |
| GET | `/api/wallets/:wallet` | Wallet settings (telegram chat id, etc.) |
| POST | `/api/wallets/:wallet` | Link wallet + backfill positions |
| GET | `/api/health/:wallet` | Wallet risk + position legs |
| POST | `/api/scenario` | Shocked simulation |
| GET | `/api/alerts/:wallet` | Persisted alert history |
| GET | `/api/wallets/:wallet/guard-rules` | List guard rules |
| POST | `/api/guard-rules` | Create or update |
| DELETE | `/api/guard-rules/:id` | Delete |
| GET | `/api/wallets/by-chat/:chat_id` | Reverse lookup (used by bot) |
| PATCH | `/api/wallets/:wallet/telegram` | Link chat id |
| DELETE | `/api/wallets/:wallet/telegram` | Unlink |
| POST | `/api/wallets/:wallet/telegram/code` | Issue one-time code |
| POST | `/api/telegram/redeem` | Bot redeems code |
| PATCH | `/api/wallets/:wallet/email` | Link email |
| POST | `/api/intents/repay` | Build unsigned repay tx + persist intent |
| POST | `/api/intents/:id/submit` | Submit signed bytes |
| GET | `/api/intents/:id` | Intent status |

---

## 4. Outstanding work

### Tier A — finish-line for "demo-able E2E" (do these next)
1. **Smoke-test the live path end-to-end.** Boot `aegis-server`, connect a real wallet on the frontend with `VITE_DEMO_MODE=false`, confirm:
   - `/api/health/:wallet` returns indexed legs
   - alerts appear in Postgres `alerts` table after a price move (or a forced threshold)
   - `/api/intents/repay` produces a tx that the wallet can actually sign
   - Telegram `/link` from the bot redeems the code and `/status` returns the wallet's health
2. **Migrate teloxide to `MarkdownV2`.** `aegis-bot/src/main.rs:299, 335, 346`. Markdown v1 is deprecated and will be removed; the project compiles today only because of warnings.
3. **Production env hygiene.** `.env` at repo root currently contains a real `OPENAI_API_KEY` and `TELEGRAM_BOT_TOKEN`. Rotate both before any public push, and confirm `.env` is gitignored.

### Tier B — polish items the user has paused on
4. **Hero radar fine-tuning.** Latest design is fine; if the user revisits, sweep colors are cobalt / moss / amber / rust per ring.
5. **Toggle wave fallback for Safari < 18.** Currently degrades to instant theme swap if `document.startViewTransition` is missing — explicitly tested via `matchMedia('(prefers-reduced-motion)')`.

### Tier C — known low-priority debt
6. `aegis-risk` warnings about deprecated `solana-sdk` chrono compat (none currently visible but watch as Solana 4.x churns).
7. `MOCK_GUARD_RULES` / `MOCK_STATUS` still imported on Settings + GuardRules pages as fallbacks. They are gated by `useLive`, so harmless in production but candidates for removal once the live path is smoke-tested.
8. No README at repo root. `brand.md` is the only top-level doc apart from this file.

---

## 5. Conventions for agents

**When you finish something on this list:** flip its row in §2 from in-progress to ✅ and add a one-line note. When you find a new gap, append to §4 in the right tier.

**File-pointer style.** When a doc references code, give a `path/to/file.rs:line` so the next agent doesn't have to grep. Keep links to lines, not whole functions.

**Don't add features for hypotheticals.** This is a hackathon MVP for Colosseum Frontier (Dodo Payments track), deadline end of April 2026. Every yak shaved is a yak not delivered.

**Telegram & API keys.** `.env` at root is the source of truth for local dev. Frontend reads `frontend/.env.production` only for production builds.

**Dark-theme contract.** All page colors must come from CSS vars (`var(--ink)`, `var(--paper)`, `var(--surface-1/2/3)`, `var(--shadow-*)`). Hard-coded `#fff` or `rgba(26,26,26,...)` values will break the dark theme. Use `color-mix(in oklab, var(--ink) X%, transparent)` for muted text.

**Demo-mode contract.** Pages must respect `useLive = !DEMO_MODE && wallet`. When false, render `MOCK_*` data unchanged. When true, derive from API hooks via `walletRiskToHealth(...)` / `alertWireToAlert(...)` adapters in `frontend/src/utils.ts`.

**Repay-authority by protocol.** Memorized but worth restating: Kamino + Save are permissionless (any signer can repay), Marginfi requires the account authority. RepayModal already surfaces this in the UI; don't strip it out.

---

## 6. Where to look next

- `RUNBOOK.md` — how to boot the stack from scratch.
- `AEGIS_MVP_PLAN.md` — does **not** exist. (Earlier auto-memory mentioned it; that memory is stale.) This file replaces it.
- `brand.md` — colors, typography, voice. Frontend uses it as source of truth.
