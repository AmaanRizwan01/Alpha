# Project State & Handoff

## Current Snapshot
- **Active Phase:** Phase 2 — Hypothesis Testing ($H_1 \rightarrow H_2 \rightarrow H_3$) & Exit Ablation ($E_0..E_2$)
- **Active Sprint:** Sprint 2.3 — Statistical Kill Gates & Holdout Evaluation (`validate_gates.py`)
- **Git Branch:** `main` (commit `33033f2`, synchronized with `origin/main`, clean)
- **Environment:** Python 3.11.9, Rustc 1.99.1, Cargo 1.99.1, MSVC BuildTools 2022
- **Status:** Sprint 2.2 completed with 100% test pass (26 Rust tests + 15 pytest tests). Purged walk-forward engine executed across 18 rolling folds (2,702 out-of-sample trades recorded in `data/walkforward_trades.parquet`). `1h` killed due to fee drag; `4h` verified with $+0.207R$ net EV across 115 trades.

## Immediate Next Task
1. Create sprint branch `feat/p2-s3-statistical-gates`.
2. Implement statistical validation suite ([`python/research/validate_gates.py`](file:///c:/Coding/alpha/python/research/validate_gates.py)):
   - Politis–Romano stationary block-bootstrap (10,000 resamples) computing 95% lower confidence bound $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > +0.02R$.
   - Deflated Sharpe Ratio ($\text{DSR} > 0.95$) accounting for non-normality, skewness, kurtosis, and trial variance.
   - Parameter perturbation stability checks ($\pm 20\%$ shifts on thresholds).
   - Single-pass Stage B holdout evaluation on May–Nov 2026 data.
3. Compile empirical findings into Phase 2 gate evidence file ([`docs/gates/PHASE-2.md`](file:///c:/Coding/alpha/docs/gates/PHASE-2.md)).

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
- [x] 100% test pass (26 Rust tests + 15 pytest tests passing), merged to `main` (`33033f2`) and pushed to GitHub.

## Active Blockers / Decisions
- None. Sprint 2.2 signed off. Ready for Sprint 2.3 statistical kill gates.
