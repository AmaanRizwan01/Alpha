# Project State & Handoff

## Current Snapshot
- **Active Phase:** Phase 2 — Hypothesis Testing ($H_1 \rightarrow H_2 \rightarrow H_3$) & Exit Ablation ($E_0..E_2$)
- **Active Sprint:** Sprint 2.2 — Purged Walk-Forward Simulation Engine (`run_walkforward.py`)
- **Git Branch:** `main` (commit `b24c80b`, synchronized with `origin/main`, clean)
- **Environment:** Python 3.11.9, Rustc 1.99.1, Cargo 1.99.1, MSVC BuildTools 2022
- **Status:** Sprint 2.1 completed with 100% test pass (26 Rust tests + 10 pytest tests). $H_1$ breakout and $E_0..E_2$ exits compiled and verified in Rust and Python.

## Immediate Next Task
1. Create sprint branch `feat/p2-s2-walkforward-engine`.
2. Implement purged walk-forward backtest runner ([`python/research/run_walkforward.py`](file:///c:/Coding/alpha/python/research/run_walkforward.py)) with rolling 12m train / 3m out-of-sample test and 48h purge/embargo across 2021–May 2026 data.
3. Simulate $H_1$ across `BTC` and `ETH` (`1h` and `4h`) under baseline $E_0$ (Chandelier ATR trail) and ablations $E_1$ (stagnation time stop) and $E_2$ (delayed breakeven ratchet).
4. Record trade metrics (net expectancy in $R$, win rate, max drawdown, fees paid) for statistical validation in Sprint 2.3.

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

## Active Blockers / Decisions
- None. Sprint 2.1 signed off. Ready for Sprint 2.2 walk-forward backtest.
