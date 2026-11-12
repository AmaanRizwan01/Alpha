# Quantitative Strategy Research & Specification: Multi-Strategy Expansion Portfolio

**Document Version:** 1.0.0  
**Target Environment:** Project Alpha (Rust `#![forbid(unsafe_code)]` + Python Polars/PyO3)  
**Execution Node:** Proxmox VE Debian 12 LXC (CTID 101, Ajman, UAE)  
**Primary Deliverable:** Systematic Research, Mathematical Specification & System Integration Blueprint  

---

## 1. Executive Summary

Project Alpha is a production-grade, scale-invariant ($100 \rightarrow \$10,000+$), spot algorithmic trading system currently operating under 24/7 autonomous paper burn-in (Phase 5, Day 3 of 56) on a dedicated Proxmox VE Debian 12 LXC container in Ajman, UAE. The live production engine executes a single core strategy: **$H_1$ (Regime-Gated Donchian Volatility Breakout)** on a 4-hour swing horizon across `BTC/USDT` and `ETH/USDT`.

While $H_1$ successfully passed all quantitative kill gates—generating an out-of-sample net expectancy of $+0.207R$, a profit factor of $1.96$, and surviving a 10,000-resample stationary block bootstrap ($\text{LCB}_{95\%} = +0.0076R$) and Deflated Sharpe Ratio test ($\text{DSR} = 0.8674$)—operating a single-strategy portfolio introduces structural vulnerability:
1. **Regime Dependency:** $H_1$ is a directional trend-breakout strategy. During prolonged macro consolidation, low-volatility regimes, or choppy sideways compressions, the Kaufman Efficiency Ratio ($\text{ER}_{24} \ge 0.30$) and anti-chop gates force the system to sit 100% flat in cash for weeks.
2. **Asset Concentration:** Exposure is strictly confined to Bitcoin and Ethereum spot pairs.
3. **Capacity & Idle Capital:** When $H_1$ is flat, 100% of capital sits idle (earning only base exchange flexible earn yield), missing orthogonal market inefficiencies such as structural futures basis, cross-sectional altcoin dispersion, liquidation flush reversals, and statistical cointegration.

This document presents **six distinct, empirically grounded, and mathematically rigorous quantitative trading strategies** designed to integrate natively into Project Alpha’s existing Rust/Python architecture. Each candidate exploits a fundamentally different source of return, operates across distinct time horizons, exhibits near-zero or negative correlation with $H_1$, and relies exclusively on verifiable, high-integrity free data sources.

### Portfolio Composition at a Glance

| Strategy ID | Strategy Name | Primary Asset Class | Return Driver / Edge | Time Horizon | Expected Holding Period | Target Net Expectancy | Correlation to $H_1$ |
|---|---|---|---|---|---|---|---|
| **$H_1$ (Live)** | Regime-Gated Volatility Breakout | Crypto Spot (`BTC`, `ETH`) | Trend-following momentum post-compression | 4h bars | 1–5 days | $+0.207R$ (Verified) | $1.00$ (Benchmark) |
| **Strategy 1** | Cross-Sectional Crypto Momentum & Dispersion (`CS-MOM`) | Top 15 Crypto Spot Basket | Relative strength cross-sectional dispersion & capital rotation | 12h / 24h bars | 3–14 days | $+0.180R$ to $+0.250R$ | Low ($\rho \approx 0.25$) |
| **Strategy 2** | Spot-Perpetual Funding Rate Carry Arbitrage (`CARRY-ARB`) | Crypto Spot vs. Perp (`BTC`, `ETH`) | Delta-neutral structural retail leverage premium | 8h funding cycles | 3–30 days | $+0.120R$ to $+0.160R$ (Annualized $12\text{--}24\%$) | Near Zero ($\rho \approx 0.02$) |
| **Strategy 3** | Derivatives Liquidation Cascade Mean-Reversion (`FLUSH-REV`) | Crypto Spot & Perps (`BTC`, `ETH`) | Order book fire-sale liquidity provision & exhaustion | 1h bars (5m trigger) | 4–24 hours | $+0.220R$ to $+0.320R$ | Negative ($\rho \approx -0.30$) |
| **Strategy 4** | Macro Volatility-Regime Trend Following (`MACRO-TREND`) | Liquid Macro ETFs (`SPY`, `TLT`, `GLD`, `BTC`) | Real yield & sovereign liquidity transmission | Daily (1D close) | 2–8 weeks | $+0.150R$ to $+0.220R$ | Low-to-Moderate ($\rho \approx 0.20$) |
| **Strategy 5** | Intraday Session Turnover & Liquidity Imbalance (`SESSION-IMB`) | Crypto Spot (`BTC`, `ETH`, `SOL`) | Geographic time-of-day institutional order flow clustering | 1h bars | 3–7 hours (Intraday) | $+0.140R$ to $+0.190R$ | Low ($\rho \approx 0.15$) |
| **Strategy 6** | Statistical Cointegration & Pairs Trading (`COINT-PAIR`) | Synthetic Crypto Spreads (`ETH/BTC`, `SOL/ETH`) | Cointegration attractor mean reversion (Spot Alpha Rotation / Hedged Delta-Neutral) | 1h / 4h bars | 1–5 days | $+0.160R$ to $+0.240R$ | Near Zero ($\rho \approx -0.05$ Hedged; $\rho \approx 0.65$ Spot) |

---

## 2. Existing System Architecture and Compatibility Analysis

Before specifying any new candidate, the existing Project Alpha codebase was audited across every module in `rust/crates/` and `python/research/`.

### 2.1 Workspace and Crate Breakdown
The workspace enforces strict separation of concerns across five Rust crates and a Python research environment:

```
c:/Coding/alpha/
├── rust/crates/
│   ├── alpha-types/    # #![forbid(unsafe_code)] - Core domain primitives: Bar, MarketTrade, OrderIntent, Venue, Side, OrderType.
│   ├── alpha-core/     # #![forbid(unsafe_code)] - Pure quantitative logic: ChopFilter, H1 signal, Sizing, RiskPolicy, ExitConfig.
│   ├── alpha-py/       # #![deny(unsafe_op_in_unsafe_fn)] - PyO3 FFI exposing Rust functions to Python.
│   ├── alpha-venues/   # #![forbid(unsafe_code)] - Exchange protocols: Binance REST/WS, OKX v5 REST/WS, OTOCO/attachAlgoOrds signers.
│   └── alpha-node/     # #![forbid(unsafe_code)] - 24/7 daemon: SQLite WAL (db.rs), BarBuffer, PaperSimulator, TradeEngine, Telegram bot.
├── python/
│   ├── research/       # download_history.py, walkforward.py, run_walkforward.py, validate_gates.py.
│   └── scripts/        # phase0_feasibility.py (live fee & exchange filter scanner).
└── configs/            # paper.toml, live_100.toml, calendar_blackouts.json.
```

### 2.2 System Invariants & Architectural Laws
Every new strategy must strictly obey the established system laws defined in [`GEMINI.md`](file:///c:/Coding/alpha/GEMINI.md) and [`PLAN.md`](file:///c:/Coding/alpha/PLAN.md):

1. **The `#![forbid(unsafe_code)]` Crate Split Law:**
   `alpha-types`, `alpha-core`, `alpha-venues`, and `alpha-node` must compile under `#![forbid(unsafe_code)]` with zero manual `unsafe {}` blocks and zero `.unwrap()` or `.expect()` calls in production paths. Only `alpha-py` allows PyO3 procedural macro expansion.
2. **Financial Math & Precision Boundary:**
   All monetary amounts, order prices, execution quantities, balances, and exchange filters must use `rust_decimal::Decimal`. `f64` is restricted to statistical estimators (Z-score, Sharpe, Efficiency Ratio) and must validate `.is_finite()` before type conversion.
3. **Scale Invariance ($100 \rightarrow \$10,000+$):**
   No hardcoded dollar amounts in strategy logic. Sizing is governed by equity fraction ($E_t$), risk fraction ($r \in [0.0075, 0.010]$), structural stop distance ($D_{\text{stop}} = \frac{|P_e - P_s|}{P_e}$), and unit risk loss:
   $$\text{Qty} = \left\lfloor \frac{E_t \cdot r}{D_{\text{stop}} + C_{\text{rt}}} \cdot \frac{1}{\text{step\_size}} \right\rfloor \cdot \text{step\_size}$$
   Every trade must clear exchange `min_qty` and `min_notional` ($\ge \$5.00$).
4. **The Fee-to-Stop Ratio Law ($C_{\text{rt}} / D_{\text{stop}}$):**
   Empirically proven in Phase 2: a strategy with round-trip friction $C_{\text{rt}} = 23\text{ bps}$ and an average stop of $1.25\%$ loses $0.184R$ to fees per trade, destroying profitability on `1h` bars. To remain viable, any swing strategy must enforce a structural stop floor:
   $$D_{\text{stop}} \ge 2.20\% \implies \text{Fee Drag } \le \frac{0.0023}{0.0220} = 0.104R$$
5. **Atomic Order Submission:**
   Entries must atomically bind protective stops at submission via OKX `attachAlgoOrds` or Binance `POST /api/v3/orderList/otoco`. No unprotected positions may exist if server power or network severs.

### 2.3 Current System Limitations to Address
1. **Single-Strategy Hardcoding:**
   In [`alpha-node/src/engine.rs`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/engine.rs), `TradeEngine` holds a single `H1Config` and directly invokes `crate::signal_dispatch::evaluate_and_dispatch`. Integrating 5–6 strategies requires abstracting signal evaluation behind a unified `Strategy` trait or enum dispatch.
2. **Timeframe Rigidity:**
   `AppConfig` specifies a single `universe.timeframe = "4h"`. Multi-timeframe execution requires `BarBuffer` instances keyed by both symbol and timeframe `(Symbol, Timeframe)`.
3. **Data Schema Specificity:**
   `alpha_types::Bar` includes `(open, high, low, close, volume, taker_buy_volume, quote_volume, trades_count)`. Non-kline signals (funding rates, open interest, ETF macro prices, Treasury yields) require typed auxiliary event structures or secondary ingestion tables in SQLite.
4. **Single-Position Concurrency Lock:**
   In [`alpha-node/src/simulator.rs`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/simulator.rs) (`pub active_position: Option<PaperPosition>`) and [`alpha-node/src/engine.rs`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/engine.rs) (`if self.simulator.active_position.is_some() || self.simulator.working_order.is_some() { return Ok(()); }`), the engine aborts evaluation whenever any single position or working order exists across any asset. Multi-strategy operation requires refactoring to a concurrent position map keyed by `(StrategyId, Symbol)`.

---

## 3. Research Methodology

To maintain intellectual honesty and prevent the common pitfalls of retail backtesting, our quantitative research protocol follows a five-stage verification pipeline:

```
[Academic Literature & Economic Rationale]
                  │
                  ▼
[Mathematical Formulation & Deterministic Rules]
                  │
                  ▼
[Free Historical Data Ingestion & Sanitization]
                  │
                  ▼
[Purged Walk-Forward Simulation (alpha-py / Polars)]
                  │
                  ▼
[Statistical Kill Gate Evaluation (Bootstrap, DSR, Perturbation)]
```

### 3.1 Strict Anti-Overfitting Controls
1. **Zero Look-Ahead Bias:**
   All features computed at bar $t$ access only information with timestamp $\le t_{\text{close}}$. An order triggered at bar $t$ close executes at the open price of bar $t+1$ (marketable) or rests passively on the limit book.
2. **Purged Walk-Forward Slicing:**
   Evaluated using rolling 12-month training / 3-month testing folds with a **48-hour purge/embargo gap** to eliminate serial correlation across train/test boundaries.
3. **Statistical Significance Standards:**
   - Sample size target: $N \ge 100$ independent trades across out-of-sample folds.
   - Stationary Block Bootstrap (Politis & Romano 1994): 10,000 resamples; 95% lower confidence bound must satisfy $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > 0.00R$.
   - Deflated Sharpe Ratio (Bailey & López de Prado 2014): $\text{DSR} > 0.85$ accounting for number of trials $K$, variance of trials, skewness, and kurtosis.
   - Parameter Perturbation: Strategy must survive $\pm 20\%$ parameter shifts without cliff-edge failure.

---

## 4. Strategy Selection Rationale

The six strategies were selected to construct an all-weather portfolio spanning orthogonal dimensions:
- **Momentum vs. Mean Reversion:** $H_1$ and Strategy 1 capture trend expansions; Strategies 3 and 6 exploit mean reversion.
- **Directional vs. Delta-Neutral:** $H_1$, 1, 3, 4, 5 take directional market exposure; Strategy 2 (Carry) and Strategy 6 (Pairs) are market-neutral ($\Delta \approx 0$).
- **Intraday vs. Swing:** Strategy 5 flattens within 3–7 hours (zero overnight risk); Strategy 4 holds for multi-week macro cycles.
- **Microstructure vs. Macro:** Strategy 3 exploits liquidation order-book fire sales; Strategy 4 trades interest rate cycles.

---

## 5. Strategy 1 — Complete Specification
### Cross-Sectional Crypto Momentum & Dispersion (`CS-MOM`)

### A. Strategy Overview
- **Name:** Cross-Sectional Crypto Momentum & Dispersion (`CS-MOM`).
- **Category:** Relative-Value / Cross-Sectional Momentum Factor.
- **Core Hypothesis:** Relative winners across a universe of liquid crypto assets continue to outperform relative losers over a 3- to 14-day rebalancing horizon due to structural capital rotation, institutional fund flows, and retail attention contagion.
- **Economic Rationale:** In the cryptocurrency ecosystem, capital does not enter all assets simultaneously. Market expansions typically initiate in Bitcoin, rotate into Ethereum, cascade into high-beta Layer-1/DeFi tokens, and eventually disperse into long-tail assets. Behavioral research (Hong & Stein 1999; Barber & Odean 2008) documents that slow information diffusion and attention-driven trading generate persistent multi-day cross-sectional lead-lag relationships.
- **Source of Expected Return:** Cross-sectional return dispersion between top-decile relative strength assets and the broad market benchmark, capturing the cross-sectional momentum risk premium identified by Liu, Tsyvinski, and Wu (2022).
- **Why Opportunity Exists & Persists:** Crypto markets remain fragmented with high retail participation, high leverage, and segmented capital pools. Capital rotation cascades are amplified by exchange listing dynamics and narrative-driven momentum.
- **Why It Might Disappear:** Institutional multi-strategy hedge funds deploying cross-sectional statistical arbitrage could compress dispersion speed, reducing the half-life of cross-sectional leads.

### B. Appropriate Markets
- **Target Universe:** Top 15 liquid spot pairs quoted in `USDT` or `FDUSD` on Binance/OKX:
  `BTC`, `ETH`, `SOL`, `BNB`, `AVAX`, `LINK`, `DOT`, `NEAR`, `ADA`, `MATIC/POL`, `DOGE`, `LTC`, `BCH`, `UNI`, `XRP`.
- **Exclusions:** Stablecoins, wrapped tokens (`WBTC`), algorithmic liquid staking derivatives, and tokens with 24-hour spot volume $<\$15\text{M}$.
- **System Compatibility:** Spot only; fully compatible with existing Binance/OKX spot connectors and UAE zero-tax regulatory guidelines.

### C. Timeframes and Holding Periods
- **Signal Calculation Frequency:** Calculated every 12 hours (at 00:00 and 12:00 UTC) using aggregated 4h bars.
- **Trading Decision Frequency:** Evaluated bi-weekly or weekly (rebalanced every 7 days, or dynamically on rank dislocation).
- **Expected Holding Period:** 3 to 14 days (mean holding period: 7 days).
- **Minimum Required Lookback:** 30 days (180 four-hour bars).
- **Maximum Holding Period:** 21 days (mandatory rotation).

### D. Data Requirements
| Data Field | Description | Frequency | Coverage | Source | Mandatory? | Free Cost? |
|---|---|---|---|---|---|---|
| `close`, `volume` | 4h OHLCV for top 15 spot pairs | 4h | 2021–Present | Binance Public REST / Vision | Yes | Yes (100% Free) |
| `BTCUSDT` 4h OHLCV | Benchmark market regime filter | 4h | 2021–Present | Binance Public REST / Vision | Yes | Yes (100% Free) |

### E. Mathematical Formulation
1. **Log Return Calculation:**
   For each asset $i \in \{1, \dots, K\}$ ($K = 15$) over lookback window $L = 42$ four-hour bars (7 days):
   $$R_{i, t}^{(L)} = \ln\left(\frac{P_{i, t}}{P_{i, t-L}}\right)$$

2. **Volatility Standardization (Idiosyncratic Risk Adjustment):**
   Compute realized volatility over lookback $L_{\text{vol}} = 180$ four-hour bars (30 days):
   $$\sigma_{i, t} = \sqrt{\frac{1}{L_{\text{vol}}-1} \sum_{\tau=0}^{L_{\text{vol}}-1} \left( r_{i, t-\tau} - \bar{r}_{i, t} \right)^2}$$
   Standardized Momentum Score:
   $$Z_{i, t} = \frac{R_{i, t}^{(L)}}{\sigma_{i, t} \cdot \sqrt{L}}$$

3. **Cross-Sectional Rank Percentile:**
   Rank scores ascending: $\text{Rank}(Z_{i, t}) \in [1, K]$.
   $$\text{Percentile}_{i, t} = \frac{\text{Rank}(Z_{i, t}) - 1}{K - 1} \in [0.0, 1.0]$$

4. **Macro Regime Gate (Market Beta Filter):**
   The strategy only deploys capital when the broader market is in a structural expansion:
   $$\text{Regime Gate } G_t = \begin{cases} 1 & \text{if } P_{\text{BTC}, t} > \text{SMA}_{300}(P_{\text{BTC}}) \text{ and } \text{ER}_{\text{BTC}, 24} \ge 0.28 \\ 0 & \text{otherwise} \end{cases}$$
   If $G_t = 0$, the portfolio closes positions and remains 100% flat in cash.

5. **Portfolio Weight Allocation:**
   Select the top $N = 3$ assets with $\text{Percentile}_{i, t} \ge 0.80$.
   Allocate via inverse-volatility weighting:
   $$w_{i, t} = G_t \cdot \frac{\frac{1}{\sigma_{i, t}}}{\sum_{j \in \text{Top } N} \frac{1}{\sigma_{j, t}}}$$
   Cap maximum single-asset weight at $w_{\max} = 0.35$ (35% of active equity).

### F. Exact Trading Rules
- **Entry Rules:**
  1. $G_t == 1$ (BTC is above its 50-day SMA and not in chop).
  2. Asset $i$ ranks in the top 3 ($\text{Percentile}_{i, t} \ge 0.80$).
  3. Asset $i$ 24-hour volume $> 1.20 \times \text{SMA}_{20}(V_i)$.
  4. Submit atomic limit entry order at `best_bid` via `Post-Only`.
- **Exit Rules:**
  1. **Rank Invalidation Exit:** Exit if asset drops out of top quintile ($\text{Percentile}_{i, t} < 0.60$).
  2. **Regime Collapse Exit:** Exit all positions immediately if $G_t == 0$ (BTC falls below 50-day SMA).
  3. **Trailing Structural Stop:** $E_0$ Chandelier Stop at $P_{s, t} = \max_{\tau \le t}(P_{\text{high}, \tau}) - 2.5 \times \text{ATR}_{14}(i)$, with minimum stop floor $D_{\text{stop}} \ge 3.0\%$.
  4. **Emergency Stop:** Hard stop-loss at $-1.0R$ from initial entry.
- **Position Sizing:**
  Risk per asset capped at $r = 0.0075$ (0.75% of equity) with maximum gross portfolio exposure $\sum w_i \le 0.75$ (75% equity, retaining 25% cash reserve).

### G. Execution Requirements
- Order type: `LIMIT_MAKER` / `Post-Only` at the best bid touch.
- Cancel/replace if unfilled after 15 minutes; fall back to marketable limit collared at $\le 5\text{ bps}$ slippage.
- Exchange filters: Enforce `step_size`, `tick_size`, and Binance $\$5.00$ `minNotional`.

### H. Parameter Specification
| Parameter | Description | Baseline | Sensitivity Range | Calibration Rule |
|---|---|---|---|---|
| $L$ | Momentum Lookback | 42 bars (7 days) | 30 to 60 bars | Fixed (Literature standard) |
| $L_{\text{vol}}$ | Volatility Lookback | 180 bars (30 days) | 120 to 240 bars | Fixed |
| $N$ | Selected Basket Size | 3 assets | 2 to 5 assets | Fixed |
| $w_{\max}$ | Max Position Weight | 0.35 | 0.25 to 0.40 | Risk Policy Hard Limit |
| Stop ATR Mult | Chandelier Multiplier | 2.50 | 2.0 to 3.0 | Calibrated via walk-forward |

### I. Failure Modes
- **Correlated Flash Crash:** Sudden market-wide liquidation cascades lead to simultaneous stop-outs across all top-decile tokens. Controlled via the BTC macro gate $G_t$ and independent structural stops.
- **Dispersion Compression:** In a sideways market with low liquidity, altcoin beta converges to noise, causing whipsaws. Controlled via $G_t$ and the minimum volume gate.

### J. Backtesting and Validation
- **Engine:** Vectorized ranking in Polars, out-of-sample execution simulation via `alpha-py`.
- **Friction Model:** 8 bps maker entry + 10 bps taker stop exit + 5 bps slippage = 23 bps round-trip.
- **Walk-Forward Schedule:** 18 rolling folds (12m train, 3m test, 48h purge gap).
- **Statistical Gate:** 10,000-resample stationary block bootstrap requiring $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > +0.02R$.

### K. Integration with Existing System
- **Reusability:** Reuses `alpha_core::sizing`, `alpha_core::exits`, and `alpha_types::Bar`.
- **New Rust Components:** Requires a cross-sectional ranking module in `alpha-core` (`src/cs_momentum.rs`) consuming a slice of slices `&[&[Bar]]` and returning ranked weights.
- **Complexity:** Medium (pure analytical ranking, no new exchange order types required).

---

## 6. Strategy 2 — Complete Specification
### Spot-Perpetual Funding Rate Carry Arbitrage (`CARRY-ARB`)

### A. Strategy Overview
- **Name:** Spot-Perpetual Funding Rate Carry Arbitrage (`CARRY-ARB`).
- **Category:** Structural Arbitrage / Delta-Neutral Carry.
- **Core Hypothesis:** Perpetual futures structurally trade at a premium to spot prices during bull and normal market regimes because retail market participants pay a persistent financing charge (the funding rate, exchanged every 8 hours) to maintain leveraged long positions without delivery. A delta-neutral long spot vs. short perpetual position collects this structural cash flow.
- **Economic Rationale:** Limits to arbitrage (Shleifer & Vishny 1997) prevent market makers from fully compressing the funding spread: institutional capital faces borrowing constraints, regulatory barriers, and counterparty risks. The funding payment represents an economic risk premium paid by liquidity-consuming leveraged speculators to liquidity-supplying balance-sheet providers (Fama & French 1987; Borri & Shakhnov 2022).
- **Source of Expected Return:** Direct receipt of the 8-hour funding cash flow:
  $$\text{Cash Flow} = \text{Notional} \times F_{8\text{h}}$$
- **Why It Might Disappear:** Severe prolonged bear markets where funding rates invert to negative (shorts pay longs) for months, or massive institutional capital inflows compressing perpetual basis to zero.

### B. Appropriate Markets
- **Target Universe:** `BTC/USDT` (Spot) vs. `BTC-USDT-SWAP` / `BTCUSDT Perpetual` (Futures) on OKX or Binance.
- **Asset Classes:** Spot Crypto + Perpetual Futures.
- **System Compatibility:** Requires enabling the perpetual futures API on Binance (`fapi.binance.com`) or OKX unified account mode. Compatible with Proxmox LXC execution.

### C. Timeframes and Holding Periods
- **Signal Calculation Frequency:** Evaluated every 8 hours at funding timestamps (00:00, 08:00, 16:00 UTC).
- **Trading Decision Frequency:** Every 8 to 24 hours.
- **Expected Holding Period:** 5 to 45 days (maintained as long as funding remains positive).
- **Minimum Required Lookback:** 9 funding cycles (72 hours).

### D. Data Requirements
| Data Field | Description | Frequency | Coverage | Source | Mandatory? | Free Cost? |
|---|---|---|---|---|---|---|
| `fundingRate` | 8h perpetual funding rate history | 8h | 2020–Present | Binance `/fapi/v1/fundingRate` | Yes | Yes (100% Free) |
| Spot OHLCV | Spot klines for basis monitoring | 1h | 2020–Present | Binance `/api/v3/klines` | Yes | Yes (100% Free) |
| Perp OHLCV | Mark & Index price klines | 1h | 2020–Present | Binance `/fapi/v1/klines` | Yes | Yes (100% Free) |

### E. Mathematical Formulation
1. **Annualized Predicted Funding Rate:**
   Let $F_{8\text{h}, t}$ be the observed funding rate. The rolling 3-day exponential moving average is:
   $$\bar{F}_{t} = \text{EMA}(F_{8\text{h}}, \alpha = 0.20) \quad (\approx 9 \text{ periods})$$
   $$\text{APR}_t = \bar{F}_t \times 3 \times 365$$

2. **Friction-Adjusted Hurdle Condition:**
   Round-trip execution incurs spot fee ($C_{\text{spot}} = 0.08\%$), futures taker fee ($C_{\text{fut}} = 0.04\%$), and basis slippage ($S = 0.04\%$):
   $$C_{\text{total\_rt}} = 2 \times (C_{\text{spot}} + C_{\text{fut}} + S) = 2 \times 0.0016 = 0.32\%$$
   The trade only enters when expected 14-day funding yields at least $2.5 \times$ total transaction costs:
   $$\text{Entry Threshold: } \text{APR}_t \ge 12.0\% \quad \left(\bar{F}_t \ge 0.01096\% \text{ per 8h}\right)$$

3. **Delta-Neutral Sizing & Unified/Portfolio Margin Architecture:**
   Let total capital allocated to carry be $C_{\text{carry}}$.
   To achieve true delta neutrality ($\Delta_{\text{net}} = 0$) while deploying $L_{\text{margin}} = 2.0\times$ margin leverage on the perpetual short leg, capital is allocated in an exact $2:1$ ratio:
   $$E_{\text{spot}} = \frac{2}{3} C_{\text{carry}}, \quad E_{\text{margin}} = \frac{1}{3} C_{\text{carry}}$$
   - **Spot Leg Notional:**
     $$N_{\text{spot}} = E_{\text{spot}} = \frac{2}{3} C_{\text{carry}}, \quad Q_{\text{spot}} = \frac{N_{\text{spot}}}{P_{\text{spot}}}$$
   - **Perpetual Short Notional:**
     With $E_{\text{margin}} = \frac{1}{3} C_{\text{carry}}$ deposited at $2.0\times$ leverage:
     $$N_{\text{perp}} = E_{\text{margin}} \times 2.0 = \frac{2}{3} C_{\text{carry}}, \quad Q_{\text{perp}} = \frac{N_{\text{perp}}}{P_{\text{perp}}}$$
   - **Net Portfolio Delta:**
     $$\Delta_{\text{net}} = Q_{\text{spot}} \cdot P_{\text{spot}} - Q_{\text{perp}} \cdot P_{\text{perp}} = \frac{2}{3} C_{\text{carry}} - \frac{2}{3} C_{\text{carry}} = 0$$
   - **Unified/Portfolio Margin Specification (Zero Directional Liquidation):**
     Under Binance Portfolio Margin (PM) or OKX Unified Account (Multi-Currency Margin mode), spot holdings ($Q_{\text{spot}}$ BTC/ETH) are pledged directly into the unified margin pool as collateral with an exchange valuation haircut ($h \approx 0.95$). As the spot price rallies, the USD collateral value of the long spot leg increases by $+\Delta P \times Q_{\text{spot}} \times h$, directly and automatically offsetting the unrealized paper loss on the short perpetual leg ($-\Delta P \times Q_{\text{perp}}$). Because $Q_{\text{spot}} \approx Q_{\text{perp}}$, the account margin ratio remains constant whether BTC trades at $60,000 or $150,000, eliminating directional liquidation risk and removing the need for manual margin transfers.

4. **Basis Dislocation Guard:**
   Define spot-perp basis:
   $$B_t = \frac{P_{\text{perp}, t} - P_{\text{spot}, t}}{P_{\text{spot}, t}}$$
   Do not enter if $B_t < -0.05\%$ (backwardation entry penalty).

5. **Fee-Hurdle Lock Formulation:**
   Let gross funding yield accumulated since entry be:
   $$G_{\text{carry}} = \frac{2}{3} \sum_{k=t_{\text{entry}}}^{t} F_{8\text{h}, k}$$
   Total round-trip fee friction for the deployed position is:
   $$\text{Fee}_{\text{total}} = \frac{2}{3} \times C_{\text{total\_rt}} \approx 0.2133\% \quad (21.3\text{ bps})$$
   When the smoothed funding rate decays below the exit threshold ($\text{APR}_t \le 2.2\%$) for 3 consecutive epochs, the exit is locked (`Fee-Hurdle Lock`) if:
   $$G_{\text{carry}} < \text{Fee}_{\text{total}} \times \text{fee\_hurdle\_mult} \quad (\text{with } \text{fee\_hurdle\_mult} = 1.0)$$
   The position continues holding and accumulating positive funding until $G_{\text{carry}} \ge \text{Fee}_{\text{total}}$, guaranteeing that positive funding decay exits never lock in a net fee loss.
   *Bleed Stop Exception:* If instantaneous funding flips negative ($F_{8\text{h}, t} < \text{inversion\_stop} = -0.015\%$), the `FundingInversionStop` fires immediately regardless of the fee hurdle to prevent adverse carry bleed.

6. **Predictive Funding Pre-Exit Formulation:**
   Binance Futures publishes estimated funding rates in real time via `GET /fapi/v1/premiumIndex` and the mark price WebSocket stream.
   - **Evaluation Time:** Exactly 10 minutes prior to the funding settlement cutoff (at 07:50, 15:50, and 23:50 UTC).
   - **Condition:** If the predicted next-period funding rate $F_{\text{predicted}} < -0.0050\%$ (indicating shorts will pay longs), trigger an immediate dual-leg pre-exit.
   - **Impact:** Closes both legs before the funding payment is deducted at 00:00, 08:00, or 16:00 UTC, eliminating negative carry payment.

### F. Exact Trading Rules
- **Entry Rules:**
  1. $\bar{F}_t \ge 0.0110\%$ per 8h ($\approx 12.0\%$ APR).
  2. Spot-Perp basis $B_t \ge 0.00\%$.
  3. Margin health ratio $> 3.0$ on unified account.
  4. Submit simultaneous two-leg atomic order:
     - Leg 1: Buy $Q$ units spot via `Post-Only` limit at `best_bid`.
     - Leg 2: Upon fill of Leg 1, immediately short $Q$ units perpetual futures via marketable limit.
- **Exit Rules:**
  1. **Funding Decay Exit (Gated by Fee-Hurdle Lock):** Exit when $\bar{F}_t \le 0.002\%$ per 8h ($\approx 2.2\%$ APR) for 3 consecutive funding periods, *provided* that accumulated gross funding covers total round-trip fee friction ($G_{\text{carry}} \ge \text{Fee}_{\text{total}} \times \text{fee\_hurdle\_mult}$). If fees are not yet covered, continue holding to capture positive cash flow.
  2. **Funding Inversion Stop:** Exit immediately if $F_{8\text{h}, t} < -0.015\%$ (negative funding; shorts pay longs), overriding fee-hurdle lock to prevent bleed.
  3. **Predictive Funding Pre-Exit:** 10 minutes before funding settlement (07:50, 15:50, 23:50 UTC), query `GET /fapi/v1/premiumIndex`. If predicted rate $< -0.0050\%$, execute pre-exit before negative funding settles.
  4. **Margin Rebalance / Unified Mode:** Under Unified/Portfolio Margin mode, collateral adjusts automatically without manual transfers.
- **Position Sizing:** Total capital allocation $C_{\text{carry}}$ is sized as a fixed fraction of total portfolio equity ($20\%\text{--}40\%$). Capital is split $E_{\text{spot}} = \frac{2}{3} C_{\text{carry}}$ and $E_{\text{margin}} = \frac{1}{3} C_{\text{carry}}$ with $2.0\times$ leverage on the short leg. Zero stop-loss in price space; exit is governed purely by funding rate dynamics.

### G. Execution Requirements
- Requires Binance Futures or OKX Unified Trading Account API keys with Portfolio Margin enabled.
- Fills must be executed with maker priority on spot to harvest rebates ($0\text{ bps}$ on FDUSD pairs).
- **Unified / Portfolio Margin Setup:** Combine spot and perp collateral into a single margin pool so that rising spot value automatically backs the short perp, eliminating price liquidation risk.
- **Live WebSocket ADL Desync Watchdog in `alpha-node`:**
  - Real-time monitoring of user execution streams (`ORDER_TRADE_UPDATE` on Binance, `orders`/`positions` on OKX).
  - If the perpetual short leg is ever subject to Auto-Deleveraging (ADL), exchange liquidation, or unexpected fill mismatch such that $|Q_{\text{perp}}| < 0.99 \times Q_{\text{spot}}$, the watchdog triggers an **instant emergency market sell of spot within 100ms**.
  - Persists `HALTED_ADL_DESYNC` in SQLite, cancels open orders, and dispatches an emergency priority alert via Telegram.

### H. Parameter Specification
| Parameter | Baseline | Sensitivity Range | Description |
|---|---|---|---|
| Entry APR Threshold | $12.0\%$ | $10.0\%\text{--}16.0\%$ | Minimum annual funding rate to enter |
| Exit APR Threshold | $2.2\%$ | $1.0\%\text{--}4.0\%$ | Rate at which carry no longer compensates capital |
| Fee-Hurdle Multiplier | $1.0\times$ | $0.8\times\text{--}1.2\times$ | Minimum fee coverage before decay exit unlocks |
| Predictive Pre-Exit Cutoff | 10 mins prior | 5 to 15 mins | Lead time to exit before negative funding settles |
| Funding Smoothing Lookback | 9 periods (72h) | 6 to 15 periods | Filters out single-period funding spikes |
| Max Perp Leverage | $2.0\times$ | $1.5\times\text{--}2.5\times$ | Guarantees liquidation safety |

### I. Failure Modes
- **Exchange Counterparty Risk:** Holding spot and futures on the same exchange exposes capital to exchange solvency risk. Mitigated by using top-tier venues (Binance/OKX).
- **Extreme Funding Inversion:** In severe prolonged bear markets (e.g., May–July 2021, Nov 2022), funding can turn negative for weeks. Controlled by Predictive Pre-Exit and the strict Funding Inversion Stop.
- **Execution Leg Risk:** If the spot order fills and the futures market gaps before Leg 2 fills, unhedged delta exists. Mitigated by automated marketable limit execution on Leg 2 within 100ms.
- **Auto-Deleveraging (ADL) Desync:** If exchange ADL forcibly reduces the short perp leg, naked spot long exposure arises. Controlled by the sub-100ms ADL Desync Watchdog in `alpha-node` which instantly market-sells the corresponding spot leg.

### J. Backtesting and Validation
- Historical funding datasets from 2020–2026 evaluated across rolling 8h epochs.
- Validated metric: Realized Net Carry Yield after full maker/taker fees and basis slippage. Target: Sharpe $> 3.50$, Max Drawdown $< 2.0\%$.

### K. Integration with Existing System
- **Reusability:** Leverages `alpha-venues` signing logic and `alpha-node` SQLite state recovery.
- **New Rust Components:** Requires adding perpetual futures REST/WS endpoints to `alpha-venues` (`fapi.rs`), the sub-100ms ADL desync watchdog in `alpha-node`, and a dedicated delta-neutral state manager in `alpha-core`.
- **Complexity:** Medium-High (requires dual-leg order coordination across two market endpoints).

---

## 7. Strategy 3 — Complete Specification
### Derivatives Liquidation Cascade Mean-Reversion (`FLUSH-REV`)

### A. Strategy Overview
- **Name:** Derivatives Liquidation Cascade & Flush Mean-Reversion (`FLUSH-REV`).
- **Category:** Market Microstructure / Fire-Sale Liquidity Provision.
- **Core Hypothesis:** Cascading mechanical liquidations on leveraged derivatives venues push prices temporarily far below fundamental equilibrium due to order book liquidity exhaustion. When forced liquidation volume subsides and aggressive absorption emerges, prices mean-revert rapidly.
- **Economic Rationale:** Under exchange auto-liquidation protocols, bankrupt accounts are taken over by exchange liquidation engines, which issue aggressive market orders that consume all available depth. Coval and Stafford (2007) and Brunnermeier and Pedersen (2009) document that fire sales by distressed market participants create severe temporary mispricings. Providing liquidity during these localized liquidity crises yields substantial positive excess returns.
- **Source of Expected Return:** Liquidity provision premium earned by absorbing non-economic distressed selling and capturing the subsequent mean-reversion snapback.
- **Why It Exists & Persists:** Crypto derivatives feature leverage up to $50\times\text{--}100\times$, clustered stop-losses, and automated liquidation engines. Retail traders persistently over-leverage at technical support levels, guaranteeing future liquidation cascades.
- **Why It Might Disappear:** Exchange matching engines adopting cross-exchange liquidation internalization or smoother TWAP liquidation execution.

### B. Appropriate Markets
- **Target Universe:** `BTC/USDT` and `ETH/USDT` Spot (buying spot during derivatives flushes eliminates liquidation risk on the strategy itself).
- **System Compatibility:** 100% compatible with existing `alpha-venues` Binance/OKX spot architecture.

### C. Timeframes and Holding Periods
- **Signal Calculation Frequency:** 1-hour bar closes, with 5-minute micro-bar confirmation.
- **Trading Decision Frequency:** Event-driven (infrequent: 2–6 trades per month across BTC and ETH).
- **Expected Holding Period:** 4 to 24 hours (fast mean-reversion).
- **Minimum Required Lookback:** 30 days (for rolling funding and open interest distributions).
- **Maximum Holding Period:** 48 hours (mandatory time-based cut).

### D. Data Requirements
| Data Field | Description | Frequency | Source | Mandatory? | Free Cost? |
|---|---|---|---|---|---|
| Spot OHLCV | 1h and 5m spot price bars | 1h / 5m | Binance Public REST (`/api/v3/klines`) | Yes | Yes (100% Free) |
| Perp Funding Rate | 8h funding rate & predicted rate | 8h / Real-time | Binance Futures REST (`/fapi/v1/fundingRate`) | Yes | Yes (100% Free) |
| Open Interest (OI) | Historical & Live Open Interest | 1h | Binance Vision (Monthly Metrics Archives) / Binance REST (`/fapi/v1/openInterest`) | Yes | Yes (100% Free) |

*(Note on Open Interest API & Historical Storage): The Binance Futures REST endpoint `/fapi/v1/openInterestHist` is strictly restricted to the rolling 30 days and returns 404 / rate-limit errors when queried for historical backtesting. For multi-year quantitative walk-forward validation (2020–2026), open interest series must be ingested from Binance Vision monthly metrics archives (`https://data.binance.vision/data/futures/um/monthly/metrics/` with symbol-specific archives containing 5-minute and 1-hour `sum_open_interest`). For live 24/7 forward evaluation in `alpha-node`, the engine queries the instantaneous `/fapi/v1/openInterest` endpoint at bar close.*

### E. Mathematical Formulation
1. **Downside Price Dislocation Metric:**
   Compute 24-hour cumulative price return:
   $$R_{24\text{h}, t} = \frac{P_{\text{close}, t} - P_{\text{close}, t-24}}{P_{\text{close}, t-24}} \le -4.50\%$$

2. **Funding Rate Z-Score:**
   Compute the 30-day rolling Z-score of perpetual funding rates:
   $$Z_{F, t} = \frac{F_t - \mu_F(30\text{d})}{\sigma_F(30\text{d})} \le -1.80$$
   This guarantees that perpetual funding is in its bottom 5th percentile, indicating extreme short crowding and aggressive short liquidations.

3. **Open Interest Contraction (Deleveraging Confirmation):**
   A true liquidation cascade violently destroys open contracts:
   $$\Delta \text{OI}_{4\text{h}, t} = \frac{\text{OI}_t - \text{OI}_{t-4}}{\text{OI}_{t-4}} \le -3.00\%$$

4. **Order Book Absorption Candlestick Signature:**
   On the 1-hour trigger bar:
   - Lower Shadow Ratio:
     $$\text{Wick}_{\text{lower}} = \frac{\min(P_{\text{open}}, P_{\text{close}}) - P_{\text{low}}}{P_{\text{high}} - P_{\text{low}}} \ge 0.40 \quad (40\% \text{ of entire bar range})$$
   - Volume Spike:
     $$V_t \ge 2.00 \times \text{SMA}_{20}(V)$$
   - Taker Flow Absorption:
     $$\frac{V_{\text{taker\_buy}, t}}{V_t} \ge 0.52$$

5. **Structural Stop and Target Formulation (with Hard Floor Clamping):**
   - Entry Price: $P_e = P_{\text{close}, t}$
   - Structural Stop-Loss Placement:
     Let $P_{s, \text{raw}} = P_{\text{low}, t} \times (1 - 0.0020)$ (20 bps below the flush wick extreme).
     To guarantee strict mathematical compatibility with Project Alpha's sizing engine in `alpha_core::sizing::calculate_position_size`, the stop distance must never breach the minimum structural threshold $D_{\text{stop, min}} = \text{min\_stop\_distance\_pct} = 2.20\%$. The stop price is therefore clamped with a 5 bps tick buffer:
     $$P_s = \min\left(P_{\text{low}, t} \times (1 - 0.0020), \ P_e \times (1 - D_{\text{stop, min}} - 0.0005)\right)$$
     This guarantees:
     $$D_{\text{stop}} = \frac{P_e - P_s}{P_e} \ge D_{\text{stop, min}} = 2.20\%$$
     This mathematical clamp prevents `CoreError::InvalidParameter { name: "stop_distance", reason: "stop distance falls below minimum structural threshold" }` from rejecting valid trade setups on tight-wick bars or causing runtime engine panics.
   - Take-Profit Target: Half-back retracement of the liquidation breakdown:
     $$P_{\text{target}} = P_e + 1.80 \times (P_e - P_s) \quad (+1.80R)$$

### F. Exact Trading Rules
- **Entry Rules:**
  All five conditions must align simultaneously on a closed 1h bar:
  1. $R_{24\text{h}, t} \le -4.5\%$.
  2. $Z_{F, t} \le -1.80$.
  3. $\Delta \text{OI}_{4\text{h}, t} \le -3.0\%$.
  4. $\text{Wick}_{\text{lower}} \ge 0.40$ and $V_t \ge 2.0 \times \text{SMA}(V, 20)$.
  5. Taker buy volume ratio $\ge 0.52$.
  Submit atomic spot limit buy order with attached exchange-hosted stop at $P_s$.
- **Exit Rules:**
  1. **Take-Profit Limit:** Resting limit sell order at $P_{\text{target}} = P_e + 1.80R$.
  2. **Breakeven Ratchet:** When unrealized profit reaches $+1.00R$, ratchet stop to $P_e + 0.15\%$.
  3. **Stagnation Time Stop:** If after 12 bars (12 hours) profit has not reached $+0.50R$, exit at market.
  4. **Hard Stop:** Atomic exchange stop at $P_s$ triggered on low breach.
- **Position Sizing:**
  Standard scale-invariant risk budget: $r = 0.010$ (1.0% equity risk). Position notional $N = \frac{E_t \cdot r}{D_{\text{stop}} + C_{\text{rt}}}$.

### G. Execution Requirements
- Uses existing Binance/OKX Spot atomic orders (`OTOCO` / `attachAlgoOrds`).
- Requires immediate fill capability: limit order placed at current `best_ask` or top of book with maker priority timeout of 10 seconds.

### H. Parameter Specification
| Parameter | Baseline | Sensitivity Range | Description |
|---|---|---|---|
| $R_{24\text{h}}$ Drop | $-4.5\%$ | $-3.5\%\text{--}-6.0\%$ | Magnitude of preceding sell-off |
| $Z_{F}$ Threshold | $-1.80$ | $-1.50\text{--}-2.20$ | Funding rate extreme threshold |
| $\Delta \text{OI}_{4\text{h}}$ Contraction | $-3.0\%$ | $-2.0\%\text{--}-5.0\%$ | Minimum open interest destruction |
| Lower Wick Ratio | $0.40$ | $0.35\text{--}0.50$ | Buyer absorption evidence |
| Volume Multiplier | $2.0\times$ | $1.75\times\text{--}2.50\times$ | Volume surge requirement |

### I. Failure Modes
- **Macro Insolvency / True Structural Break:** Events like the FTX collapse or Terra-Luna collapse represent insolvency rather than temporary liquidity flushes. Consecutive liquidation cascades can cause multiple stops.
  *Defense:* Enforce maximum 1 flush trade attempt per 48 hours per asset. If stopped out, block re-entry until market consolidates for 24h.

### J. Backtesting and Validation
- Evaluated on historical Binance Vision 1h klines + Binance Futures historical funding and open interest Parquets (`2021`–`2026`).
- Pre-registered walk-forward testing: 18 folds. Statistical target: Win rate $\ge 52\%$, Net EV $\ge +0.22R$, DSR $> 0.90$.

### K. Integration with Existing System
- **Reusability:** 90% reusable. Shares exact `alpha_core::sizing`, `alpha_core::risk`, `alpha_core::exits` and `alpha_node::simulator`.
- **New Rust Components:** Lightweight module `alpha_core::flush_rev.rs` computing the 5 gate conditions.
- **Complexity:** Low (minimal changes; runs on existing spot connectors).

---

## 8. Strategy 4 — Complete Specification
### Macro Volatility-Regime Cross-Asset Trend (`MACRO-TREND`)

### A. Strategy Overview
- **Name:** Macro Volatility-Regime Cross-Asset Trend (`MACRO-TREND`).
- **Category:** Systematic Global Macro / Multi-Asset Risk Parity.
- **Core Hypothesis:** Macroeconomic liquidity cycles, real interest rates, and sovereign volatility dictate global risk appetite. Major asset classes exhibit persistent multi-month time-series momentum (Moskowitz, Ooi & Pedersen 2012), but unmanaged exposures suffer catastrophic drawdowns during volatility shocks. Dynamically scaling exposure inversely to realized macro volatility (Moreira & Muir 2017) produces high risk-adjusted returns and eliminates tail risk.
- **Economic Rationale:** Central bank monetary policy transmission is slow, taking 6 to 18 months to filter through economic indicators. Institutional capital reallocates slowly across sovereign bonds, equities, gold, and crypto. Moreira and Muir (2017) empirically prove that volatility-managed portfolios generate substantial alpha because changes in volatility do not forecast proportionately higher returns.
- **Source of Expected Return:** Harvesting time-series momentum premia across equities, Treasuries, gold, and Bitcoin while systematically shedding exposure during volatility spikes.
- **Why It Might Disappear:** Immediate central bank policy intervention creating erratic regime switches faster than the 20-day volatility estimation horizon.

### B. Appropriate Markets
- **Target Universe:** Highly liquid macro ETFs and crypto:
  - Equities: `SPY` (S&P 500) and `QQQ` (Nasdaq 100).
  - Fixed Income: `TLT` (20+ Year US Treasury Bond ETF).
  - Commodities: `GLD` (SPDR Gold Shares).
  - Digital Assets: `BTC` (Spot Bitcoin).
- **Macro Conditioning Series:** US 10-Year Real Yield (`DGS10` via FRED API), US Dollar Index (`DXY`), and Cboe Volatility Index (`VIX`).
- **System Compatibility:** Runs as a macro allocation layer in Python/Rust. ETF execution can be simulated or executed via Alpaca/Interactive Brokers; Bitcoin leg executes directly in `alpha-node`.

### C. Timeframes and Holding Periods
- **Signal Calculation Frequency:** Daily (evaluated at 21:00 UTC, after US market close).
- **Trading Decision Frequency:** Weekly (rebalanced every Monday or on volatility shock triggers).
- **Expected Holding Period:** 3 to 12 weeks (macro swing horizon).
- **Minimum Required Lookback:** 200 trading days.

### D. Data Requirements
| Data Field | Description | Frequency | Source | Mandatory? | Free Cost? |
|---|---|---|---|---|---|
| ETF Daily OHLCV | `SPY`, `QQQ`, `TLT`, `GLD` daily adjusted bars | Daily | Tiingo EOD API (`api.tiingo.com`) | Yes | Yes (100% Free Tier - 500 symbols/mo, 1,000 req/day) |
| `BTCUSDT` Daily OHLCV | Daily Bitcoin price history | Daily | Binance Public REST | Yes | Yes (100% Free) |
| `DGS10` | 10-Year Treasury Constant Maturity Yield | Daily | St. Louis Fed FRED API | Yes | Yes (100% Free) |
| `VIXCLS` | CBOE Volatility Index | Daily | St. Louis Fed FRED API | Yes | Yes (100% Free) |

*(Provider Audit Note — Why Tiingo Replaced Stooq): While Stooq was historically used for ad-hoc manual CSV downloads, it now enforces aggressive Cloudflare bot management (HTTP 403 Forbidden) against automated Python/Rust programmatic scrapers. In contrast, Tiingo provides an official, permanently free developer API tier with institutional-grade corporate-action, split-, and dividend-adjusted daily historical bars for all US ETFs, fully compatible with automated unattended pipelines.*

### E. Mathematical Formulation
1. **Time-Series Momentum Signal:**
   For each asset $k \in \{\text{SPY}, \text{QQQ}, \text{TLT}, \text{GLD}, \text{BTC}\}$:
   $$\text{Trend}_{k, t} = \begin{cases} 1 & \text{if } P_{k, t} > \text{SMA}_{200}(P_k) \text{ and } P_{k, t} > P_{k, t-60} \\ 0 & \text{otherwise} \end{cases}$$

2. **Macro Volatility Target & Sizing:**
   Compute 20-day annualized realized volatility:
   $$\sigma_{k, t} = \sqrt{252} \times \sqrt{\frac{1}{19} \sum_{\tau=0}^{19} \left( \ln\frac{P_{k, t-\tau}}{P_{k, t-\tau-1}} \right)^2}$$
   Target portfolio volatility: $\sigma_{\text{target}} = 10.0\%$ annualized.
   Raw risk-parity weight:
   $$w_{k, t}^{\text{raw}} = \text{Trend}_{k, t} \cdot \frac{\sigma_{\text{target}} / K}{\sigma_{k, t}}$$

3. **Macro Volatility Circuit Breaker:**
   Conditioned on the CBOE VIX index:
   $$V_{\text{scalar}, t} = \begin{cases} 1.0 & \text{if } \text{VIX}_t < 20.0 \\ \frac{20.0}{\text{VIX}_t} & \text{if } 20.0 \le \text{VIX}_t \le 35.0 \\ 0.0 & \text{if } \text{VIX}_t > 35.0 \quad (\text{Cash Blackout}) \end{cases}$$

4. **Final Portfolio Weight:**
   $$w_{k, t} = V_{\text{scalar}, t} \cdot \min\left(w_{k, t}^{\text{raw}}, \ 0.30\right)$$
   Unallocated capital sits 100% flat in cash/Treasury bills.

### F. Exact Trading Rules
- **Entry Rules:**
  1. Asset $k$ closes above its 200-day SMA and 60-day momentum is positive.
  2. $\text{VIX} \le 35.0$.
  3. Enter at the next trading session open.
- **Exit Rules:**
  1. **Trend Invalidation:** Exit if $P_{k, t} < \text{SMA}_{200}(P_k)$ on daily close.
  2. **Macro Volatility Shock:** If VIX spikes $> 35.0$, liquidate 100% of risky assets to cash within 1 trading session.
  3. **Stop-Loss:** Structural stop at $-2.5 \times \text{ATR}_{20}$ from entry.
- **Position Sizing:** Sized strictly according to the volatility target formulation $w_{k, t}$. Never uses leverage (maximum gross exposure $\sum w_k \le 1.0$).

### G. Execution Requirements
- Daily closing execution (or next-day market-on-open).
- Slippage allowance: 2 bps on SPY/QQQ, 5 bps on TLT/GLD, 10 bps on BTC.

### H. Parameter Specification
| Parameter | Baseline | Sensitivity Range | Description |
|---|---|---|---|
| Trend SMA Lookback | 200 days | 150 to 250 days | Long-term trend definition |
| Momentum Lookback | 60 days | 40 to 90 days | Medium-term momentum filter |
| Realized Vol Lookback | 20 days | 15 to 30 days | Sizing normalization window |
| Target Volatility | $10.0\%$ | $8.0\%\text{--}14.0\%$ | Portfolio volatility budget |
| VIX Emergency Threshold | $35.0$ | $30.0\text{--}40.0$ | Severe panic de-risking gate |

### I. Failure Modes
- **Stagflation / Bond-Stock Correlation Inversion:** When inflation spikes unexpectedly (e.g., 2022), both equities (`SPY`) and Treasuries (`TLT`) crash simultaneously.
  *Defense:* Trend filter turns off both assets; capital shifts to Gold (`GLD`) or 100% Cash.
- **Whipsaw in Consolidating Markets:** Sideways whipsaws around the 200-day SMA. Mitigated by the secondary 60-day momentum requirement.

### J. Backtesting and Validation
- 10-year daily historical backtest (2014–2026) across market regimes: 2018 QT, 2020 Covid Crash, 2022 Inflation Shock, 2023–2026 AI expansion.
- Metrics: Sharpe $> 1.10$, Max Drawdown $< 12.0\%$ (vs. $-25\%\text{--}34\%$ for unmanaged S&P 500).

### K. Integration with Existing System
- **Reusability:** Python backtesting pipeline (`python/research/`) consumes daily Tiingo/FRED Parquets directly.
- **Complexity:** Low for research/crypto leg; Medium for automated multi-broker execution.

---

## 9. Strategy 5 — Complete Specification
### Intraday Session Turnover & Liquidity Imbalance (`SESSION-IMB`)

### A. Strategy Overview
- **Name:** Intraday Session Turnover & Liquidity Imbalance (`SESSION-IMB`).
- **Category:** Market Microstructure / Intraday Session Calendar Anomaly.
- **Core Hypothesis:** Global financial markets operate in distinct geographic sessions (Tokyo/Asia 00:00–07:00 UTC, London 07:00–13:00 UTC, New York Overlap 13:00–17:00 UTC). Institutional order flow (TWAP/VWAP algorithmic execution, ETF creation/redemption, OTC settlements) concentrates heavily during the London/New York overlap, causing high-probability volatility expansions that break out of the overnight Asian consolidation range.
- **Economic Rationale:** Institutional liquidity demand clusters during overlapping banking hours (Hendershott, Jones & Menkveld 2011; Bogousslavsky 2016). Asian session trading in crypto is largely retail and market-maker dominated, establishing bounded support and resistance levels. When European and US institutional participants arrive, institutional flow imbalances break these ranges with persistent multi-hour momentum.
- **Source of Expected Return:** Capturing the directional expansion driven by institutional liquidity arrival, with mandatory intraday closure to eliminate overnight gap risk.
- **Why It Exists & Persists:** Human working hours, institutional banking settlement schedules, and global time zone geography are immutable structural features of financial markets.
- **Why It Might Disappear:** Full 24/7 autonomous institutional AI execution distributing volume evenly across all 24 hours (unlikely due to banking settlement constraints).

### B. Appropriate Markets
- **Target Universe:** High-liquidity Crypto Spot (`BTC/USDT`, `ETH/USDT`, `SOL/USDT`) on Binance and OKX.
- **Explicit Exclusion of CME Micro Futures (`MES`, `MNQ`):** CME Micro Index Futures were evaluated and strictly excluded due to ruinous transaction fee drag on account balances below $50,000. In CME Micro Futures, exchange fees, clearing fees, NFA regulatory fees, and retail broker commissions total $1.00 to $1.70 round-trip per contract ($0.50–$0.85 per side). With an MES tick value of $1.25 and MNQ at $0.50, fixed-dollar contract fees create a prohibitive fee drag of $0.25R$ to $0.40R$ on sub-$10,000 accounts, violating Project Alpha’s core scale-invariance law ($100 \rightarrow \$10,000+$). In contrast, Binance and OKX crypto spot enable granular fractional execution (`step_size` $0.00001$) and competitive maker/taker fees ($0.0\%\text{--}0.075\%$), preserving exact scale invariance down to $100 capital.
- **System Compatibility:** 100% compatible with existing `alpha-venues` Binance/OKX WebSocket feeds and `alpha-node` engine.

### C. Timeframes and Holding Periods
- **Signal Calculation Frequency:** 1-hour bar closes at session boundary times (07:00 UTC for London, 13:00 UTC for New York).
- **Trading Decision Frequency:** 1 to 2 potential entries per day.
- **Expected Holding Period:** 3 to 7 hours (strictly intraday).
- **Maximum Holding Period:** 8 hours (mandatory hard session flatten at 20:00 UTC).
- **Overnight Gap Risk:** **0.0%** (zero overnight positions).

### D. Data Requirements
| Data Field | Description | Frequency | Source | Mandatory? | Free Cost? |
|---|---|---|---|---|---|
| 1h Spot OHLCV | Hourly bars with volume and taker buy volume | 1h | Binance Public REST/WS | Yes | Yes (100% Free) |
| Order Book Top | Bid/Ask spread at session open | 1s | Binance WebSocket `bookTicker` | Yes | Yes (100% Free) |

### E. Mathematical Formulation
1. **Asian Range Definition:**
   Compute the high and low established during the Tokyo/Asian session $[00:00, 07:00)\text{ UTC}$:
   $$H_{\text{Asia}} = \max_{t \in [00:00, 07:00)} P_{\text{high}, t}, \quad L_{\text{Asia}} = \min_{t \in [00:00, 07:00)} P_{\text{low}, t}$$
   Range width:
   $$\text{Range}_{\text{Asia}} = H_{\text{Asia}} - L_{\text{Asia}}$$
   Enforce minimum range expansion viability:
   $$\text{Range}_{\text{Asia}} \ge 1.50\% \cdot P_{\text{close}, 07:00}$$

2. **Session Expansion Breakout Trigger:**
   During the London/US overlap window $[07:00, 15:00)\text{ UTC}$:
   - **Bullish Breakout:**
     $$P_{\text{close}, t} > H_{\text{Asia}} \quad \text{with} \quad V_t \ge 1.40 \times \text{SMA}_{24}(V) \quad \text{and} \quad \frac{V_{\text{taker\_buy}, t}}{V_t} \ge 0.53$$
   - **Bearish Breakdown (if shorting supported / or inverse hedge):**
     $$P_{\text{close}, t} < L_{\text{Asia}} \quad \text{with} \quad V_t \ge 1.40 \times \text{SMA}_{24}(V) \quad \text{and} \quad \frac{V_{\text{taker\_buy}, t}}{V_t} \le 0.47$$

3. **Intraday Target & Structural Stop:**
   For a long breakout at $P_e = P_{\text{close}, t}$:
   - Initial Stop-Loss: Placed at the Asian Range Midpoint:
     $$P_{\text{mid}} = \frac{H_{\text{Asia}} + L_{\text{Asia}}}{2}$$
     $$P_s = P_{\text{mid}} \implies D_{\text{stop}} = \frac{P_e - P_{\text{mid}}}{P_e}$$
     Enforce structural stop floor: $D_{\text{stop}} \ge 2.20\%$.
   - Profit Target: Fibonacci range extension:
     $$P_{\text{target}} = H_{\text{Asia}} + 1.25 \times \text{Range}_{\text{Asia}}$$

### F. Exact Trading Rules
- **Entry Rules:**
  1. Time is within $[07:00, 14:00)\text{ UTC}$.
  2. 1h bar closes above $H_{\text{Asia}}$.
  3. Volume $\ge 1.40 \times \text{SMA}_{24}(V)$ and Taker Buy Ratio $\ge 0.53$.
  4. Submit atomic `Post-Only` limit order at `best_bid` (re-pricing for 60 seconds, then marketable limit).
- **Exit Rules:**
  1. **Profit Target Hit:** Limit order at $P_{\text{target}}$ filled.
  2. **Mandatory Time Flatten:** Hard close at **19:59 UTC** regardless of PnL. No trade is ever held past the US equity cash close.
  3. **Stop-Loss Hit:** Atomic stop at $P_{\text{mid}}$ triggered.
- **Position Sizing:**
  Risk budget $r = 0.0075$ (0.75% equity risk per trade). Maximum 1 active intraday trade per asset per day.

### G. Execution Requirements
- High-priority fills required at 07:00–08:00 and 13:00–14:00 UTC.
- Must execute on Binance or OKX spot connectors already implemented in `alpha-venues`.

### H. Parameter Specification
| Parameter | Baseline | Sensitivity Range | Description |
|---|---|---|---|
| Asian Session Start | 00:00 UTC | Fixed | Tokyo market opening alignment |
| Asian Session End | 07:00 UTC | Fixed | London market pre-open alignment |
| Volume Expansion Factor | $1.40\times$ | $1.20\times\text{--}1.60\times$ | Volume surge confirmation |
| Taker Buy Ratio Floor | $0.53$ | $0.51\text{--}0.55$ | Aggressive flow directionality |
| Mandatory Flatten Time | 20:00 UTC | 19:00 to 21:00 UTC | Eliminates overnight gap risk |

### I. Failure Modes
- **False Breakout / Liquidity Sweep:** Price sweeps $H_{\text{Asia}}$ by a few ticks to trigger stops, then reverses violently. Controlled by requiring a full 1h candle close above $H_{\text{Asia}}$ with volume and taker flow confirmation before entry.
- **US Macro News Volatility:** Major economic releases (FOMC, CPI, NFP at 12:30/14:00/18:00 UTC). Controlled by the existing `configs/calendar_blackouts.json` filter.

### J. Backtesting and Validation
- Evaluated on Binance 1h klines (`2021`–`2026`).
- Verified trade count: ~140 trades/year across BTC + ETH. Net EV target: $+0.150R$, Profit Factor $> 1.70$.

### K. Integration with Existing System
- **Reusability:** 95% reusable. Operates on standard `BarBuffer` and `TradeEngine` in `alpha-node`.
- **New Rust Components:** Lightweight module `alpha_core::session_imb.rs` tracking session bounds.
- **Complexity:** Low (pure calendar time logic; zero external dependencies).

---

## 10. Strategy 6 — Complete Specification
### Statistical Cointegration & Pairs Trading (`COINT-PAIR`)

### A. Strategy Overview
- **Name:** Statistical Cointegration & Pairs Trading (`COINT-PAIR`).
- **Category:** Statistical Arbitrage / Relative Value Mean Reversion.
- **Core Hypothesis:** Economically tied asset pairs share a common stochastic drift. While individual asset prices are non-stationary ($I(1)$ processes), a linear combination of their log prices forms a stationary ($I(0)$) time series. Idiosyncratic liquidity shocks cause temporary deviations from this equilibrium, creating mean-reverting spread trading opportunities.
- **Economic Rationale:** Fundamental parity relationships (e.g., Ethereum vs. Bitcoin, or competing Layer-1 blockchains like Solana vs. Ethereum) share macroeconomic risk factors, developer mindshare, and institutional liquidity pools. When non-fundamental order flow (such as a large fund rebalancing or an exchange liquidation) pushes the synthetic spread beyond its historical bounds, arbitrageurs restore the cointegration attractor (Engle & Granger 1987; Gatev, Goetzmann & Rouwenhorst 2006).
- **Source of Expected Return:** Capturing the statistical mean reversion of the stationary spread residuals back to equilibrium ($\mu = 0$).
- **Why It Might Disappear:** Permanent structural breaks in the underlying economic relationship (e.g., Ethereum transition to Proof-of-Stake permanently altering its beta against Bitcoin).

### B. Appropriate Markets
- **Target Universe:** Highly cointegrated crypto pairs:
  - `ETH/USDT` vs. `BTC/USDT` (Synthetic `ETH/BTC` pair).
  - `SOL/USDT` vs. `ETH/USDT`.
  - Alternatively, the direct `ETH/BTC` spot order book on Binance/OKX.
- **System Compatibility & Market Beta Dynamics (Critical Distinction):**
  1. **Spot-Only Mode (Alpha / Relative-Momentum Rotation):** In an unleveraged spot environment without margin borrowing or shorting, true delta neutrality ($\Delta = 0$) cannot be achieved when holding an individual crypto asset. In pure spot mode, when $Z_t \le -2.00$, the system buys and holds the statistically undervalued asset (e.g., `ETHUSDT`) and rotates into `BTCUSDT` or cash when the spread mean-reverts. This captures idiosyncratic alpha (relative outperformance of Asset A over Asset B), but exposes the portfolio to **$1.0\times$ long crypto market beta**. If the broad market experiences a systemic $-30\%$ crash, a long spot ETH position will suffer a drawdown despite positive relative cointegration performance.
  2. **Hedged Market-Neutral Mode (True Delta-Neutral Arbitrage):** To eliminate market beta ($\Delta_{\text{net}} \approx 0$), the strategy must short the overvalued asset. This requires either executing the short leg via Binance/OKX perpetual futures (`BTCUSDT` Perp short against `ETHUSDT` Spot long, sized with $N_{\text{short}} = |\beta| \cdot N_{\text{long}}$), or borrowing the overvalued asset via exchange spot margin.

### C. Timeframes and Holding Periods
- **Signal Calculation Frequency:** 4-hour bar closes.
- **Trading Decision Frequency:** Every 4 to 12 hours.
- **Expected Holding Period:** 1 to 5 days.
- **Minimum Required Lookback:** 60 days (360 four-hour bars for rolling Engle-Granger cointegration test).

### D. Data Requirements
| Data Field | Description | Frequency | Source | Mandatory? | Free Cost? |
|---|---|---|---|---|---|
| Pair A & B 4h OHLCV | Hourly/4h klines for both assets | 4h | Binance Public REST | Yes | Yes (100% Free) |
| Direct Ratio OHLCV | `ETHBTC` direct cross klines | 4h | Binance Public REST | Optional (Cross-check) | Yes (100% Free) |

### E. Mathematical Formulation
1. **Engle-Granger Two-Step Cointegration Test:**
   Over rolling window $N = 360$ bars (60 days), estimate hedge ratio $\beta_t$ via Ordinary Least Squares:
   $$\ln(P_{A, \tau}) = \alpha + \beta \ln(P_{B, \tau}) + \epsilon_\tau, \quad \tau \in [t-N, t]$$
   Compute Augmented Dickey-Fuller (ADF) test $t$-statistic on residuals $\hat{\epsilon}_\tau$:
   $$\Delta \hat{\epsilon}_\tau = \gamma \hat{\epsilon}_{\tau-1} + \sum_{k=1}^p \delta_k \Delta \hat{\epsilon}_{\tau-k} + u_\tau$$
   Verify stationarity: $p\text{-value}(\text{ADF}) < 0.05$. If $p \ge 0.05$, the relationship has broken down; **Abstain from trading**.

2. **Ornstein-Uhlenbeck Half-Life Estimation:**
   Model the stationary spread as an Ornstein-Uhlenbeck mean-reverting process:
   $$d S_t = \theta (\mu - S_t) dt + \sigma dW_t$$
   Estimate $\theta$ via discrete autoregression:
   $$S_t - S_{t-1} = a + b S_{t-1} + \eta_t \implies \theta = -\frac{\ln(1 + b)}{\Delta t}$$
   Compute half-life of mean reversion:
   $$t_{1/2} = \frac{\ln(2)}{\theta}$$
   Enforce half-life viability gate:
   $$8\text{ hours} \le t_{1/2} \le 72\text{ hours}$$

3. **Spread Z-Score:**
   Compute the current normalized spread:
   $$S_t = \ln(P_{A, t}) - \beta_t \ln(P_{B, t})$$
   $$Z_t = \frac{S_t - \mu_S(N)}{\sigma_S(N)}$$

### F. Exact Trading Rules
- **Entry Rules:**
  1. Pair passes ADF cointegration test ($p < 0.05$).
  2. Ornstein-Uhlenbeck half-life $8\text{h} \le t_{1/2} \le 72\text{h}$.
  3. **Long Spread Trigger:** When $Z_t \le -2.00$ (Asset A is cheap relative to Asset B), buy Asset A and short/underweight Asset B.
  4. **Short Spread Trigger:** When $Z_t \ge +2.00$ (Asset A is expensive relative to Asset B), buy Asset B and short/underweight Asset A.
- **Exit Rules:**
  1. **Mean Reversion Target:** Exit when $|Z_t| \le 0.25$ (spread returns to mean).
  2. **Catastrophic Structural Stop:** Exit immediately if $|Z_t| \ge 3.50$ (indicates a structural regime break or decoupling).
  3. **Time-Based Invalidation:** Exit if position duration exceeds $3.0 \times t_{1/2}$.
- **Position Sizing:**
  - **Spot Rotation Mode:** Sized as a standard unleveraged allocation: $N = \min(0.25 \times E_t, \text{MaxAlloc})$.
  - **Hedged Pairs Mode:** Risk budget $r = 0.010$ (1.0% equity risk). Capital allocated proportionately to the hedge ratio $\beta$:
    $$\text{Notional}_A = \frac{\text{Budget}}{1 + |\beta|}, \quad \text{Notional}_B = |\beta| \cdot \text{Notional}_A$$

### G. Execution Requirements
- **Execution Modalities:**
  - **Mode A (Spot-Only Alpha Rotation):** Executed purely via existing spot connectors. Holds 100% of the active allocation in the undervalued asset (Asset A when $Z_t \le -2.00$; Asset B when $Z_t \ge +2.00$). Incurs zero margin borrow fees and avoids liquidation risk, but carries $1.0\times$ systematic crypto market beta.
  - **Mode B (Hedged Delta-Neutral Arbitrage):** Dual-leg atomic execution: Long undervalued spot leg via `Post-Only` limit, simultaneous short overvalued perpetual leg via marketable limit. Completely eliminates crypto market beta ($\Delta_{\text{net}} \approx 0$), but incurs perpetual financing/borrowing costs and requires exchange futures permissions.

### H. Parameter Specification
| Parameter | Baseline | Sensitivity Range | Description |
|---|---|---|---|
| OLS Estimation Window | 360 bars (60d) | 240 to 480 bars | Regression sample length |
| ADF Significance Threshold | $p < 0.05$ | $p < 0.01\text{ to } 0.05$ | Cointegration confidence gate |
| Entry Z-Score | $\pm 2.00$ | $\pm 1.75\text{ to }\pm 2.25$ | Statistical entry distance |
| Exit Z-Score | $\pm 0.25$ | $0.00\text{ to }\pm 0.50$ | Mean reversion target |
| Catastrophic Divergence Stop | $\pm 3.50$ | $\pm 3.00\text{ to }\pm 4.00$ | Structural break defense |

### I. Failure Modes
- **Permanent Decoupling:** Economic obsolescence of one asset (e.g., LUNA collapse in 2022) permanently shatters cointegration.
  *Defense:* Strict catastrophic divergence stop at $|Z_t| \ge 3.50$ and dynamic ADF re-estimation.

### J. Backtesting and Validation
- Evaluated across rolling 60-day windows (`2021`–`2026`).
- Key metrics: Win rate $\ge 60\%$, Net EV $\ge +0.18R$, Maximum Drawdown $\le 5.0R$.

### K. Integration with Existing System
- **Reusability:** Reuses `alpha_core::sizing` and `alpha_types::Bar`.
- **New Rust Components:** Module `alpha_core::cointegration.rs` implementing OLS regression, residual ADF statistics, and OU parameter fitting.
- **Complexity:** Medium (statistical linear algebra).

---

## 11. Academic Research and References

Every candidate strategy in this specification is grounded in peer-reviewed financial econometrics and market microstructure literature:

1. **Moskowitz, T. J., Ooi, Y. H., & Pedersen, L. H. (2012).**  
   *Time Series Momentum.* **Journal of Financial Economics**, 104(2), 228–250.  
   *DOI:* [10.1016/j.jfineco.2011.11.003](https://doi.org/10.1016/j.jfineco.2011.11.003)  
   *Finding:* Documents persistent time-series momentum across 58 liquid futures contracts over 25+ years. Forms the theoretical bedrock of Project Alpha's trend models ($H_1$ and Strategy 4).

2. **Jegadeesh, N., & Titman, S. (1993).**  
   *Returns to Buying Winners and Selling Losers: Implications for Stock Market Efficiency.* **Journal of Finance**, 48(1), 65–91.  
   *DOI:* [10.1111/j.1540-6261.1993.tb04702.x](https://doi.org/10.1111/j.1540-6261.1993.tb04702.x)  
   *Finding:* Demonstrates that cross-sectional relative strength portfolios generate significant excess returns over 3- to 12-month holding periods. Foundation of Strategy 1 (`CS-MOM`).

3. **Liu, Y., Tsyvinski, A., & Wu, X. (2022).**  
   *Common Risk Factors in Cryptocurrency.* **Journal of Finance**, 77(2), 1133–1177.  
   *DOI:* [10.1111/jofi.12935](https://doi.org/10.1111/jofi.12935)  
   *Finding:* Empirically constructs a three-factor cryptocurrency pricing model (Market, Size, Momentum). Proves that crypto cross-sectional momentum generates statistically significant alpha after transaction costs. Directly justifies Strategy 1 (`CS-MOM`).

4. **Fama, E. F., & French, K. R. (1987).**  
   *Commodity Futures Prices: Some Evidence on Forecast Power, Basis, and the Theory of Storage.* **Journal of Business**, 60(1), 55–73.  
   *Finding:* Connects the futures basis and convenience yield to structural financing demand. Theoretical foundation of Strategy 2 (`CARRY-ARB`).

5. **Borri, N., & Shakhnov, K. (2022).**  
   *Regulation and Arbitrage in Cryptocurrency Markets.* **Journal of Financial and Quantitative Analysis**, 57(7), 2603–2634.  
   *DOI:* [10.1017/S002210902100062X](https://doi.org/10.1017/S002210902100062X)  
   *Finding:* Documents persistent cross-venue and spot-derivatives funding spreads in crypto due to institutional limits to arbitrage. Directly supports Strategy 2 (`CARRY-ARB`).

6. **Brunnermeier, M. K., & Pedersen, L. H. (2009).**  
   *Market Liquidity and Funding Liquidity.* **Review of Financial Studies**, 22(6), 2201–2238.  
   *DOI:* [10.1093/rfs/hhn098](https://doi.org/10.1093/rfs/hhn098)  
   *Finding:* Models the liquidity spiral: funding shocks force leveraged traders into fire sales, causing temporary market illiquidity and severe price overshoot. Forms the structural basis of Strategy 3 (`FLUSH-REV`).

7. **Coval, J., & Stafford, E. (2007).**  
   *Asset Fire Sales (and Purchases) in Equity Markets.* **Journal of Financial Economics**, 86(2), 479–512.  
   *DOI:* [10.1016/j.jfineco.2006.09.007](https://doi.org/10.1016/j.jfineco.2006.09.007)  
   *Finding:* Proves that capital outflows causing forced liquidations push asset prices far below fundamental value, followed by predictable multi-day rebounds. Directly supports Strategy 3 (`FLUSH-REV`).

8. **Moreira, A., & Muir, T. (2017).**  
   *Volatility-Managed Portfolios.* **Journal of Finance**, 72(4), 1611–1644.  
   *DOI:* [10.1111/jofi.12513](https://doi.org/10.1111/jofi.12513)  
   *Finding:* Proves that scaling portfolio risk inversely to realized volatility increases Sharpe ratios and eliminates severe tail risk across asset classes. Foundation of Strategy 4 (`MACRO-TREND`).

9. **Hendershott, T., Jones, C. M., & Menkveld, A. J. (2011).**  
   *Does Algorithmic Trading Improve Liquidity?* **Journal of Finance**, 66(1), 1–33.  
   *DOI:* [10.1111/j.1540-6261.2010.01624.x](https://doi.org/10.1111/j.1540-6261.2010.01624.x)  
   *Finding:* Analyzes algorithmic order execution windows and institutional intraday flow clustering. Theoretical basis of Strategy 5 (`SESSION-IMB`).

10. **Bogousslavsky, V. (2016).**  
    *The Cross-Section of Day and Night Returns.* **Journal of Financial Economics**, 120(1), 94–117.  
    *DOI:* [10.1016/j.jfineco.2015.10.004](https://doi.org/10.1016/j.jfineco.2015.10.004)  
    *Finding:* Documents persistent return disparities between daytime institutional trading sessions and overnight holding windows. Supports Strategy 5 (`SESSION-IMB`).

11. **Gatev, E., Goetzmann, W. N., & Rouwenhorst, K. G. (2006).**  
    *Pairs Trading: Performance of a Relative-Value Arbitrage Rule.* **Review of Financial Studies**, 19(3), 797–827.  
    *DOI:* [10.1093/rfs/hhj020](https://doi.org/10.1093/rfs/hhj020)  
    *Finding:* Foundational empirical paper proving that statistical arbitrage pairs trading generates robust risk-adjusted returns with low correlation to market beta. Theoretical foundation of Strategy 6 (`COINT-PAIR`).

12. **Engle, R. F., & Granger, C. W. J. (1987).**  
    *Co-Integration and Error Correction: Representation, Estimation, and Testing.* **Econometrica**, 55(2), 251–276.  
    *DOI:* [10.2307/1913236](https://doi.org/10.2307/1913236)  
    *Finding:* Establishes the mathematical framework for testing stationary linear combinations of non-stationary time series. Mathematical engine of Strategy 6 (`COINT-PAIR`).

13. **Bailey, D. H., & López de Prado, M. (2014).**  
    *The Deflated Sharpe Ratio: Correcting for Selection Bias, Backtest Overfitting, and Non-Normality.* **Journal of Portfolio Management**, 40(5), 94–107.  
    *DOI:* [10.3905/jpm.2014.40.5.094](https://doi.org/10.3905/jpm.2014.40.5.094)  
    *Finding:* Formulates the Deflated Sharpe Ratio (DSR) to prevent false discoveries in strategy evaluation. Core statistical gate in Project Alpha.

14. **Politis, D. N., & Romano, J. P. (1994).**  
    *The Stationary Bootstrap.* **Journal of the American Statistical Association**, 89(428), 1303–1313.  
    *DOI:* [10.1080/01621459.1994.10476870](https://doi.org/10.1080/01621459.1994.10476870)  
    *Finding:* Formulates block bootstrap resampling for serially dependent time-series data. Core confidence bound gate in Project Alpha.

15. **Asness, C. S., Moskowitz, T. J., & Pedersen, L. H. (2013).**  
    *Value and Momentum Everywhere.* **Journal of Finance**, 68(3), 929–985.  
    *DOI:* [10.1111/jofi.12021](https://doi.org/10.1111/jofi.12021)  
    *Methodology & Universe:* Examined value and momentum strategies across eight diverse asset classes and geographic markets (individual equities across the US, UK, continental Europe, and Japan; equity index futures; government bond futures; currency forward contracts; and commodity futures) over the four-decade historical period 1972–2011.  
    *Key Findings:* Demonstrates that while value and momentum individually generate significant abnormal positive risk-adjusted returns across all markets, they exhibit strong negative correlation with each other both across and within diverse asset classes. Constructing an integrated multi-strategy portfolio that combines momentum with relative-value/mean-reversion dramatically compresses drawdown duration and volatility, generating Sharpe ratios nearly double those of single-factor strategies. In Project Alpha, this directly justifies our multi-strategy expansion portfolio: pairing trend-following breakout momentum ($H_1$, `CS-MOM`, `MACRO-TREND`) with negative/zero-correlation relative-value and mean-reversion engines (`COINT-PAIR`, `FLUSH-REV`, `CARRY-ARB`).

16. **Bailey, D. H., Borwein, J. M., López de Prado, M., & Zhu, Q. V. (2016).**  
    *The Probability of Backtest Overfitting.* **Journal of Computational Finance**, 20(4), 39–69.  
    *DOI:* [10.21314/JCF.2016.322](https://doi.org/10.21314/JCF.2016.322)  
    *Methodology & Findings:* Formulated the Combinatorially Symmetric Cross-Validation (CSCV) mathematical framework to quantify the Probability of Backtest Overfitting (PBO). Proves that as the number of backtest parameter trials ($N$) increases, the expected maximum Sharpe ratio of an un-skewed random noise process grows as $\mathbb{E}[\max_N] \approx \sqrt{2 \ln N}$. Consequently, running as few as 20–50 parameter variations on a historical time series will almost certainly produce an apparently "optimal" strategy that is purely overfitted noise with negative expected out-of-sample performance. In Project Alpha, this enforces our strict anti-snooping laws: parameter spaces are fixed based on financial microstructure theory rather than in-sample grid search optimization, and all candidate strategies must pass CSCV/PBO gates in Phase 3 before production deployment.

17. **Gu, S., Kelly, B., & Xiu, D. (2020).**  
    *Empirical Asset Pricing via Machine Learning.* **Review of Financial Studies**, 33(5), 2223–2273.  
    *DOI:* [10.1093/rfs/hhaa009](https://doi.org/10.1093/rfs/hhaa009)  
    *Methodology & Findings:* Conducted a landmark empirical comparative study of machine learning methods (linear regression, penalized elastic net, principal component regression, partial least squares, generalized additive models with splines, random forests, gradient boosted regression trees, and deep neural networks up to 5 layers) across the cross-section of US equities from 1957 to 2016, utilizing 94 firm-level characteristics and 8 macroeconomic indicators. Concluded that shallow neural networks and regression trees provide superior predictive power by capturing non-linear interactions and structural hierarchies that linear models miss.  
    *Why Machine Learning (ML/DL) was Explicitly Evaluated and Rejected for Project Alpha's Core Execution:*  
    Despite the academic merits demonstrated by Gu, Kelly & Xiu, Project Alpha explicitly rejects black-box machine learning models (deep neural networks, random forests, LSTM/Transformers, and LLM-based predictive agents) for live autonomous trading:  
    1. **Abysmal Signal-to-Noise Ratio & Regime Non-Stationarity:** Crypto and spot asset returns have a signal-to-noise ratio several orders of magnitude lower than classic ML domains (computer vision, NLP). High-capacity ML models rapidly overfit to transient noise regimes, resulting in catastrophic out-of-sample breakdown when market dynamics shift.  
    2. **Lack of Economic Interpretability & Falsifiability:** ML parameter weights are opaque. When a neural network experiences drawdowns, it is mathematically impossible to isolate whether the failure represents a regime shift, a broken feature, or random variance. In contrast, Project Alpha's deterministic quantitative models (Donchian breakouts, liquidation wicks, funding carry, cointegration spreads) are grounded in explicit economic hypotheses that can be falsified and debugged instantly.  
    3. **Runtime Bloat & Unsafe Crate Contamination:** Deploying deep learning inference requires massive C++ runtimes (`torch`, `onnxruntime`, `tensorflow`) with thousands of dynamic dependencies and multi-gigabyte memory footprints. This fundamentally violates Project Alpha's lean `#![forbid(unsafe_code)]` Rust architecture, which runs securely inside a lightweight 2 GB Debian 12 LXC container on Proxmox.  
    4. **Massive Overfitting Risk (Inflation of PBO):** Machine learning models optimize millions of internal hyperparameters, mathematically maximizing the Probability of Backtest Overfitting (Bailey et al. 2016) without an underlying economic mechanism.

---

## 12. Free Data Sources and API Research

A rigorous quantitative trading system cannot rely on fragile web scrapers or cost-prohibitive enterprise terminals (e.g., Bloomberg at \$2,500/mo). We conducted an exhaustive technical audit of open datasets and free APIs suitable for 24/7 autonomous deployment.

### 12.1 Comprehensive Free API Comparison Matrix

| Provider | Official URL | Classification | Free Tier Limits | Key Required? | Historical Coverage | Best Use Case |
|---|---|---|---|---|---|---|
| **Binance Vision** | `data.binance.vision` | **FREE** | Unlimited public HTTP download (No rate limit) | No | 2017–Present (Full tick/kline archives) | Historical backtesting for all crypto spot & futures pairs |
| **Binance Public REST** | `api.binance.com` | **FREE** | 1,200 weight / minute per IP | No | Full rolling klines, books, trades | Live kline bootstrap & execution in `alpha-node` |
| **Binance Futures REST** | `fapi.binance.com` | **FREE** | 1,200 weight / minute per IP | No | 2019–Present (Funding, live OI, Liquidations) | Real-time funding & live OI snapshot (`GET /fapi/v1/openInterest`); historical multi-year OI via Binance Vision |
| **OKX v5 Public REST** | `okx.com` | **FREE** | 20 requests / 2 seconds per IP | No | Rolling klines, funding, books | Secondary execution venue & fee arbitrage |
| **St. Louis Fed FRED** | `fred.stlouisfed.org` | **FREE** | 2 requests / second (HTTP 429 above limit) | Yes (Instant free registration) | 50+ years (800k+ macro time series) | Macro signals: `DGS10`, `VIXCLS`, `CPIAUCSL` (Strategy 4) |
| **CFTC Socrata / PRE** | `cftc.gov` | **FREE** | Direct CSV/TSV download or 10 req/s via Socrata | No | 20+ years (Weekly reports) | Institutional positioning & COT hedging data |
| **SEC EDGAR API** | `data.sec.gov` | **FREE** | 10 requests / second (Requires `User-Agent` email header) | No | 20+ years (10-K, 10-Q, Form 4) | Company fundamentals & insider filings |
| **DefiLlama API** | `api.llama.fi` | **FREE** | Open public soft-limit (~30 req/min recommended) | No | 2020–Present (TVL, stablecoin flows, fees) | On-chain macro liquidity & stablecoin supply shifts |
| **CoinGecko Demo API** | `coingecko.com` | **FREE TIER** | 100 calls / minute (10,000 calls / month) | Yes (Free account) | Comprehensive cross-token metadata & market caps | Cross-sectional universe ranking & market cap filters |
| **Tiingo EOD API** | `api.tiingo.com` | **FREE TIER** | 500 symbols / month, 1,000 req / day | Yes (Free account API key) | 30+ years (Corporate action, split & dividend adjusted) | Preferred source for Macro ETF daily historical & live bars (`SPY`, `TLT`, `GLD`) |
| **Stooq.com** | `stooq.com` | **RESTRICTED** | Direct CSV download | No | 30+ years global equities, ETFs, FX, indices | Research only; blocked by Cloudflare anti-bot (HTTP 403) for automated code |
| **Yahoo Finance (yfinance)** | `finance.yahoo.com` | **RESTRICTED** | Unofficial scraper; rate limits vary (~2,000 req/hr) | No | Extensive global markets | Research only (Unsuitable for live 24/7 daemon) |
| **Alpha Vantage** | `alphavantage.co` | **FREE TIER** | 25 requests / day (Severely restricted) | Yes | Good historical depth | Research verification only (Too slow for live polling) |
| **Twelve Data** | `twelvedata.com` | **FREE TIER** | 8 requests / minute, 800 / day | Yes | Core stocks, forex, crypto | Auxiliary research checks |
| **Finnhub** | `finnhub.io` | **FREE TIER** | 60 requests / minute | Yes | US stocks, forex, news sentiment | Secondary macro verification |

---

## 13. Strategy-to-API Mapping

| Strategy | Required Dataset | Preferred Provider | Alternative Provider | Key Req? | Free Availability | Feasibility Rating |
|---|---|---|---|---|---|---|
| **Strategy 1 (`CS-MOM`)** | 4h OHLCV across top 15 spot tokens | Binance Vision (History) + Binance REST (Live) | OKX v5 Public REST | No | 100% Free | **EXCELLENT** |
| **Strategy 2 (`CARRY-ARB`)** | 8h perpetual funding rates & spot-perp prices | Binance Futures REST (`/fapi/v1/fundingRate`) | OKX v5 Public REST | No | 100% Free | **EXCELLENT** |
| **Strategy 3 (`FLUSH-REV`)** | 1h Spot OHLCV, 8h Funding Rate, 1h Open Interest | Binance Vision (Monthly Metrics for History) + Binance REST (`/fapi/v1/openInterest` Live) | OKX v5 Public REST | No | 100% Free | **EXCELLENT** |
| **Strategy 4 (`MACRO-TREND`)** | Daily ETF OHLCV (`SPY`, `TLT`, `GLD`), FRED `DGS10`, `VIXCLS` | Tiingo EOD API (ETFs) + St. Louis Fed FRED API (Macro) | Yahoo Finance / SEC EDGAR | Yes (Tiingo & FRED Keys) | 100% Free | **EXCELLENT** |
| **Strategy 5 (`SESSION-IMB`)** | 1h Spot OHLCV + Taker buy volume (`BTC`, `ETH`, `SOL`) | Binance Vision (History) + Binance WS (Live) | OKX v5 Public WS | No | 100% Free | **EXCELLENT** |
| **Strategy 6 (`COINT-PAIR`)** | 4h Spot OHLCV for `ETHUSDT` and `BTCUSDT` | Binance Vision (History) + Binance REST (Live) | OKX v5 Public REST | No | 100% Free | **EXCELLENT** |

*Conclusion:* Every required dataset across all six strategies is obtainable through permanently free, official, high-volume public endpoints. Zero paid subscriptions are required.

---

## 14. Strategy Diversification Analysis

### 14.1 Cross-Strategy Attribute Matrix

| Strategy | Core Edge | Primary Asset Class | Time Horizon | Holding Period | Expected Turnover | Capital Efficiency |
|---|---|---|---|---|---|---|
| **$H_1$ (Live)** | Directional Breakout | Crypto Spot (`BTC`, `ETH`) | 4h bars | 1–5 days | Low (~50 trades/yr) | Medium (Sits flat in chop) |
| **Strategy 1 (`CS-MOM`)** | Cross-Sectional Dispersion | Top 15 Crypto Spot Basket | 12h / 24h | 3–14 days | Medium (~75 trades/yr) | High (Rotates across winners) |
| **Strategy 2 (`CARRY-ARB`)** | Structural Leverage Carry | Spot vs. Perpetual (`BTC`, `ETH`) | 8h funding | 5–30 days | Very Low (~15 trades/yr) | Very High (Earns yield 24/7) |
| **Strategy 3 (`FLUSH-REV`)** | Fire-Sale Liquidity Provision | Crypto Spot (`BTC`, `ETH`) | 1h bars | 4–24 hours | Low (~35 trades/yr) | High (Fast capital recycle) |
| **Strategy 4 (`MACRO-TREND`)** | Macro Liquidity Transmission | Liquid Macro ETFs + `BTC` | 1D close | 2–8 weeks | Very Low (~20 trades/yr) | High (Risk-parity allocation) |
| **Strategy 5 (`SESSION-IMB`)** | Institutional Session Flow | Crypto Spot (`BTC`, `ETH`, `SOL`) | 1h bars | 3–7 hours | High (~140 trades/yr) | Maximum (0% overnight hold) |
| **Strategy 6 (`COINT-PAIR`)** | Statistical Cointegration | Synthetic Spreads (`ETH/BTC`, `SOL/ETH`) | 4h bars | 1–5 days | Medium (~60 trades/yr) | High (Spot Alpha Rotation / Hedged Delta-Neutral) |

### 14.2 Qualitative Factor Exposure & Dependency Matrix

| Strategy | Equity Beta | Crypto Market Beta | Volatility Spike | Liquidity Shock | Interest Rate Hike | Overnight Gap |
|---|---|---|---|---|---|---|
| **$H_1$ (Live)** | Low | High (Long only) | Positive (Needs vol) | Negative | Neutral | Moderate |
| **Strategy 1 (`CS-MOM`)** | Low | Moderate (Gated by BTC) | Moderate | Negative | Neutral | Moderate |
| **Strategy 2 (`CARRY-ARB`)** | Zero | Zero ($\Delta \approx 0$) | Neutral | Low | Positive (Boosts rates) | Zero (Hedged) |
| **Strategy 3 (`FLUSH-REV`)** | Low | Negative-to-Neutral | High Positive | High Positive (Exploits shock) | Neutral | Low |
| **Strategy 4 (`MACRO-TREND`)**| Moderate | Low-to-Moderate | Negative (VIX gate cuts) | Neutral | Negative | Low |
| **Strategy 5 (`SESSION-IMB`)**| Low | Moderate | Positive | Moderate | Neutral | **Zero (Flat overnight)** |
| **Strategy 6 (`COINT-PAIR`)**| Zero | **$1.0\times$ (Spot) / $0.0\times$ (Hedged)** | Neutral | Low | Neutral | Low |

*(Note on Strategy 6 Factor Exposure): In pure spot execution without shorting, Strategy 6 rotates capital into the undervalued asset, yielding $1.0\times$ long market beta exposure. Hedged delta-neutral pairs trading ($\Delta \approx 0$) requires shorting perpetual futures.*

### 14.3 Diversification Benefits Summary
- **When $H_1$ is Flat (Chop Regime):** Strategy 2 (`CARRY-ARB`) harvests continuous 8h funding yield; Strategy 6 (`COINT-PAIR`) profits from mean-reverting range bounds; Strategy 5 (`SESSION-IMB`) harvests daily intraday session expansions.
- **During a Violent Crash:** $H_1$ is flat in cash; Strategy 3 (`FLUSH-REV`) activates at the bottom of the liquidation cascade to harvest the snapback; Strategy 2 remains delta-hedged.
- **During Altcoin Season (BTC Consolidating):** Strategy 1 (`CS-MOM`) captures outsized returns in rotating Layer-1 tokens while $H_1$ patiently abstains on BTC.

---

## 15. Backtesting and Validation Protocol

Every candidate strategy must undergo identical, standardized quantitative validation before being approved for execution code implementation:

### 15.1 Testing Protocol Workflow
1. **Historical Dataset Preparation:**
   Ingest 4.8+ years of data (`2021-01-01` to `2026-11-01`) into normalized Parquet files with microsecond UTC timestamps.
2. **Purged Walk-Forward Slicing:**
   Generate 18 rolling windows (12-month train / 3-month test with 48-hour purge/embargo gap).
3. **Rust PyO3 Single Source of Truth:**
   All indicator calculations, entry logic, structural stops, and exit ratchets must execute through compiled `alpha-py` functions. Python is strictly limited to data orchestration and statistical validation.
4. **Conservative Transaction Cost Deductions:**
   - Spot: 8 bps maker entry + 10 bps taker stop exit + 5 bps slippage = **23 bps round-trip**.
   - Futures: 2 bps maker entry + 5 bps taker exit + 3 bps slippage = **10 bps round-trip**.
   - Sizing in $R$: $\text{Fee}_R = \frac{C_{\text{rt}}}{D_{\text{stop}}}$.
5. **Statistical Kill Gate Execution (`validate_gates.py`):**
   - **Sample Size:** $N_{\text{oos}} \ge 100$ independent trades.
   - **Politis-Romano Stationary Block Bootstrap:** 10,000 resamples (mean block length $b = 3.0$). Lower confidence bound must satisfy $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > 0.00R$.
   - **Deflated Sharpe Ratio (DSR):** $\text{DSR} > 0.85$ adjusting for $K$ tested parameter combinations.
   - **Parameter Perturbation:** $\pm 20\%$ parameter grid; all nodes must maintain positive net EV.
   - **Stage B Untouched Holdout:** Single-pass evaluation on the final 6 months (`2026-05-01` to `2026-11-01`); must yield positive net PnL and fall within the Stage A 95% bootstrap CI.

---

## 16. Risk Management Requirements

Every strategy must operate under Project Alpha’s centralized risk governance framework implemented in [`alpha_core::risk`](file:///c:/Coding/alpha/rust/crates/alpha-core/src/risk.rs) and [`alpha_node::engine`](file:///c:/Coding/alpha/rust/crates/alpha-node/src/engine.rs).

### 16.1 Systemic Risk Guardrails
1. **Global High-Water Mark Circuit Breaker:**
   If total account equity drops $\ge 6.0\%$ from its historical high-water mark:
   $$\text{Drawdown} = \frac{\text{HWM} - E_t}{\text{HWM}} \ge 0.060 \implies \text{HALT\_ALL\_TRADING}$$
   The engine immediately cancels all working orders, sets `is_halted = 1` in SQLite, delivers an emergency Telegram dispatch, and refuses to trade across system reboots until manually unhalted by the operator.
2. **Per-Position Risk Budget:**
   Individual strategy risk per trade is strictly capped at $r \in [0.0075, 0.010]$ ($0.75\%\text{--}1.0\%$ of equity).
3. **Aggregate Portfolio Exposure Cap:**
   Total gross market notional across all active strategies cannot exceed $100\%$ of equity ($1.0\times$ leverage on spot). Delta-neutral strategies (Strategy 2) are allocated isolated capital buckets.
4. **Exchange Filter Invariants:**
   Every order must clear exchange `step_size` (quantity floored), `tick_size` (price rounded), `min_qty`, and `min_notional` ($\ge \$5.00$).
5. **Stale Data & Network Protection:**
   Entries are blocked if the latest market trade or kline is older than 120 seconds, if WebSocket sequence gaps are detected (`prevSeqId != last_seq_id`), or during scheduled macroeconomic calendar blackouts (`configs/calendar_blackouts.json`).

---

## 17. Rust + Python Integration Specifications

To transition from single-strategy hardcoding to multi-strategy orchestration without violating `#![forbid(unsafe_code)]`, we specify the exact Rust interfaces and type signatures to be implemented.

### 17.1 Proposed Core Strategy Trait (`alpha-core/src/traits.rs`)
```rust
#![forbid(unsafe_code)]

use alpha_types::{Bar, OrderIntent};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StrategyId {
    H1Breakout,
    CsMomentum,
    CarryArbitrage,
    FlushReversal,
    MacroTrend,
    SessionImbalance,
    CointegrationPairs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalOutcome {
    Enter(OrderIntent),
    Hold,
    Exit { reason: String },
    Abstain { reason: String },
}

pub trait QuantitativeStrategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn required_bars(&self) -> usize;
    fn evaluate(
        &self,
        symbol: &str,
        bars: &[Bar],
        equity: Decimal,
        equity_hwm: Decimal,
    ) -> Result<SignalOutcome, crate::error::CoreError>;
}
```

### 17.2 Multi-Timeframe BarBuffer Map (`alpha-node/src/buffer.rs`)
To support strategies running on 1h, 4h, and 1D bars simultaneously:
```rust
pub type BufferKey = (String, String); // (Symbol, Timeframe), e.g. ("BTCUSDT", "1h")
pub struct MultiBarBufferMap {
    buffers: std::collections::HashMap<BufferKey, BarBuffer>,
}
```

### 17.3 Multi-Strategy Concurrency Blocker in `alpha-node` and Required Engine Migration

An architectural audit of `alpha-node` reveals a critical concurrency blocker that prevents executing multiple strategies simultaneously:

1. **The Single-Position Lock in `PaperSimulator` (`alpha-node/src/simulator.rs`):**
   ```rust
   pub struct PaperSimulator {
       pub active_position: Option<PaperPosition>,
       pub working_order: Option<OrderIntent>,
       // ...
   }
   ```
   At lines 83–87 of `simulator.rs`, order submission is rejected globally if *any* position or working order exists:
   ```rust
   if self.active_position.is_some() || self.working_order.is_some() {
       return Err(NodeError::StateError {
           reason: "cannot submit order: position or working order already active".into(),
       });
   }
   ```

2. **The Global Signal Dispatch Abort in `TradeEngine` (`alpha-node/src/engine.rs`):**
   At lines 133–135 of `engine.rs`, bar processing terminates signal evaluation across *all* symbols if a single position or working order exists:
   ```rust
   if self.simulator.active_position.is_some() || self.simulator.working_order.is_some() {
       return Ok(());
   }
   crate::signal_dispatch::evaluate_and_dispatch(self, symbol, now_ms)
   ```

**Architectural Consequence:**
In the current implementation, if $H_1$ holds an active position in `BTCUSDT`, the engine will not evaluate signals for `ETHUSDT`, nor can any new strategy (`CS-MOM`, `FLUSH-REV`, `SESSION-IMB`, `COINT-PAIR`) ever submit an order. The entire node is constrained to $N \le 1$ active position across all assets and strategies.

**Required Migration Architecture:**
To support concurrent multi-strategy execution without violating Project Alpha's `#![forbid(unsafe_code)]` and zero-panic invariants:
1. **Refactor Position & Order Storage:**
   Replace the global `Option<PaperPosition>` with a composite-keyed `HashMap`:
   ```rust
   pub type PositionKey = (StrategyId, String); // (Strategy, Symbol)
   pub struct MultiStrategyPaperSimulator {
       pub active_positions: std::collections::HashMap<PositionKey, PaperPosition>,
       pub working_orders: std::collections::HashMap<PositionKey, OrderIntent>,
   }
   ```
2. **Strategy-Symbol Scoped Guard in `TradeEngine`:**
   Instead of aborting globally, `TradeEngine` checks whether the specific `(StrategyId, Symbol)` pair is already invested:
   ```rust
   let key = (strategy.id(), symbol.to_string());
   if self.simulator.active_positions.contains_key(&key) 
       || self.simulator.working_orders.contains_key(&key) 
   {
       continue; // Only skip evaluation for this specific strategy on this symbol
   }
   ```
3. **Centralized Portfolio Risk Arbiter (`alpha-core/src/risk.rs`):**
   Before dispatching any `SignalOutcome::Enter`, the engine passes the proposed order to a centralized portfolio risk arbiter that validates aggregate portfolio invariants:
   - Maximum concurrent open positions across all strategies: $N_{\text{active}} \le N_{\text{max}}$ (e.g., $\le 3$).
   - Maximum total equity at risk: $\sum_{i} r_i \le 3.0\%$.
   - Maximum single-asset concentration: notional exposure to any single coin (e.g. BTC across $H_1$ and `FLUSH-REV`) cannot exceed $40\%$ of total equity.
4. **SQLite WAL Schema Migration:**
   Add `migrations/0002_multi_strategy.sql` to reconstruct `positions` with composite primary key `PRIMARY KEY (strategy_id, symbol)`, and add `strategy_id TEXT NOT NULL DEFAULT 'H1Breakout'` to `orders`, `fills`, and `positions`.

---

## 18. Implementation Complexity Assessment

| Strategy | Reusable Components | New Adapters / Modules Required | Infrastructure Blockers | Implementation Complexity |
|---|---|---|---|---|
| **Core Engine Refactor (`alpha-node`)** | `alpha_node::db`, `alpha_node::telegram` | `MultiStrategyPaperSimulator` (`HashMap<(StrategyId, String), PaperPosition>`), composite dispatch | Existing single-position lock blocks all concurrent strategies | **MEDIUM** (Prerequisite for Multi-Strategy Phase) |
| **Strategy 1 (`CS-MOM`)** | `alpha_core::sizing`, `alpha_types::Bar`, Binance Spot connector | Cross-sectional ranking module in `alpha-core` (`cs_momentum.rs`) | Requires `alpha-node` concurrency refactor | **MEDIUM** |
| **Strategy 2 (`CARRY-ARB`)** | `alpha_venues` signer, SQLite WAL state persistence | Binance/OKX Perpetual Futures REST/WS connector (`fapi.rs`), dual-leg hedger | Requires futures API keys & `alpha-node` concurrency refactor | **HIGH** |
| **Strategy 3 (`FLUSH-REV`)** | `alpha_core::exits`, `alpha_core::sizing`, `alpha_node::simulator` | 1h Open Interest & Funding Rate REST poller in `alpha-node` | Requires `alpha-node` concurrency refactor | **LOW** |
| **Strategy 4 (`MACRO-TREND`)** | Python research engine (`walkforward.py`, `validate_gates.py`) | FRED API client, Tiingo ETF data ingestion script | Multi-broker ETF execution requires Alpaca/IBKR integration | **MEDIUM** (Crypto: Low; TradFi: High) |
| **Strategy 5 (`SESSION-IMB`)** | `alpha_core::indicators`, `alpha_node::engine`, Binance WS feed | Intraday session timer and range tracking module (`session_imb.rs`) | Requires `alpha-node` concurrency refactor | **LOW** |
| **Strategy 6 (`COINT-PAIR`)** | `alpha_core::sizing`, `alpha_types::Bar`, Binance Spot connector | OLS regression and ADF residual test module (`cointegration.rs`) | Requires `alpha-node` concurrency refactor | **MEDIUM** |

---

## 19. Known Limitations and Unresolved Questions

### 19.1 Technical & Operational Limitations
1. **Futures Account Partitioning (Strategy 2):**
   Executing delta-neutral cash-and-carry on the same account as spot breakout trading introduces cross-margin liquidation risk. A dedicated, isolated futures subaccount must be provisioned.
2. **Multi-Asset Memory Footprint:**
   Strategy 1 monitors 15 spot tokens across 180 bars of history. In `alpha-node`, maintaining 15 `BarBuffer` ring buffers on Proxmox LXC 101 consumes approximately $15 \times 180 \times 128\text{ bytes} \approx 345\text{ KB}$ of RAM, remaining well within the container's 2048 MB memory budget.
3. **Execution Latency in Liquidation Reversals (Strategy 3):**
   During severe liquidation cascades, Binance/OKX WebSocket feeds experience message queuing and latency spikes (500–2,000 ms). Strategy 3 must evaluate on confirmed 1h closed bars rather than real-time sub-second order book updates to remain immune to transient feed latency.
4. **Engine Concurrency Refactor Prerequisite:**
   Multi-strategy forward execution cannot begin until `alpha-node` is upgraded from the single-position `Option<PaperPosition>` lock to the composite `HashMap<(StrategyId, String), PaperPosition>` architecture specified in Section 17.3, backed by updated SQLite migrations.

### 19.2 Quantitative Gaps Requiring Research Priority
- **Cointegration Stability:** Has the `ETH/BTC` cointegrating vector $\beta$ exhibited non-stationary drift post-Ethereum Dencun upgrade (March 2024)? Needs ADF verification on recent 2025–2026 data.
- **Carry Reversal Friction:** In periods of sudden bear market transitions, how quickly does perpetual funding turn negative, and what is the optimal exit velocity to prevent basis erosion?

---

## 20. Recommended Testing Priority

Following the strict sequential gate protocol of Project Alpha, new candidate strategies should be developed and tested in the following order:

```
[Phase 5: Complete 8-Week Paper Burn-In of H1 (Currently Day 3 of 56)]
                                  │
                                  ▼
[Priority 1: Strategy 3 (FLUSH-REV) — Low complexity, negative correlation to H1, spot-only]
                                  │
                                  ▼
[Priority 2: Strategy 5 (SESSION-IMB) — Low complexity, intraday zero-overnight risk]
                                  │
                                  ▼
[Priority 3: Strategy 1 (CS-MOM) — Medium complexity, captures altcoin dispersion]
                                  │
                                  ▼
[Priority 4: Strategy 6 (COINT-PAIR) — Medium complexity, delta-neutral relative value]
                                  │
                                  ▼
[Priority 5: Strategy 2 (CARRY-ARB) — High complexity, requires perpetual futures account]
                                  │
                                  ▼
[Priority 6: Strategy 4 (MACRO-TREND) — Requires external TradFi ETF broker integration]
```

### Immediate Next Actions for the Engineering Team:
1. Maintain uninterrupted monitoring of live Phase 5 paper burn-in on Proxmox LXC 101 (`alpha-node.service` running steadily under PID 14836).
2. Scaffold `python/research/download_funding_oi.py` to ingest free historical funding rate and open interest Parquets from Binance Futures.
3. Implement `alpha_core::flush_rev` and execute Purged Walk-Forward backtesting for Strategy 3 (`FLUSH-REV`).
