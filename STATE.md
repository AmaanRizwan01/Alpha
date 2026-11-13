# Project State & Handoff

## Current Snapshot
- **Active Phase:** Phase 5 (8-Week Paper Burn-In on Proxmox) - Day 3 of 56
- **Active Sprint:** Sprint 5.10 — COMPLETE. Merged to `main`. Proxmox LXC 101 running latest release binary.
- **Git Branch:** `main` (Sprint 5.10 merged from `feat/p5-s5.10-subaccounts-sim-tv-tg`, commit `883ca2f`)
- **Environment:** Debian 12 LXC (CTID 101, `192.168.1.230`, Proxmox VE UAE node) & Windows 11 Dev Station (Python 3.11.9, Rustc 1.99.1)
- **Status:** Sprint 5.10 fully deployed end-to-end. Multi-sub-account simulation (`alphabreakout`, `alphacarry`, `alphamacro`) live on Proxmox with isolated SQLite state, independent drawdown tracking, and 6% circuit breakers. Telegram keyboard simplified to clean 2x2 grid. Unified TradingView Pine Script v5 published (`tradingview/alpha_portfolio.pine`). `.env` prepped: all 14 environment variables populated (sub-account slots temporarily use main OKX read-only key until $100 deposit + sub-account key generation; variable names are final — only the values need swapping at go-live). All 96 cargo workspace tests and 43 pytest tests green on both Windows and Proxmox Debian 12. Node `RUNNING / HEALTHY` verified via `alpha-node status`.

## Immediate Next Task
1. **Fund sub-accounts**: Deposit ≥ $100 each to `alphabreakout`, `alphacarry`, `alphamacro` via OKX → Assets → Sub-Account Transfer.
2. **Generate trade API keys**: For each funded sub-account, create a V5 API key with `Read + Trade` (no Withdraw), bind to Proxmox static IP.
3. **Swap keys in `.env`**: Replace the 3 `OKX_API_KEY_BREAKOUT/CARRY/MACRO` placeholder values with the real sub-account keys.
4. **Sync to Proxmox**: `scp c:/Coding/alpha/.env root@192.168.1.230:/opt/alpha/.env`
5. **Flip to live**: Change `allow_live_orders = true` in `configs/live_100.toml`, restart `alpha-node.service`.
6. **Continue monitoring** paper burn-in until 4-week paper gate passes (see Phase 4 roadmap).

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
- [x] Built 5 hardware severance fault drills in [`tests/test_fault_drills.rs`](file:///c:/Coding/alpha/rust/crates/alpha-node/tests/test_fault_drills.rs) (power cuts, offline stops, rogue coin alarms, dead-man heartbeat).
- [x] Compiled and signed off Phase 4 gate evidence file ([`docs/gates/PHASE-4.md`](file:///c:/Coding/alpha/docs/gates/PHASE-4.md)).
- [x] Scaffolded Debian 12 LXC provisioning script (`deploy/setup_lxc.sh`), watchdog monitor (`deploy/scripts/heartbeat.sh`), and operator emergency flatten script (`deploy/scripts/emergency_flatten.sh`).
- [x] Authored systemd daemon & timer units (`deploy/systemd/alpha-node.service`, `alpha-heartbeat.service`, `alpha-heartbeat.timer`).
- [x] Provisioned Debian 12 LXC container (CTID 101, `192.168.1.230`) on Proxmox VE node in Ajman, UAE.
- [x] Hardened filesystem layout under `/opt/alpha` with dedicated unprivileged `alpha:alpha` ownership, `chmod 600` on `.env`, and `chmod 700` on `data/`.
- [x] Natively compiled `alpha-node` in release mode inside Debian 12 container in 51s with zero warnings.
- [x] Launched `alpha-node.service` and `alpha-heartbeat.timer` under systemd; verified 60s logging ticks.
- [x] Tested operator emergency flatten drill: confirmed real Telegram alert dispatch, SQLite halt persistence, and node trade refusal across systemd restarts.
- [x] Tested cold-boot recovery drill: cleared halt state and restored daemon to `RUNNING / HEALTHY`.
- [x] Upgraded watchdog heartbeat (`deploy/scripts/heartbeat.sh`) with single-alert latch and recovery resolution message (zero notification spam).
- [x] Built `MarketTrade` event in [`alpha-types`](file:///c:/Coding/alpha/rust/crates/alpha-types/src/trade.rs) with price and quantity validation.
- [x] Implemented deterministic paper fill simulator in [`alpha-node`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/simulator.rs) with maker passive fills, atomic stop/take-profit triggers, and fees.
- [x] Implemented zero-loss Binance WebSocket stream models and deserializers in [`alpha-venues`](file:///c:/Coding/alpha/rust/crates/alpha-venues/src/binance_ws.rs).
- [x] Built pure-Rust async WebSocket subscriber and stream parser in [`alpha-node`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/ws.rs).
- [x] Integrated live feeds, paper matching, trailing ratchet, and SQLite WAL updates into `TradeEngine` in [`alpha-node`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/engine.rs).
- [x] Verified live Binance WebSocket stream connectivity in [`tests/test_live_ws_smoke.rs`](file:///c:/Coding/alpha/rust/crates/alpha-node/tests/test_live_ws_smoke.rs).
- [x] Compiled and signed off Phase 5 gate evidence file ([`docs/gates/PHASE-5.md`](file:///c:/Coding/alpha/docs/gates/PHASE-5.md)).
- [x] Implemented Sprint 5.4: FIFO `BarBuffer`, REST kline bootstrap (`bootstrap.rs`), live $H_1$ signal evaluator (`signal.rs`), fill recorder (`fill_recorder.rs`), and end-to-end paper order execution in `TradeEngine`.
- [x] Implemented Sprint 5.5: Non-blocking asynchronous Telegram notifications (`telegram.rs`) for signals, fills, exits, circuit breakers, and startup; deep status CLI command (`status.rs` via `alpha-node status <config>`); extracted `signal_dispatch.rs`; 78 cargo tests + 18 pytest tests green.
- [x] Implemented Sprint 5.6: Interactive Telegram Bot listener (`telegram_listener.rs`) with deleted obsolete commands/webhooks, native Alpha slash commands (`/status`, `/positions`, `/pnl`, `/orders`, `/flatten`, `/help`), persistent custom action keyboard, and full mobile node control; 79 cargo tests + 18 pytest tests green.
- [x] Implemented Sprint 5.7: Resolved WebSocket kline interval clash causing 4h restart loop. Tagged `interval` on `LiveMarketEvent::Kline`, restricted stream URL to configured timeframe (`4h`), added non-fatal buffer error handling, committed to `main` (`72f7813`).
- [x] Implemented Sprint 5.8: Added persistent auto-reconnecting WebSocket stream loop with backoff, non-fatal live frame parsing (skipping transient crossed quotes), and single-shot startup telemetry latch; committed to `main` (`1ebf501`). All 79 cargo workspace tests and 18 pytest tests green.
- [x] Deployed Sprint 5.8 to Proxmox LXC 101: Native release compilation succeeded in 13.9s, replaced `/opt/alpha/bin/alpha-node`, verified `alpha-node.service` running 24/7 with zero Telegram restart spam.
- [x] Evaluated exploratory H2 CME index futures & ICT models: failed quantitative edge/fee gates due to micro-contract friction; cleanly decommissioned and deleted all branches and scratch artifacts to protect H1 production isolation.
- [x] Completed multi-strategy research portfolio expansion: purged 4 failed strategy candidates (`cs_momentum.py`, `session_imb.py`, `flush_rev.py`, `coint_pair.py`) due to fee drag or non-stationary breakdown; consolidated and standardized 3 orthogonal, zero-correlation surviving strategies (`VOLATILITY_BREAKOUT`, `FUNDING_CARRY`, `MACRO_TREND`); verified complete modular independence and self-containment; refreshed `strategy_comparison_matrix.md` and `all_strategies_trades.parquet`; all 79 cargo workspace tests and 36 pytest tests green.
- [x] Implemented `FUNDING_CARRY` robust execution safeguards: Fee-Hurdle Lock in `FundingCarryStrategy` (preventing premature decay exits when gross carry < fee friction while preserving immediate `FundingInversionStop` on negative rates), added comprehensive unit & synthetic tests in `python/tests/test_multi_strategy.py` (100% pass), refreshed `strategy_comparison_matrix.md` and `all_strategies_trades.parquet` (Win Rate rose to 92.3%, PF rose to 464.12), and documented live execution safeguards (Fee-Hurdle Lock, Predictive Pre-Exit via `GET /fapi/v1/premiumIndex`, Unified/Portfolio Margin zero-liquidation pool, and sub-100ms live WebSocket ADL Desync Watchdog in `alpha-node`) in [`docs/STRATEGY.md`](file:///c:/Coding/alpha/docs/STRATEGY.md) Section 6. All 76 workspace cargo tests and 38 pytest tests green.
- [x] Created comprehensive backtest repository documentation in [`docs/backtests/`](file:///c:/Coding/alpha/docs/backtests/): structured individual markdown reports for all three surviving strategies (`VOLATILITY_BREAKOUT.md`, `FUNDING_CARRY.md`, `MACRO_TREND.md`), master portfolio comparison matrix (`README.md`), and quantitative post-mortem on rejected candidates (`KILLED_STRATEGIES.md`). Fully documented $100 starting capital performance, win rates, payoff ratios, yearly/monthly/weekly/daily earnings, drawdowns, and loss breakdowns.
- [x] Cleaned codebase and restructured documentation hierarchy: decommissioned legacy `brainstorming/` directory (deleted obsolete v1-v4 plan drafts, relocated canonical multi-strategy specification to `docs/STRATEGY.md`), reorganized uppercase `BACKTEST/` into clean `docs/backtests/`, resolved `python/research/run_walkforward.py` standalone module import pathing, purged transient SQLite test databases and stale pycache artifacts. Verified 100% test suite pass across Rust (76 cargo tests) and Python (38 pytest tests).
- [x] Audited codebase architecture & research laboratory: established canonical Quantitative Research Laboratory specification in [`python/research/README.md`](file:///c:/Coding/alpha/python/research/README.md), resolved Windows SQLite file handle locking in `alpha-node` test suites (`test_engine.rs`, `test_telemetry.rs`) ensuring zero persistent test artifact leakage, updated root `README.md` repository layout, and verified full suite health across Windows workstation and Proxmox LXC 101 node.
- [x] Implemented Sprint 5.9: Ported `FUNDING_CARRY` strategy from Python research to pure Rust in `alpha-core` (`carry.rs`, `carry_sim.rs`, `carry_sizing.rs`) with Fee-Hurdle Lock, Funding Inversion Stop, and delta-neutral scale-invariant sizing; exposed via PyO3 in `alpha-py` (`carry.rs`); implemented Binance Futures models and parsers in `alpha-venues` (`fapi.rs`); implemented live carry evaluation (`carry_signal.rs`) with Predictive Funding Pre-Exit and sub-100ms ADL Desync Watchdog (`adl_watchdog.rs`) with Drill 6 in `alpha-node`. All 91 cargo workspace tests and 43 pytest tests green.
- [x] Implemented Sprint 5.10: Added OKX sub-account setup runbook and $100 balance requirement in `PLAN.md` Section 5.1; built multi-sub-account simulation in `alpha-node` with isolated SQLite state (`sub_accounts`, `sub_account_positions`, `sub_account_orders`), independent equity tracking, drawdown calculation, and circuit breakers for `alphabreakout`, `alphacarry`, and `alphamacro`; simplified Telegram custom keyboard to sleek 2x2 grid (`📊 Status`, `📈 Positions`, `💰 PnL`, `🚨 Flatten`) with sub-account breakdown; authored unified Pine Script v5 script in `tradingview/alpha_portfolio.pine` and documentation in `docs/TRADINGVIEW.md`; enforced $\le 250$ line limits across all files. All 95 cargo workspace tests and 43 pytest tests green.

## Active Blockers / Decisions
- None. Branch `feat/p5-s5.10-subaccounts-sim-tv-tg` verified and ready for deployment to Proxmox LXC 101.
