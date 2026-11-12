# Project Alpha: High-Patience Spot Quantitative Trading System

> **A scale-invariant, zero-leverage algorithmic spot trading architecture designed for high-patience regime capture on digital asset markets.**

[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)
[![Python](https://img.shields.io/badge/Python-3.11%2B-blue.svg)](https://www.python.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Math](https://img.shields.io/badge/Formulation-Formal%20Quant-blueviolet.svg)](#mathematical-formulation)

---

## 1. Abstract & Quantitative Thesis

Project Alpha is a spot-native algorithmic trading engine engineered for scale-invariant capital compounding ($100 \rightarrow \$10,000+$). The system rejects high-frequency scalping, perpetual leverage, and arbitrary technical indicators in favor of a mathematically rigorous, structural trend-following model operating on multi-hour time horizons ($1\text{h}$ and $4\text{h}$ bars).

### The Core Problem in Retail Spot Trading
Most retail trading systems fail due to three structural traps:
1. **Fee Asphyxiation:** Scalping low-volatility horizons ($< 15\text{m}$) causes round-trip exchange fees ($15\text{--}20\text{ bps}$) to devour $30\%\text{--}50\%$ of gross mathematical edge.
2. **Capital Fragility on Low Balances:** Fixed lot sizes or constant-dollar bets violate scale invariance, either failing exchange minimum notional requirements ($\text{minNotional} \approx \$5\text{--}\$10$) or over-risking total equity.
3. **Chop-Induced Degradation:** Standard breakout models hemorrhage capital during prolonged sideways consolidations.

### The Alpha Solution
Alpha solves these dynamics by:
- Operating exclusively on structural swings where minimum stop distances satisfy $D_{\text{stop}} \ge 2.20\%$, compressing round-trip fee friction to $\le 0.064R$.
- Enforcing the **Zero-Action Law**: remaining 100% flat in yield-bearing fiat/stablecoins during choppy regimes is treated as a positive-EV mathematical edge, not idle downtime.
- Dynamically sizing every trade as a scale-invariant function of active equity $E_t$, trade risk fraction $r$, and volatility stop distance $D_{\text{stop}}$.

---

## 2. Strategy Architecture ($H_1$: Structural Trend Following)

The primary model ($H_1$) combines directional momentum, noise filtering, and volume confirmation to isolate genuine market expansions from false breakouts.

```
                      ┌────────────────────────┐
                      │   1h / 4h Bar Close    │
                      └───────────┬────────────┘
                                  │
                                  ▼
                      ┌────────────────────────┐
                      │ Kaufman Efficiency (ER)│ ──[ ER < θ_ER ]──► ABSTAIN (100% Cash)
                      └───────────┬────────────┘
                                  │ (ER >= θ_ER: High Trend Efficiency)
                                  ▼
                      ┌────────────────────────┐
                      │ Donchian Upper Channel │ ──[ P_t <= Upper ]► ABSTAIN
                      └───────────┬────────────┘
                                  │ (P_t > Upper: Breakout Detected)
                                  ▼
                      ┌────────────────────────┐
                      │ Taker Buy Volume Ratio │ ──[ Ratio < 0.50 ]► VETO (Fakeout)
                      └───────────┬────────────┘
                                  │ (Ratio >= 0.50: Aggressive Demand)
                                  ▼
                      ┌────────────────────────┐
                      │ Scale-Invariant Sizer  │ ──► Atomic Entry + Stop-Loss
                      └────────────────────────┘
```

---

## 3. Mathematical Formulation

### 3.1. Trend Signal & Breakout Boundary
Let $P_t = [O_t, H_t, L_t, C_t, V_t]$ represent the discrete bar at close time $t$. The $L$-period Donchian breakout envelope is defined as:

$$U_t(L) = \max_{i \in [1, L]} \{ H_{t-i} \}$$

$$D_t(L) = \min_{i \in [1, L]} \{ L_{t-i} \}$$

A raw long breakout signal $S_t^{\text{raw}} \in \{0, 1\}$ triggers if and only if the current close breaches the rolling lookback maximum:

$$S_t^{\text{raw}} = \begin{cases} 1 & \text{if } C_t > U_t(L) \\ 0 & \text{otherwise} \end{cases}$$

### 3.2. Kaufman Efficiency Ratio (KER / Anti-Chop Filter)
To immunize the system against range-bound whipsaws, price path efficiency is quantified via the Kaufman Efficiency Ratio over lookback horizon $n$:

$$\text{ER}_t(n) = \frac{\vert C_t - C_{t-n} \vert}{\sum_{i=0}^{n-1} \vert C_{t-i} - C_{t-i-1} \vert}$$

- $\text{ER}_t \rightarrow 1.0$: Perfectly directional, frictionless trend.
- $\text{ER}_t \rightarrow 0.0$: Pure Brownian motion / sideways chop.

The efficiency filter mandates:

$$F_{\text{ER}}(t) = \mathbb{I}\left( \text{ER}_t(24) \ge \theta_{\text{ER}} \right), \quad \text{where } \theta_{\text{ER}} = 0.35$$

### 3.3. Order-Flow Taker Confirmation
A breakout must exhibit net aggressive buyer dominance to filter liquidity-seeking institutional fakeouts. Using taker-buy base volume $V_{\text{buy}, t}$ and total volume $V_{\text{total}, t}$:

$$\text{TBR}_t = \frac{V_{\text{buy}, t}}{V_{\text{total}, t}}$$

$$F_{\text{vol}}(t) = \mathbb{I}\left( \text{TBR}_t \ge 0.50 \right)$$

### 3.4. Composite Execution Signal
The executable trade signal $S_t \in \{0, 1\}$ at bar close $t$ requires simultaneous satisfaction of all sub-conditions:

$$S_t = S_t^{\text{raw}} \times F_{\text{ER}}(t) \times F_{\text{vol}}(t)$$

By temporal causality, execution of $S_t$ occurs strictly at or after $t+1$.

---

## 4. Scale-Invariant Position Sizing

Fixed-dollar or fixed-coin sizing destroys small accounts and under-allocates on large accounts. Alpha defines capital allocation strictly through equity fractions and risk preservation.

### 4.1. Structural Stop Distance ($D_{\text{stop}}$)
The initial stop-loss price $P_{\text{stop}}$ is anchored to structural volatility using the 14-period Average True Range ($\text{ATR}_{14}$):

$$\text{TR}_t = \max(H_t - L_t, \vert H_t - C_{t-1} \vert, \vert L_t - C_{t-1} \vert)$$

$$P_{\text{stop}} = C_t - k_{\text{ATR}} \cdot \text{ATR}_{14}(t)$$

The fractional stop distance is:

$$D_{\text{stop}} = \frac{C_t - P_{\text{stop}}}{C_t}$$

To ensure exchange fees never degrade trade expectancy, $D_{\text{stop}}$ is bounded by a hard structural floor:

$$D_{\text{stop}} \ge \max\left(2.20\%, \; 12.5 \times C_{\text{rt, maker}}\right)$$

where $C_{\text{rt, maker}}$ is the total round-trip maker fee and slippage allowance.

### 4.2. Exact Order Sizing Formula
Given total portfolio equity $E_t$ and risk parameter $r \in [0.006, 0.010]$ ($0.6\%\text{--}1.0\%$ of equity risked per trade), unconstrained position quantity $q_{\text{raw}}$ is:

$$q_{\text{raw}} = \frac{E_t \cdot r}{C_t \cdot \left(D_{\text{stop}} + C_{\text{rt, maker}}\right)}$$

The executable order quantity $q$ enforces exchange lot-step truncation (`step_size`):

$$q = \left\lfloor \frac{q_{\text{raw}}}{\text{step\_size}} \right\rfloor \times \text{step\_size}$$

### 4.3. Notional Feasibility Gate
Before submitting any order to the matching engine, the notional value must clear exchange minimum thresholds:

$$\text{Notional} = q \times C_t \ge \text{minNotional} \quad (\approx \$5.00)$$

If $\text{Notional} < \text{minNotional}$, the engine emits an explicit `Abstain` verdict—preventing uncalculated risk expansion.

---

## 5. Multi-Horizon Exit Architecture

Alpha implements an asymmetric exit framework designed to let positive outliers compound while strictly cutting regime failures:

| Exit Class | Mechanism | Mathematical Trigger | Purpose |
|---|---|---|---|
| **$E_0$: Structural Stop** | Exchange-Hosted Atomic Stop | $P_{\text{low}} \le P_{\text{stop}}$ | Capital preservation against sudden adverse shocks. |
| **$E_1$: Trailing Channel** | Donchian Lower Envelope | $C_t < D_t(L_{\text{exit}})$ | Trend following: riding full momentum until market structurally breaks. |
| **$E_2$: Time-Decay Stop** | Horizon Expectation Decay | $\Delta t \ge T_{\text{max}} \text{ and } \text{MFE} < 0.50R$ | Liquidation of stagnant trades that fail to initiate expansion. |

```
Price
  ▲
  │                                    ┌─── Upper Channel (Entry)
  │                                ───►│
  │                         ▲     /    └───
  │                        / \   /
  │                       /   \_/
  │                      /            ───► Exit E1: Lower Channel Break
  │                     /            /
  │             ▲      /            /
  │            / \    /            /
  │           /   \__/            ▼
  │          /
  │  ───────┴───────────────────────────── Exit E0: Initial Structural Stop
  └───────────────────────────────────────► Time
```

---

## 6. Quantitative Verification & Statistical Gates

A trading hypothesis is only valid if it satisfies out-of-sample statistical significance tests. Project Alpha subjects all candidate models to a formal mathematical kill gate before capital allocation:

### 6.1. Net Trade Expectancy
Every closed trade is normalized into risk units ($R$-multiples):

$$R_i = \frac{\text{Net PnL}_i}{E_{t_i} \cdot r}$$

The net expected value is:

$$\text{EV}_{\text{net}} = \frac{1}{N} \sum_{i=1}^N R_i$$

### 6.2. 10,000-Sample Stationary Block Bootstrap
To account for temporal auto-correlation and fat-tailed return distributions, trade sequences undergo a 10,000-sample Politis–Romano stationary block bootstrap. 

The strategy is terminated if the 95% Lower Confidence Bound fails the profitability gate:

$$\text{LCB}_{95\%}(\text{EV}_{\text{net}}) \le +0.02R \implies \textbf{REJECT HYPOTHESIS}$$

### 6.3. Deflated Sharpe Ratio (DSR)
To guard against selection bias across indicator parameter combinations, the Deflated Sharpe Ratio must exceed the institutional confidence threshold:

$$\text{DSR}\left(\widehat{\text{SR}}\right) > 0.95$$

---

## 7. System Architecture & Repository Layout

Alpha is built with a dual-language architecture separating pure compute from high-level research:

- **`alpha-core` (Rust):** Pure, zero-IO Rust library compiling feature calculations, sizing mathematics, and exit state machines under `#![forbid(unsafe_code)]`.
- **`alpha-py` (PyO3):** Native C-FFI extension exposing compiled Rust functions directly to Python research pipelines, ensuring zero discrepancy between backtest and production execution.
- **`alpha-node` (Rust / Tokio):** Asynchronous live execution daemon managing WebSocket feeds, REST order routers, and SQLite Write-Ahead Logging (WAL) state recovery.
- **`python/research/` (Quantitative Research Laboratory):** Modular research suite hosting plug-and-play strategy algorithms (`strategies/`), Binance Vision data ingestion, purged walk-forward simulators, and statistical kill-gate audits.
- **`docs/backtests/`:** Empirical backtest reports, loss analyses, and multi-year trade logs across active strategies and killed candidates.

---

## 8. License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
