# Project State & Handoff

## Current Snapshot
- **Active Phase:** Phase 1 — Historical Data Loader & Rust Workspace (Completed & Pushed to `origin/main`)
- **Active Sprint:** Sprint 1.1 — Workspace initialization & data loader (Completed)
- **Git Branch:** `main` (commit `49f31f2`, synchronized with `origin/main`, clean)
- **Environment:** Python 3.11.9, Rustc 1.99.1, Cargo 1.99.1, MSVC BuildTools 2022
- **Status:** Phase 1 historical data and Rust core verified with 100% test pass. Evidence committed in [`docs/gates/PHASE-1.md`](file:///c:/Coding/alpha/docs/gates/PHASE-1.md).

## Immediate Next Task
1. Create sprint branch `feat/p2-s1-h1-strategy-and-exits`.
2. Implement $H_1$ hypothesis signal engine (`evaluate_h1_signal`) in [`rust/crates/alpha-core`](file:///c:/Coding/alpha/rust/crates/alpha-core).
3. Implement exit policies $E_0$ (Chandelier ATR trail), $E_1$ (stagnation time stop), and $E_2$ (delayed breakeven ratchet) in [`rust/crates/alpha-core`](file:///c:/Coding/alpha/rust/crates/alpha-core).
4. Expose strategy & exit functions via [`rust/crates/alpha-py`](file:///c:/Coding/alpha/rust/crates/alpha-py) for Python walk-forward simulation.

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

## Active Blockers / Decisions
- None. Phase 1 gate requirements met. Ready for Phase 2.
