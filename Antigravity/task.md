# Aegis — DeFi Risk Copilot: Task Tracker

## Week 1 (Completed)
- [x] Day 1: Rust workspace structure + Cargo.toml setup
- [x] Day 2: Yellowstone gRPC connection
- [x] Day 3: Deserializing Kamino obligation accounts
- [x] Day 4: Deserializing Save (Solend) obligation accounts
- [x] Day 5: Deserializing Marginfi accounts
- [x] Day 7: Postgres schema + sqlx setup

---

## Phase 1: Fix Bugs + Production Indexer Refactor
- [ ] Fix Kamino scale factor: replace `10^18` with `1u128 << 60`
- [ ] Add `FRACTION_ONE_SCALED` and `WAD` constants
- [ ] Filter out zero-value obligations (Kamino + Save)
- [ ] Create `aegis-indexer/src/parsers/mod.rs` — define `ProtocolParser` trait
- [ ] Create `aegis-indexer/src/parsers/kamino.rs` — implement `KaminoParser`
- [ ] Create `aegis-indexer/src/parsers/save.rs` — implement `SaveParser`
- [ ] Create `aegis-indexer/src/parsers/marginfi.rs` — implement `MarginfiParser`
- [ ] Create `aegis-indexer/src/state.rs` — `AppState` with `DashMap` + `AtomicU64`
- [ ] Refactor `grpc.rs` — trait-based dispatch loop
- [ ] Add bounded `mpsc` channel + background DB writer task
- [ ] Batch DB upserts (collect N, single INSERT)
- [ ] Update `test_stream.rs` — create `PgPool`, build `AppState`
- [ ] Add `dashmap` + `sqlx` to `aegis-indexer/Cargo.toml`
- [ ] New migration: `guard_rules`, `alerts`, `executions` tables
- [ ] Verify: run `test-stream`, confirm correct USD values in logs AND Postgres

## Phase 2: Oracle Engine + Risk Scoring
- [ ] Create `aegis-indexer/src/oracle.rs` — dynamic oracle discovery
- [ ] Fetch all Kamino Reserves → extract `pyth_oracle` pubkeys
- [ ] Fetch all Marginfi Banks → extract oracle pubkeys
- [ ] Background oracle poller (every 5s) with `Arc<RwLock<HashMap>>`
- [ ] Create `aegis-risk/src/health.rs` — health score computation
- [ ] Create `aegis-risk/src/scenario.rs` — price shock simulator
- [ ] Unit tests: health score math with known inputs
- [ ] Unit tests: scenario simulation with known shocks

## Phase 3: AI Brain — Free LLM Integration
- [ ] Create `aegis-alerts/src/llm.rs` — free LLM API client (Groq/HuggingFace)
- [ ] Design prompt template for portfolio risk analysis
- [ ] Parse LLM response into `AlertPayload` struct
- [ ] Create `aegis-alerts/src/engine.rs` — alert poller + deduplication
- [ ] Store alerts in Postgres with cooldown window
- [ ] Unit tests: prompt construction + response parsing (mock HTTP)

## Phase 4: API + One-Click Execution
- [ ] Create `aegis-api/src/lib.rs` — Axum server setup
- [ ] `GET /api/health/:wallet` — reads from DashMap cache
- [ ] `POST /api/scenario` — runs price shock
- [ ] `GET /api/alerts/:wallet` — reads from Postgres
- [ ] `POST /api/guard-rules` — CRUD for guard rules
- [ ] Create `aegis-api/src/tx_builder.rs` — transaction builder
- [ ] Add collateral transaction template
- [ ] Partial repay transaction template
- [ ] Jupiter swap integration
- [ ] `POST /api/execute` — submit pre-signed transaction

## Phase 5: Autonomous Guardrails (Full Build)
- [ ] Create `aegis-agent/` crate
- [ ] Define `GuardRule` + `Trigger` + `ProtectiveAction` types
- [ ] Create `aegis-agent/src/rule_engine.rs` — background evaluator
- [ ] Create `aegis-agent/src/executor.rs` — tx builder + submitter
- [ ] Design on-chain PDA delegation program (Anchor)
- [ ] Implement PDA vault with per-action + daily limits
- [ ] Implement kill switch (instant revocation)
- [ ] Integration test: rule trigger → tx execution → log to DB

## Phase 6: Dashboard + Telegram Bot
- [ ] Next.js project setup
- [ ] Wallet connect integration
- [ ] Health score gauge (color-coded)
- [ ] Per-protocol positions table with LTV bars
- [ ] Scenario simulator form + results
- [ ] Guard rule builder UI
- [ ] Alert history timeline
- [ ] One-click action buttons with tx preview
- [ ] Telegram bot: `/watch`, `/status`, `/scenario`
- [ ] Telegram bot: `/guard add`, `/guard list`, `/guard pause`

## Testing & Benchmarking
- [ ] Unit tests: parser fixtures (raw bytes → correct USD values)
- [ ] Unit tests: health score math
- [ ] Integration tests: Postgres upsert + dedup
- [ ] Integration tests: 30s mainnet gRPC stream validation
- [ ] Benchmarks: `Obligation::from_bytes` throughput (criterion)
- [ ] Benchmarks: `DashMap` concurrent ops (criterion)
- [ ] Benchmarks: full parse pipeline latency (criterion)
