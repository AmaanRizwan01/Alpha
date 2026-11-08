# Project State & Handoff

## Current Snapshot
- **Active Phase:** Phase 5 (8-Week Paper Burn-In on Proxmox)
- **Active Sprint:** Sprint 5.2 Signed Off -> Autonomous 8-Week Paper Incubation Underway
- **Git Branch:** `feat/p5-s2-proxmox-lxc`
- **Environment:** Debian 12 LXC (CTID 101, `192.168.1.230`, Proxmox VE UAE node) & Windows 11 Dev Station (Python 3.11.9, Rustc 1.99.1)
- **Status:** Phase 5 complete with 100% test pass (53 Rust workspace tests + 18 pytest tests). Zero clippy warnings under strict `-D warnings`, zero unsafe code (`#![forbid(unsafe_code)]`). Dedicated unprivileged Debian 12 LXC container (`alpha-node`, CTID 101) provisioned on Proxmox VE. 24/7 daemon service (`alpha-node.service`) and external watchdog timer (`alpha-heartbeat.timer`) actively running on `/opt/alpha`. Emergency flatten kill switch, Telegram broadcast alerts, and cold-boot SQLite reconciliation drills executed and verified. Official gate evidence committed in `docs/gates/PHASE-5.md`.

## Immediate Next Task
1. Merge `feat/p5-s2-proxmox-lxc` into `main` and push to GitHub.
2. Allow the Proxmox node to run autonomously during the 8-week paper burn-in incubation window.
3. Track paper trading metrics: maker fill rates, slippage in bps against recorded order books, and circuit breaker stability.
4. Prepare Phase 6 ($100 micro-live incubation configuration & deposit protocol).

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
- [x] Compiled and signed off Phase 5 gate evidence file ([`docs/gates/PHASE-5.md`](file:///c:/Coding/alpha/docs/gates/PHASE-5.md)).

## Active Blockers / Decisions
- None. Sprint 5.2 signed off. Phase 5 8-week paper burn-in actively running autonomously on Proxmox VE.
