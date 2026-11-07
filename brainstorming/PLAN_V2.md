# Project Alpha v2 - Coding Agent Plan

Nov 7, 2026 · @Amaan Rizwan

## Verdict

Build a selective, hours-scale, fee-aware bot on OKX that stays flat unless a validated edge survives every cost. v1's millisecond microstructure, arbitrage and X-post trading are cut: none of them can pay for themselves on 100 USD with a 0 USD monthly budget.

What changed from v1:

- **Horizon:** milliseconds to hours (1h to 1d bars), so fees stop eating the edge.
- **Venue:** one UAE-licensed exchange (OKX), spot first, perpetuals only after a measured feasibility check.
- **Intelligence:** the X API is removed. It is pay-per-use with no free tier, so it costs money from the first post read. Free scheduled-event blackouts and exchange status feeds replace it.
- **Stack:** 13 services become 4 processes on your Proxmox box: one Rust workspace plus an offline Python research kit.
- **Order of work:** the data recorder runs from day 1, and no live money moves until every gate in this plan passes.
- **Safety:** a layered kill switch plus exchange-side stops, so a dead server cannot leave an unprotected position.

**Honest expectation.** No system wins every trade, and nobody can build one that only wins. The standard this plan enforces is stricter and real: trade only when the lower confidence bound of net expectancy, after all costs, is above zero. Expect a win rate near 40 to 55 percent with winners larger than losers. On 100 USD a good year is tens of dollars; the real prize is a proven autonomous system you can scale later. The most likely result of the research phase is that nothing passes and the bot stays flat. That counts as the process working.

## Venue: OKX first, everything else read-only

OKX is the only large exchange whose VARA licence lets UAE retail clients trade perpetuals, capped at 5x leverage, and it also has a full licence for spot. Use only the licensed entity the exchange assigns to your residency, funded from your own bank account. No VPNs and no offshore entities: VARA has fined unlicensed exchanges for serving UAE customers.

| Venue | UAE licence status (Oct 2026) | Retail derivatives | Role in this plan |
| --- | --- | --- | --- |
| [OKX](https://www.coinperps.com/learn/best-crypto-exchanges-uae) (OKX Middle East Fintech FZE) | Full VARA exchange licence, active since Sep 2024 | Yes, up to 5x | Live trading and data |
| Binance (Binance FZE) | Full VARA licence since Apr 2024 | No, qualified investors only | Read-only data, free history |
| Bybit | Federal CMA licence for emirates outside Dubai; VARA was in-principle only on 16 Oct, a later report says a full licence followed (verify on the register) | Not confirmed | Read-only data, possible second venue |

**Fees decide the instrument.** Reported OKX UAE spot fees are 0.08% maker and 0.10% taker on USDT pairs. UAE derivatives fees start at 0.10% maker and 0.25% taker, several times OKX's global schedule. A perpetual round trip therefore costs more than a spot round trip here, so **spot long-or-flat is the live instrument**. Perpetuals are used only if a validated strategy needs shorting and the feasibility check passes.

**Regulatory risk is real.** VARA caps retail leverage at 5:1 with at least 20% initial margin, reassesses client suitability on an ongoing basis, and can raise margin, change leverage caps or suspend derivatives services at any time ([VARA rulebook summary](https://www.mondaq.com/commoditiesderivativesstock-exchanges/1770516/dubais-derivatives-framework-vara-is-now-open-for-crypto-futures-perps-options-and-cfds)). Treat perpetual access as revocable, and never design the system so that it depends on it.

**Cut for good:** live trading on Binance, Bybit or Coinbase, DEXs, and long-tail altcoins. The live universe is BTC/USDT and ETH/USDT only. Long-tail pairs stay in paper research because wide spreads, manipulation and delisting risk do not suit a bot nobody watches.

**Tax:** the source above states UAE residents pay no personal tax on crypto gains, but trading run as a business can fall under the 9% corporate tax, and exchanges begin collecting data for OECD crypto reporting in 2027. Confirm your own position; this is not tax advice.

## Reality check at 100 USD

Fees and minimum contract sizes, not ideas, are the binding constraints. The agent computes them from live exchange metadata before any strategy work, and never trusts the figures below over what the exchange returns.

| Constraint | Number | Consequence |
| --- | --- | --- |
| Spot round-trip cost | 0.20% taker, 0.16% maker | About 0.10 USD on a 50 USD position; a trade needs an expected move well above 0.5% |
| Perpetual round-trip cost (UAE) | About 0.50% taker, 0.20% maker | Shorting is expensive; only for a large validated edge |
| Risk per trade | 1% of equity = 1 USD | A 2% stop gives roughly 43 USD of position (fee 0.10% + slippage 0.05% per side) |
| Leverage | 5x legal cap; project limit 2x | At most about 200 USD of notional |
| BTC perpetual minimum | Typically 0.01 BTC per contract, about 760 USD at roughly 76,000 USD per BTC | Above even 5x of 100 USD: BTC perpetuals are infeasible |
| ETH perpetual minimum | Typically 0.1 ETH per contract, about 224 USD at roughly 2,235 USD per ETH | Above the 2x project limit: infeasible unless the limit is raised on purpose |
| Spot minimum order | Small and divisible (verify) | Spot is the only fully feasible live instrument at this size |

The contract sizes are from memory and the prices from a market ticker on a page read today, so both are approximate. Phase 0 replaces them with the exchange's real `ctVal`, `lotSz`, `minSz` and `tickSz`.

A perpetual is feasible only when its minimum position fits inside the leverage budget:

```latex
minSz \cdot ctVal \cdot P \le E \cdot \lambda_{max}
```

Position size is the smallest of the risk, leverage and liquidity limits, then rounded down to the lot size:

```latex
q = \min\left(\frac{E\,r}{L_{unit}},\ \frac{E\,\lambda_{max}}{P_e},\ q_{liquidity}\right), \qquad L_{unit} = |P_e - P_s| + P_e(f_e + s_e) + P_s(f_x + s_x)
```

Here E is equity, r the risk fraction (0.01), P\_e and P\_s the entry and stop prices, f the fee rates and s the slippage allowances. If the rounded order is below the exchange minimum, or its real risk exceeds 1% of equity, the trade is skipped. Risk is never raised to meet a minimum.

**Cost rule (pre-registered):** a trade is eligible only if its expected gross move is at least 3 times the round-trip cost. This kills most signals, on purpose.

## Edge hypotheses, in test order

Nobody can promise an edge, so the plan lists falsifiable hypotheses with kill rules written before any test is run. A hypothesis that fails is deleted, not tuned until it passes. The live bot is long-or-flat on spot, so it sits in USDT most of the time, and that is intended.

| # | Hypothesis | Instruments | Horizon | Free data | Kill rule |
| --- | --- | --- | --- | --- | --- |
| H1 | Trend: hold when a volatility-adjusted breakout and a regime filter agree, otherwise stay in USDT; size by volatility | BTC/USDT, ETH/USDT spot | 4h to 1d | Candles, funding history | Net Sharpe 95% interval includes 0 in walk-forward, or the edge appears in only one market regime |
| H2 | Capitulation bounce: buy after extreme negative funding, an open-interest drop and a liquidation flush; exit on mean reversion or a time stop | Same, spot execution | 4h to 24h | Funding, open interest, liquidations (recorded forward) | Fewer than about 100 independent events, or net expectancy at or below 0 after costs |
| H3 | Meta-labeler: a calibrated model (logistic regression first, then LightGBM) that trades an H1 or H2 signal only when predicted win probability times payoff beats cost plus a margin | Same | Same | Same plus features | No gain over the unfiltered signal on untouched data |
| H4 | Delta-neutral funding carry: long spot, short perpetual | ETH, BTC | Days | Funding history | Research only: UAE perpetual fees and minimum sizes make it infeasible at 100 USD, so it stays paper until capital grows |
| H5 | Relative value on the ETH/BTC ratio | ETH, BTC | Hours to days | Candles | Research only, no live permission |

Run H1 and H3 first, because they need only free candle and funding history. H2 follows once the recorder has collected enough liquidation and open-interest data. H4 and H5 are the scaling-stage candidates.

How every hypothesis is tested:

- **Labels:** triple-barrier outcomes (profit target, stop, time limit) with costs already deducted, so the model learns net results.
- **Features:** only information stamped before the decision time: multi-horizon returns, realized volatility, ATR, funding level and z-score, open-interest change, taker buy/sell imbalance, ETH/BTC ratio, time of day.
- **Baselines:** buy-and-hold, and random entries with the same trade count and costs. A strategy that cannot beat random entries has no edge.

**Excluded, with reasons:** millisecond microstructure on BTC/USDT (a few basis points of edge against 16 to 20 basis points of fees, and colocated firms on the other side), cross-exchange and triangular arbitrage (split capital and minimum sizes), trading on X posts (bots reprice before an API call returns, and the data costs money), long-tail pump chasing (manipulation, and unattended), and any martingale or averaging down.

## Architecture

&#91;embedded content: architecture - 4 processes, 1 offline research box\]

Only the engine holds the trade key and opens risk. It opens its own public market stream, so the recorder can crash without blinding it. Commands flow from your phone to the notifier, the watchdog and the engine; the watchdog can only cancel and flatten, and exchange-side stops protect you even if the whole box dies.

## Language split

Rust for everything that touches money, Python for everything that explores ideas, and one shared Rust core so that research and live trading run the same strategy code. At hour-scale decisions latency barely matters, so Rust is chosen for correctness and a single static binary per process, not for speed.

| Component | Language | Why |
| --- | --- | --- |
| `alpha-core`: features, strategy rules, sizing, risk rules, order state machine | Rust library | Compiled twice: into the live binaries, and into a Python module with PyO3 and maturin so backtests execute the exact live code. This removes backtest-versus-live drift, the most common way bots fail |
| `recorder`, `engine`, `watchdog`, `notifier` | Rust (tokio) | Memory safety, no garbage-collection pauses, strong types for order and money logic |
| Research, labeling, backtests, model training, reports | Python (Polars, DuckDB, NumPy, SciPy, statsmodels, scikit-learn, LightGBM, Jupyter) | Fastest iteration; strictly offline |
| Model deployment | Python exports coefficients or trees to a signed JSON or ONNX file; Rust loads it | No Python process in the live path |
| Storage | Parquet and DuckDB for research data; SQLite for live state and the audit log | Zero cost, zero administration |
| Config | YAML validated by Rust types, plus SQL migrations | Invalid config refuses to start |
| Operator interface | Telegram bot and a CLI `status` command; Grafana later only if useful | No UI before an edge exists |
| Deployment | Debian VM on Proxmox, systemd units, Docker Compose only for optional Grafana | Simple, reproducible, restartable |

**Not used in v1:** C++, Go, Kafka, NATS, Redis, Postgres, React, Kubernetes. Each adds failure modes without adding edge. NautilusTrader is optional as an independent cross-check simulator in the research phase, never as the live engine.

## Risk engine and kill switch

The risk gate is the only code allowed to approve an order, and strategies, models and news can never change its limits. Limits can be tightened by config at any time but are never loosened by code or by the bot itself.

| Control | Setting |
| --- | --- |
| Risk per trade | 1% of current equity, sized with the cost-aware formula |
| Leverage | Spot only at launch; if perpetuals are ever enabled, 2x maximum |
| Liquidation buffer (perpetuals only) | Liquidation price at least 3 times farther from entry than the stop |
| Open positions | 1 |
| Entries per day | 3 maximum, to cap fee bleed |
| Daily loss stop | 2% of start-of-day equity: flat and no entries for 24 hours |
| Weekly loss stop | 4%: no entries until the next week starts |
| Drawdown halt | 12% from the equity high-water mark: full halt, human re-arm required |
| Averaging down, martingale, pyramiding losers | Forbidden |
| API keys | No withdrawal permission, IP-allowlisted to the server |
| Event blackouts | No new entries around scheduled FOMC, CPI and jobs-report releases, read from official calendars |

Kill switch, four independent layers:

1. **Exchange side, works if everything else is dead.** Every entry has an exchange-hosted stop confirmed within a timeout, otherwise the engine closes the position with a bounded order and halts. Working entry orders use the exchange's cancel-all-after dead-man timer, which the engine keeps refreshing (verify the endpoint on your entity).
2. **Software.** A global HALT flag in SQLite is checked by the risk gate before every order. The watchdog process can set it and flatten positions using its own reduce-only key, so a hung engine cannot block a shutdown.
3. **Human.** Telegram commands `/status`, `/halt`, `/flatten` and `/rearm` from an allowlisted chat ID with a confirmation code. Last resort: revoke the API keys in the exchange app.
4. **External heartbeat.** The watchdog pings a free dead-man monitor; if your Proxmox box, power or internet dies, the monitor alerts your phone.

Automatic HALT triggers: any loss stop, stale market data, a gap in candles or the book, private stream disconnect, balance or position mismatch, an unknown open order, repeated rejects, rate-limit lockout, clock drift above 500 ms, low disk, model or config hash mismatch, abnormal spread, and an active event blackout.

Day-to-day operation is fully autonomous. The single deliberate exception to zero human intervention is re-arming after a drawdown halt or an integrity halt, because those mean the model or the plumbing is wrong.

## Execution rules

At hour-scale horizons the goal is cheap, correct fills, not fast ones.

- **One path to an order:** strategy proposal, then risk gate, then an immutable order intent with a deterministic client order ID, then the exchange. Nothing else may submit orders.
- **Entries:** a post-only limit at the touch for a fixed number of seconds to earn the maker fee, then cancel. Fall back to a marketable limit with a strict price collar only if the expected edge still clears the cost rule at taker fees. No uncollared market orders, ever.
- **Stops:** every position has an exchange-hosted stop (conditional or algo order; confirm which spot order types your entity supports). Prefer a stop that executes at market or a limit with a wide collar, because a tight stop-limit can be skipped in a fast move. Targets, trailing logic and the time stop run in the engine; the time stop exits at the bar close.
- **Fill truth:** state comes from the private WebSocket plus periodic REST reconciliation. An HTTP 200 is an acknowledgement, never a fill.
- **Ambiguity:** on a timeout, query by client order ID before any retry. Never resend blindly.
- **Startup reconciliation:** compare balances, positions and open orders with local state on every start. Any mismatch means HALT and an alert.
- **Order states:** CREATED, VALIDATED, SUBMITTED, ACKED, PARTIAL, FILLED, CANCELED, REJECTED, UNKNOWN. Transitions are enforced in `alpha-core` and property-tested.
- **Limits:** respect documented rate limits with jittered backoff; a rate-limit lockout is a HALT trigger.
- **Rehearsal:** run the full order lifecycle on OKX's demo-trading environment before any real order (confirm it exists on your entity), and log expected versus actual slippage and fill rate for every paper and live trade.

## Data at zero cost, and the validation protocol

**Data plan.** The recorder starts on day 1 and runs 24/7 on the Proxmox box, capturing OKX (and Binance, read-only) trades, top-of-book and 20-level order book, candles, funding, mark and index price, open interest and liquidations to compressed Parquet with receive timestamps. Years of candles and funding history come from the exchanges' free public endpoints and public data archives (the agent verifies availability and licence terms for each). Paid tick-level vendors start in the hundreds of USD per month, so none is bought. There is no free full order-book history, which is acceptable: H1 to H3 need candles, funding and open interest; the order book is used only for execution and fill modelling, and it accumulates from the recorder. The agent calculates storage per day and sets a retention policy.

**Validation protocol.**

- Pre-register each hypothesis, its parameter range and its kill rule in `docs/research/` before running it.
- Chronological walk-forward with purged and embargoed splits where labels overlap. The most recent 6 months are an untouched holdout, evaluated once per strategy.
- Conservative costs: taker fees even for maker entries in the first pass, plus slippage of at least 5 basis points per side.
- Log every experiment with its config hash and report the number of trials. Report the deflated Sharpe ratio, the probability of backtest overfitting, and block-bootstrap confidence intervals.
- Leakage tests in CI: shuffled labels must destroy the edge, and no feature may read data stamped after the decision time.
- Low-frequency honesty: if a strategy produced fewer than about 100 independent trades in its history, the statistics cannot prove it. It may reach paper and micro-live but never scaled.

| Gate | Requirement |
| --- | --- |
| Research to paper | Lower 95% bound of net expectancy per trade above 0 on walk-forward and on the untouched holdout; deflated Sharpe above 0; works in at least 3 market regimes (up, down, chop); edge survives a 20% change in each parameter; drawdown inside the risk policy |
| Paper to micro-live | At least 8 weeks of continuous operation and at least 100 trades (more weeks if fewer trades); paper fills reconcile with the recorded book; zero unresolved critical bugs; every kill-switch drill passed |
| Micro-live to larger size | At least 8 weeks live at 100 USD; live expectancy inside the paper confidence interval; scaling is decided by you, never automatically, in steps of at most 2x |

## Roadmap and gates

The agent builds in this order and writes gate evidence to `docs/gates/PHASE-N.md` before starting the next phase. Realistic time to first live order is about five to six months, and the project may legitimately end at phase 3.

| Phase | Build | Exit gate |
| --- | --- | --- |
| 0. Setup and feasibility (week 1) | Confirm your emirate and the OKX entity serving you; spot API key (trade and read, no withdrawal, IP-allowlisted) plus a separate reduce-only key; Debian VM with chrony time sync; Telegram bot; external dead-man monitor; a feasibility calculator that reads live instrument metadata and your real fee tier | `docs/decisions/0001-feasibility.md` lists the instruments that fit 100 USD, the real fees and the entity; keys verified read-only |
| 1. Recorder (weeks 1 to 2) | OKX and Binance WebSocket clients, exchange-documented book snapshot and delta rebuild, gap detection, reconnect with jittered backoff, Parquet writer, deterministic replayer, exchange and local monotonic timestamps | 7-day soak with forced reconnects, no silent book corruption, replay output identical run to run |
| 2. Research kit (weeks 2 to 5) | History loaders, `alpha-core` as a Python module, triple-barrier labeling, cost model, backtester, baselines, leakage tests | A trivial strategy matches a hand calculation; shuffled labels kill the edge; baselines reproduce |
| 3. Hypotheses H1 to H3 (weeks 4 to 9) | Pre-registered tests, walk-forward, holdout, trial log | One strategy passes the research-to-paper gate, or the agent writes a no-edge report and the project stops here without building the engine |
| 4. Engine, risk and execution on demo (weeks 9 to 13) | Live binaries from `alpha-core`, risk gate, order state machine, private stream, reconciliation, exchange-side stops, watchdog, notifier, fault injection | Lifecycle, duplicate-order, disconnect and restart-reconciliation tests pass; every HALT trigger drilled; secret scan clean |
| 5. Paper trading (8 weeks or more) | Live data, simulated fills from the recorded book, weekly review | Paper-to-micro-live gate |
| 6. Micro-live, 100 USD (8 weeks or more) | Real orders at 1% risk; daily report to Telegram | Micro-live-to-larger-size gate |
| 7. Scaling and new hypotheses | H2, then H4 and H5 research; capital steps you approve | Each new strategy repeats the full gate chain |

## Rules for the coding agent

1. **Build in phase order.** Do not start a phase until the previous gate's evidence file exists and you have written it honestly. If a gate fails, report it and stop; do not weaken the gate.
2. **Fail closed.** Default config is paper mode with live orders disabled. Live trading requires a signed `LIVE_APPROVED` file created by the operator, and the engine refuses to start without it.
3. **Never guess an API.** Verify every endpoint, field, order type and limit against OKX's official documentation at build time, record the finding in `docs/decisions/`, and write down an open question instead of inventing behaviour.
4. **Tests before money paths.** Property tests for sizing and lot rounding, golden fixtures for every feature, replay tests for determinism, fault injection for disconnects, timeouts, duplicate fills and partial fills.
5. **One strategy code path.** Strategy, sizing and risk logic live only in `alpha-core`; research imports the same code the live binaries run.
6. **No look-ahead.** A feature may only read data stamped at or before the decision time. CI enforces it with a leakage test.
7. **Never fabricate.** No invented data, backtest fills, results or performance claims, and no promise of returns in any document or message.
8. **Secrets.** Environment injection only, an `.env.example` with names and no values, a CI secret scan, and keys never written to logs, the database or Telegram.
9. **Audit everything.** Persist raw inputs or their hashes, feature values, model version, risk verdict, order intent, exchange responses and fills for every decision.
10. **Keep it small.** Four processes, no extra infrastructure, until a measured problem demands more.

Repository layout:

```text
project-alpha/
  README.md
  docs/            decisions/  gates/  research/  runbooks/
  rust/
    Cargo.toml     (workspace)
    crates/
      alpha-core/  features, strategies, sizing, risk rules, order state machine, PyO3 module
      okx-client/  REST and WebSocket adapter, exchange types only, no business logic
      recorder/    engine/    watchdog/    notifier/
  python/
    research/      loaders, labeling, backtest, models, reports
    notebooks/     pyproject.toml
  configs/         risk/  strategies/  calendar/
  migrations/      SQLite schema
  tests/           unit/  replay/  fault-injection/  leakage/
  infra/           systemd/  proxmox/  monitoring/
  .env.example
```

Definition of done for going live: every gate in this plan has evidence on file; the kill switch was drilled end to end, including a pulled network cable and a killed engine with an open position; backups of the SQLite state and configs restore cleanly; and you have read the first week of paper results and signed the release. Production-ready does not mean profitable: the plan guarantees process quality, not returns.

## Verify before relying on it, and sources

These facts came from third-party pages or from memory and must be confirmed in your own account or the official docs during phase 0.

| Item | Why it matters | How to verify |
| --- | --- | --- |
| Which regulator and OKX entity cover your emirate | Decides which licence protects you and which products you can use | Check the VARA, CMA and ADGM public registers and your account's legal entity |
| Perpetuals enabled for your account | Access follows a suitability check and can be revoked | In-app, after the education module |
| Real fee tier, spot and perpetual | The whole cost model rests on it | Account fee page and the API fee endpoint |
| `ctVal`, `lotSz`, `minSz`, `tickSz` for BTC and ETH spot and swap | Feasibility at 100 USD | OKX public instruments endpoint |
| Demo trading, cancel-all-after and spot conditional orders on your entity | Rehearsal and the exchange-side kill layers | OKX API documentation |
| Free history: candles, funding, open interest, liquidations | Phase 2 and 3 data | Exchange public endpoints and data archives, including licence terms |
| Bybit's VARA status | Whether it can be a second venue | VARA public register |

**Optional later: X data.** The X API is credit-based pay-per-usage with no monthly minimum. Posts delivered through the filtered stream each count toward billing, with a monthly cap of 3 million post reads, and current prices are only in the developer console ([X usage and billing docs](https://docs.x.com/x-api/fundamentals/post-cap.md)). Third-party pages quote about 0.005 USD per post read, which would add up quickly on prolific accounts. It is out of scope until the strategy is live and measured value justifies the cost, and then it is used only for risk-off flags, never for direction.

**Sources (pages opened on 2026-11-07):**

- [Best Crypto Exchanges in the UAE (2026), Coinperps](https://www.coinperps.com/learn/best-crypto-exchanges-uae): licences, spot and derivatives fees, 5x cap, market prices used in the feasibility table.
- [Dubai's Derivatives Framework, Mondaq](https://www.mondaq.com/commoditiesderivativesstock-exchanges/1770516/dubais-derivatives-framework-vara-is-now-open-for-crypto-futures-perps-options-and-cfds): VARA derivatives rulebook, leverage cap, suitability, intervention powers.
- [X API usage and billing, docs.x.com](https://docs.x.com/x-api/fundamentals/post-cap.md): pay-per-usage billing and the filtered-stream charge.

This plan is not financial, legal or tax advice.
