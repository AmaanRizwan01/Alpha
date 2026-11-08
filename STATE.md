# Project State & Handoff

## Current Snapshot
- **Active Phase:** Phase 4 (Live Execution Engine & Venue Drivers)
- **Active Sprint:** Sprint 4.2 Signed Off -> Ready for Sprint 4.3 (Fault Injection & Recovery Drills)
- **Git Branch:** `feat/p4-s2-alpha-node-fsm`
- **Environment:** Python 3.11.9, Rustc 1.99.1, Cargo 1.99.1, MSVC BuildTools 2022
- **Status:** Sprint 4.2 completed with 100% test pass (44 Rust tests + 18 pytest tests). Zero clippy warnings under strict `-D warnings`, zero unsafe code (`#![forbid(unsafe_code)]`). Built `alpha-node` binary and library crate with embedded SQLite WAL migrations (`migrations/0001_state.sql`), crash-safe database manager (`db.rs`), cold-boot state reconciliation state machine (`reconcile.rs`), -6.0% equity drawdown circuit breaker, and operational CLI subcommands (`status`, `flatten`, `trade`, `record`).

## Immediate Next Task
1. Review Sprint 4.2 diffs and commit to `feat/p4-s2-alpha-node-fsm`.
2. Merge `feat/p4-s2-alpha-node-fsm` into `main` and push to GitHub.
3. Start Sprint 4.3: Fault Injection Drills & Phase 4 Gate Documentation.

## Verified Milestones
- [x] Master architecture specification audited ([`PLAN.md`](file:///c:/Coding/alpha/PLAN.md) v3.2).
- [x] UAE regulatory & zero-tax spot feasibility confirmed.
- [x] Workspace rules configured ([`GEMINI.md`](file:///c:/Coding/alpha/GEMINI.md)).
- [x] Public GitHub repository created and synchronized (`AmaanRizwan01/Alpha`).
- [x] Scaffolded `python/scripts/phase0_feasibility.py` and ran live account scanner.
- [x] Verified live Binance fees: confirmed 0 bps maker on `BTCFDUSD` and 10 bps on `BTCUSDT`/`ETHUSDT`.
- [x] Verified live OKX fees: Lv1 tier with 8 bps maker and 10 bps taker.
- [x] Verified exchange filters: Binance $5.0 minNotional and 0.00001 BTC lot step.
- [x] Generated Phase 0 evidence file ([`docs/gates/PHASE-0.md`](file:///c:/Coding/alpha/docs/gates/PHASE-0.md)).
- [x] Built Binance Vision historical spot data downloader with microsecond timestamp normalization ([`python/research/download_history.py`](file:///c:/Coding/alpha/python/research/download_history.py)).
- [x] Downloaded 4.8 years of spot klines with taker buy volume for BTC and ETH (1h & 4h) into compressed Parquets in `data/raw/`.
- [x] Created `#![forbid(unsafe_code)]` Rust crates: [`alpha-types`](file:///c:/Coding/alpha/rust/crates/alpha-types) and [`alpha-core`](file:///c:/Coding/alpha/rust/crates/alpha-core).
- [x] Built PyO3 FFI wrapper crate [`alpha-py`](file:///c:/Coding/alpha/rust/crates/alpha-py) and compiled wheel via Maturin into `.venv`.
- [x] Verified unit tests, scale invariance property tests ($100 to $50,000), and Python integration tests (19 Rust tests + 6 pytest tests passing).
- [x] Generated Phase 1 evidence file ([`docs/gates/PHASE-1.md`](file:///c:/Coding/alpha/docs/gates/PHASE-1.md)).
- [x] Merged Phase 1 into `main` and pushed to GitHub.
- [x] Implemented $H_1$ regime-gated volatility breakout signal engine in [`rust/crates/alpha-core/src/h1.rs`](file:///c:/Coding/alpha/rust/crates/alpha-core/src/h1.rs).
- [x] Implemented structural stop floor ($\ge 2.20\%$), $E_0$ Chandelier ATR trail, $E_1$ stagnation cut, and $E_2$ delayed breakeven ratchet in [`rust/crates/alpha-core/src/exits.rs`](file:///c:/Coding/alpha/rust/crates/alpha-core/src/exits.rs).
- [x] Exposed `evaluate_h1`, vectorized `scan_h1_signals`, `calc_initial_stop`, and `simulate_trade_exit` via PyO3 in [`rust/crates/alpha-py`](file:///c:/Coding/alpha/rust/crates/alpha-py).
- [x] Verified 100% test pass (26 workspace cargo tests, 10 pytest tests), zero clippy warnings, clean `cargo fmt`, merged to `main` (`b24c80b`) and pushed to GitHub.
- [x] Built purged walk-forward backtest engine with rolling 12m train / 3m out-of-sample test and 48h purge gap ([`python/research/walkforward.py`](file:///c:/Coding/alpha/python/research/walkforward.py)).
- [x] Implemented scale-invariant fee deduction ($\text{Fee}_R = C_{\text{rt}} / D_{\text{stop}}$) with conservative 23 bps friction.
- [x] Simulated $H_1$ breakout across BTC and ETH (`1h` and `4h`) under $E_0..E_2$ exit ablations across 18 rolling folds (`2021`–`2026-05-01`), recording 2,702 out-of-sample trades in `data/walkforward_trades.parquet` ([`python/research/run_walkforward.py`](file:///c:/Coding/alpha/python/research/run_walkforward.py)).
- [x] Empirically validated fee-to-stop ratio law: killed `1h` (friction eats edge, negative net EV), confirmed `4h` with durable $+0.207R$ net EV, 2.01 profit factor on BTC, and confirmed $E_1$ stagnation cut reduces ETH max drawdown from 6.60R to 4.27R.
- [x] Built statistical gate validation suite ([`python/research/validate_gates.py`](file:///c:/Coding/alpha/python/research/validate_gates.py)) implementing Politis-Romano block bootstrap (10,000 resamples), Deflated Sharpe Ratio, parameter perturbation grid ($\pm 20\%$), and Stage B untouched holdout evaluation.
- [x] Verified parameter perturbation stability: all 7 grid nodes generated positive net return between $+19.73R$ and $+25.71R$ with net EV $+0.169R$ to $+0.230R$.
- [x] Verified single-pass Stage B holdout (`2026-05-01` to `2026-11-01`): 17 trades, $+2.82R$ net return, $+0.166R$ EV, max DD $3.81R$, inside Stage A 95% bootstrap CI.
- [x] Compiled and verified Phase 2 gate evidence file ([`docs/gates/PHASE-2.md`](file:///c:/Coding/alpha/docs/gates/PHASE-2.md)).
- [x] Scaffolding of [`alpha-venues`](file:///c:/Coding/alpha/rust/crates/alpha-venues) with strict `#![forbid(unsafe_code)]` and zero clippy warnings.
- [x] Built OKX v5 atomic order builder with `attachAlgoOrds` (`slTriggerPx`, `slOrdPx: "-1"`).
- [x] Built OKX dead-man switch (`cancel-all-after` payload with 10–120s guard).
- [x] Built OKX WebSocket order book sequencer verifying `prevSeqId == last_seq_id` and `-1` snapshot reset.
- [x] Built Binance Spot atomic `OTOCO` parameter and query signer (`POST /api/v3/orderList/otoco`).
- [x] Built SQLite WAL state schema ([`migrations/0001_state.sql`](file:///c:/Coding/alpha/migrations/0001_state.sql)) with crash-safe journal and singleton guards.
- [x] Built [`alpha-node`](file:///c:/Coding/alpha/rust/crates/alpha-node) crate with bundled SQLite, embedded migrations, and cold-boot reconciliation FSM.
- [x] Built 6.0% equity drawdown circuit breaker persisting halt state in SQLite.
- [x] 100% test pass (44 Rust workspace tests + 18 pytest tests passing).

## Active Blockers / Decisions
- None. Sprint 4.2 signed off and ready for review, commit & merge.
