# Aegis — DeFi Risk Copilot: Implementation Plan v2

An AI agent that monitors your Solana lending positions across all major protocols, explains risk in plain language, and autonomously executes protective actions based on rules you set.

---

## Confirmed Bugs & Fixes

> [!CAUTION]
> **Kamino scale factor bug**: `deposited_value_sf` and `borrow_factor_adjusted_debt_value_sf` use `2^60` (= `1_152_921_504_606_846_976`), NOT `10^18`. Our current code on line 116–118 of `grpc.rs` is **~13% off**. Fix: `const FRACTION_ONE_SCALED: u128 = 1u128 << 60;`

> [!NOTE]
> **Save (Solend) scale factor is CORRECT**: Verified from source — `pub const WAD: u64 = 1_000_000_000_000_000_000` in `token-lending/sdk/src/math/common.rs`. Our `10^18` divisor for Save is accurate.

---

## Architecture Overview

```
                         ┌──────────────────────────┐
                         │   Solana Mainnet          │
                         │  (Kamino, Save, Marginfi) │
                         └────────────┬─────────────┘
                                      │ Yellowstone gRPC (push, ~5-50ms)
                                      ▼
                    ┌─────────────────────────────────────┐
                    │          aegis-indexer               │
                    │                                     │
                    │  grpc.rs ──► ProtocolParser trait    │
                    │              ├─ KaminoParser         │
                    │              ├─ SaveParser           │
                    │              └─ MarginfiParser       │
                    │                     │                │
                    │         Arc<DashMap> (hot cache)     │
                    │           + AtomicU64 counters       │
                    │                     │                │
                    │         mpsc::channel (bounded 10k)  │
                    └──────┬──────────────┬───────────────┘
                           │              │
                    ┌──────▼──────┐ ┌─────▼──────────┐
                    │  Postgres   │ │  aegis-risk     │
                    │  (sqlx)     │ │  health.rs      │
                    │  batched    │ │  scenario.rs    │
                    │  upserts    │ └─────┬──────────┘
                    └──────┬──────┘       │
                           │              │
                    ┌──────▼──────────────▼───────────┐
                    │         aegis-alerts             │
                    │  llm.rs (free API) → engine.rs   │
                    └──────┬──────────────┬───────────┘
                           │              │
                    ┌──────▼──────┐ ┌─────▼──────────┐
                    │ aegis-bot   │ │  aegis-agent    │
                    │ (Telegram)  │ │  rule_engine.rs │
                    └─────────────┘ │  executor.rs    │
                                    │  (PDA delegate) │
                    ┌───────────────┴────────────────┐
                    │         aegis-api (Axum)        │
                    │  GET  /health/:wallet           │
                    │  POST /scenario                 │
                    │  GET  /alerts/:wallet           │
                    │  POST /guard-rules              │
                    │  POST /execute                  │
                    └────────────────────────────────┘
```

---

## Low-Latency Rust Patterns

These are the exact patterns used by production Solana indexers (Jito, Triton, Helius):

| Pattern | Where | Why | Latency |
|---------|-------|-----|---------|
| `Arc<DashMap<String, PositionUpdate>>` | Position cache | Lock-free concurrent reads. API + risk engine read without blocking the gRPC thread | ~50ns read |
| `tokio::sync::mpsc` (bounded 10k) | gRPC → DB writer | `try_send()` never blocks the hot path. If DB is slow, oldest updates drop — we always have fresh data in the cache anyway | ~100ns send |
| `AtomicU64` | Update counter, slot tracking | Zero-cost thread-safe counters. No mutex, no lock, single CPU instruction | ~1ns |
| `Arc<RwLock<HashMap>>` | Oracle price map | Many concurrent readers (risk engine, API), single writer (oracle poller every 5s) | ~20ns read |
| `trait ProtocolParser` | Indexer dispatch | Open/closed principle. Add Drift later without touching the hot loop | Zero overhead (static dispatch) |
| Batched DB writes | DB worker | Collect N updates in a `Vec`, then `INSERT ... VALUES (...), (...), (...)` in one roundtrip | 1 query per 100 updates |
| `#[inline]` on hot-path functions | Parsers | Hint to compiler to inline small parser functions, eliminating function call overhead | ~2ns saved per call |

### Why NOT crossbeam / flume / kanal?

`tokio::sync::mpsc` integrates natively with our async runtime. crossbeam channels are faster in pure sync benchmarks (~30ns vs ~100ns), but they require `spawn_blocking` to bridge into async code, which costs ~500ns. Net loss. We stay with tokio.

---

## Phase 1: Fix Bugs + Production Indexer Refactor

**Goal**: Fix the Kamino bug, introduce trait-based parsing, add `DashMap` cache, wire up the DB writer.

### 1.1 Fix Kamino Scale Factor

#### [MODIFY] [grpc.rs](file:///Users/aster27/Desktop/github/Side_Projects/Aegis/aegis-indexer/src/grpc.rs)
- Add `const FRACTION_ONE_SCALED: u128 = 1u128 << 60;` for Kamino
- Add `const WAD: u128 = 1_000_000_000_000_000_000;` for Save
- Replace hardcoded divisors with these constants
- Filter out zero-value obligations (empty accounts)

### 1.2 Introduce `ProtocolParser` Trait

#### [NEW] `aegis-indexer/src/parsers/mod.rs`
#### [NEW] `aegis-indexer/src/parsers/kamino.rs`
#### [NEW] `aegis-indexer/src/parsers/save.rs`
#### [NEW] `aegis-indexer/src/parsers/marginfi.rs`

Define the trait:
```rust
pub trait ProtocolParser: Send + Sync {
    fn program_id(&self) -> &str;
    fn can_parse(&self, data_len: usize) -> bool;
    fn parse(&self, pubkey: &str, data: &[u8], slot: u64) -> Option<PositionUpdate>;
}
```

Each protocol gets its own file with its own struct implementing this trait. The main gRPC loop becomes a clean iterator over `Vec<Box<dyn ProtocolParser>>`.

### 1.3 Shared State + DB Worker

#### [NEW] `aegis-indexer/src/state.rs`

```rust
pub struct AppState {
    pub positions: Arc<DashMap<String, PositionUpdate>>,
    pub update_count: AtomicU64,
    pub db_pool: PgPool,
}
```

#### [MODIFY] [grpc.rs](file:///Users/aster27/Desktop/github/Side_Projects/Aegis/aegis-indexer/src/grpc.rs)

- `start_account_stream` takes `Arc<AppState>` instead of just `&str`
- Hot path: parse → `DashMap::insert` → `tx.try_send()`
- Background task: `rx.recv()` → batch → `sqlx` upsert

#### [MODIFY] [test_stream.rs](file:///Users/aster27/Desktop/github/Side_Projects/Aegis/aegis-indexer/src/bin/test_stream.rs)

- Create `PgPool` from `DATABASE_URL`
- Build `AppState`
- Pass to `start_account_stream`

### 1.4 Database Migration

#### [NEW] `migrations/YYYYMMDD_add_agent_tables.sql`

Add tables: `guard_rules`, `alerts`, `executions` (for Phases 3–5).
Update `positions` table to add `last_slot` constraint for out-of-order protection.

---

## Phase 2: Oracle Engine + Risk Scoring

**Goal**: Dynamically discover all Pyth oracles from Reserve/Bank configs, poll prices, compute health scores.

### 2.1 Dynamic Oracle Discovery

#### [NEW] `aegis-indexer/src/oracle.rs`

On startup:
1. Fetch ALL Kamino `Reserve` accounts via RPC → extract `pyth_oracle` pubkey from each
2. Fetch ALL Marginfi `Bank` accounts via RPC → extract oracle pubkey
3. Build `HashMap<Pubkey, OracleInfo>` mapping reserve/bank → oracle → token symbol
4. Spawn background task: poll `get_multiple_accounts` every 5 seconds
5. Store prices in `Arc<RwLock<HashMap<Pubkey, f64>>>`

### 2.2 Health Score Engine

#### [NEW] `aegis-risk/src/health.rs`

- Read positions from `DashMap` cache
- Read prices from oracle cache
- Compute per-protocol LTV, aggregate portfolio health score (0–100)
- Compute liquidation buffer in USD

### 2.3 Scenario Simulator

#### [NEW] `aegis-risk/src/scenario.rs`

- Accept `HashMap<String, f64>` of price shocks (e.g., `{"SOL": -0.20}`)
- Clone current positions, apply shocked prices
- Return list of positions that would breach liquidation

---

## Phase 3: AI Brain — Free LLM Integration

**Goal**: Use a free LLM API to generate plain-English risk analysis and actionable suggestions.

### 3.1 LLM Integration

#### [NEW] `aegis-alerts/src/llm.rs`

- Use Groq API (free tier, fast inference) or HuggingFace Inference API
- Send structured portfolio state as prompt
- Parse response into `AlertPayload` with severity, summary, suggested actions
- Each action includes: what to do, estimated impact, plain English explanation

### 3.2 Alert Engine

#### [NEW] `aegis-alerts/src/engine.rs`

- Background task: poll `DashMap` every 30 seconds
- If health score < threshold → trigger LLM analysis
- Deduplication: cooldown window per wallet (stored in Postgres `alerts` table)
- Push to Telegram webhook

---

## Phase 4: API + One-Click Execution

**Goal**: REST API for dashboard, transaction builder for protective actions.

### 4.1 Axum REST API

#### [MODIFY] `aegis-api/src/lib.rs`

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health/:wallet` | GET | Health score + positions from DashMap cache |
| `/api/scenario` | POST | Run price shock simulation |
| `/api/alerts/:wallet` | GET | Alert history from Postgres |
| `/api/guard-rules` | POST | Create/update guard rules |
| `/api/execute` | POST | Submit pre-signed transaction |

### 4.2 Transaction Builder

#### [NEW] `aegis-api/src/tx_builder.rs`

Build unsigned transactions for:
- Add collateral (SPL token transfer to reserve)
- Partial repay (borrow repayment instruction)
- Swap via Jupiter API

Frontend shows preview → user signs with Phantom → backend submits signed tx.

---

## Phase 5: Autonomous Guardrails (Full Build)

**Goal**: PDA-based delegated authority with on-chain hard caps for autonomous protective actions.

### 5.1 Guard Rules

#### [NEW CRATE] `aegis-agent/`

```rust
pub struct GuardRule {
    pub wallet: Pubkey,
    pub protocol: Protocol,
    pub trigger: Trigger,         // HealthBelow(1.2), PriceDrop(0.15)
    pub action: ProtectiveAction, // AddCollateral { token, max_usd }
    pub max_usd_per_action: f64,
    pub daily_limit_usd: f64,
    pub is_active: bool,
}
```

### 5.2 Rule Engine

#### [NEW] `aegis-agent/src/rule_engine.rs`

Background `tokio::spawn` task:
1. Poll `DashMap` cache every 30 seconds
2. Evaluate all active `GuardRule`s against current positions
3. When triggered: build tx → sign with delegated authority → submit
4. Log execution to `executions` table
5. Send Telegram notification

### 5.3 On-Chain Delegated Authority

#### [NEW] Anchor program (separate repo or workspace member)

- User delegates SPL token spending to an Aegis PDA
- PDA enforces: max per-action, daily aggregate limit
- User can revoke delegation at any time (kill switch)
- Aegis backend holds a hot wallet keypair that can invoke the PDA
- The PDA program validates every action against the user's rules before executing

---

## Phase 6: Dashboard + Telegram Bot

### 6.1 Next.js Dashboard

- Wallet connect (Solana wallet-adapter)
- Health score gauge (color-coded green/yellow/red)
- Per-protocol positions table with LTV progress bars
- Scenario simulator form
- Guard rule builder UI
- Alert history timeline
- One-click action buttons with transaction preview

### 6.2 Telegram Bot (teloxide)

- `/watch <wallet>` — register for monitoring
- `/status` — current health + positions
- `/scenario SOL -20%` — simulate
- `/guard add health<1.2 add_collateral 300 USDC`
- `/guard list` / `/guard pause` / `/guard kill` — manage

---

## Testing & Benchmarking

### Unit Tests
- **Parsers**: Save fixture bytes → verify correct USD values with WAD divisor
- **Parsers**: Kamino fixture bytes → verify correct USD values with 2^60 divisor
- **Health**: Known positions → expected health scores
- **Scenario**: Known positions + known shocks → expected breaches

### Integration Tests
- Spin up test Postgres → run migrations → verify upsert + dedup logic
- Connect to mainnet gRPC for 30s → verify parse counts across all 3 protocols

### Benchmarks (`criterion` crate)
- `Obligation::from_bytes` throughput (target: >1M/sec)
- `DashMap` concurrent read/write (target: >10M ops/sec)
- Full parse pipeline: raw bytes → `PositionUpdate` struct (target: <1μs)
- Channel throughput: events/sec before backpressure kicks in

---

## Open Questions

> [!IMPORTANT]
> 1. **Free LLM**: Which free API do you prefer? Options: Groq (fast, free tier), HuggingFace Inference (fully free), or self-hosted Ollama (local, no rate limits)?
> 2. **On-chain program**: Should the Anchor delegation program live inside this workspace, or as a separate repo?
