# Quantitative Strategy Evaluation Protocol (`EVALUATE_STRAT.md`)

## 1. Objective & Philosophy

The purpose of the Evaluation stage is to act as a **strict, unbiased filter**. Most raw strategy ideas are statistical illusions caused by noise, transaction cost underestimation, or hidden data leakage. Before investing research hours into optimization or deployment, every candidate strategy must pass standardized diagnostic gates across data integrity, return quality, risk-adjusted efficiency, trade-level expected value (EV), basis/carry attribution, and portfolio diversification value.

> **Core Principle:** A backtest is not proof of edge; it is only a screening mechanism to eliminate ideas that fail even under idealized historical conditions.

---

## 2. Pre-Flight Data & Simulation Hygiene (Mandatory Checks)

Before inspecting a single performance metric, verify that the backtest engine and dataset are clean. Any violation invalidates the entire run.

1. **Zero Look-Ahead Bias:** Signals computed at bar $t$ must only use data strictly available at or before timestamp $t$, with execution occurring at $t + 1$ (next bar open, next tick, or realistic delayed fill).
2. **Zero Survivorship Bias:** Asset universes (equities, crypto tokens, futures) must include delisted, bankrupt, merged, or halted instruments at their historical timestamps.
3. **Corporate Actions & Continuous Contracts:**
   - Equities must be split- and dividend-adjusted cleanly (using raw prices for volume/liquidity filters and adjusted returns for PnL).
   - Futures must use proper roll calendars and back-adjusted or ratio-adjusted series without trading artificial roll gaps.
4. **Data Partitioning Hygiene:**
   - **In-Sample (IS) / Training:** Typically $50\%$ to $60\%$ of historical data (used for hypothesis discovery).
   - **Validation Set:** $20\%$ (used during `IMPROVE_STRAT.md` iterations).
   - **Out-of-Sample (OOS) / True Holdout:** $20\%$ to $30\%$ strictly locked until final verification.
   - Apply **Purging & Embargoing** around split boundaries so multi-bar overlapping labels do not leak information across folds.
5. **Baseline Friction Inclusion:** Never evaluate a "gross-only" strategy as final. Baseline evaluation must include realistic maker/taker fees, bid-ask spread, borrow/funding rates, and conservative baseline slippage.

---

## 3. Core Return & Compounding Metrics

### 3.1 Total Return (Arithmetic vs. Geometric)
Measure both **compounded (geometric)** and **uncompounded (fixed-notional / arithmetic)** returns. Fixed-notional evaluation isolates pure signal quality from position sizing and compounding distortion.

- **Geometric Total Return (Compounded):**
  $$R_{\text{total}} = \prod_{t=1}^{T} (1 + r_t) - 1 = \frac{E_T - E_0}{E_0}$$
- **Arithmetic Total Return (Fixed Capital):**
  $$R_{\text{arith}} = \sum_{t=1}^{T} r_t$$
  where $E_0$ is starting equity, $E_T$ is ending equity, and $r_t$ is the net period return.

### 3.2 APR vs. CAGR (APY)
Distinguish strictly between simple annualized return (**APR**) and compounded annualized growth (**CAGR / APY**):

- **APR (Annual Percentage Return — non-compounded):**
  $$\text{APR} = \bar{r} \times N_{\text{ann}}$$
  where $\bar{r}$ is mean periodic return and $N_{\text{ann}}$ is the annualization factor ($252$ for daily TradFi equities/futures, $365$ for 24/7 crypto, $252 \times 6.5$ for hourly US equities, etc.).
- **CAGR (Compound Annual Growth Rate):**
  $$\text{CAGR} = \left( \frac{E_T}{E_0} \right)^{\frac{1}{Y}} - 1$$
  where $Y$ is the exact duration of the backtest in years.

**Quant Rule:** Always inspect the **Volatility Drag** gap between APR and CAGR:
$$\text{CAGR} \approx \text{APR} - \frac{1}{2}\sigma_{\text{ann}}^2$$
If APR looks impressive but CAGR collapses, the strategy suffers from excessive variance or leverage decay.

### 3.3 Benchmark-Relative Alpha & Capital Efficiency
- **Excess Return:** $R_{\text{excess}} = R_{\text{strat}} - R_{f}$ (subtract risk-free rate $R_f$, e.g., T-bill yield).
- **CAPM Alpha ($\alpha$) and Beta ($\beta$):**
  $$r_{\text{strat}, t} - r_{f, t} = \alpha + \beta (r_{\text{bench}, t} - r_{f, t}) + \epsilon_t$$
  Verify whether a strategy is generating genuine alpha or merely disguising leveraged market beta ($\beta > 0.5$ in a bull market).
- **Return on Margin / Capital Utilization:** Track **Average Capital Exposure** (percentage of time and equity actively deployed) and **Return on Max Margin Used** to judge true capital efficiency.

---

## 4. Risk-Adjusted Performance Ratios

### 4.1 Annualized Sharpe Ratio
The primary gauge of return per unit of total volatility:
$$\text{SR} = \frac{\mathbb{E}[r_t - r_{f,t}]}{\sigma(r_t - r_{f,t})} \times \sqrt{N_{\text{ann}}}$$

**Institutional Benchmarks (Net of Costs, OOS):**
- $\text{SR} < 0.8$: **Reject** (unless acting as a negatively correlated crash hedge).
- $0.8 \le \text{SR} < 1.5$: **Borderline / Portfolio Filler** (viable only if low correlation to existing book and high capacity).
- $1.5 \le \text{SR} < 2.5$: **Strong Candidate** (proceed to improvement and stress testing).
- $2.5 \le \text{SR} < 4.0$: **High-Frequency / Niche Edge** (valid for HFT, stat-arb, or market making; rare for low-frequency directional strategies).
- $\text{SR} \ge 4.0$ (Low/Mid Frequency): **Extreme Red Flag** — immediately audit for look-ahead bias, bug in fee modeling, or severe overfitting.

### 4.2 Deflated Sharpe Ratio (DSR) & Probabilistic Sharpe Ratio (PSR)
Standard Sharpe assumes i.i.d. normal returns and ignores how many trials were tested. Quants evaluate statistical significance using **PSR** and **DSR**:

$$\text{PSR}(\text{SR}^*) = \Phi\left( \frac{(\widehat{\text{SR}} - \text{SR}^*) \sqrt{T - 1}}{\sqrt{1 - \hat{\gamma}_3 \widehat{\text{SR}} + \frac{\hat{\gamma}_4 - 1}{4} \widehat{\text{SR}}^2}} \right)$$

where $\hat{\gamma}_3$ is return skewness, $\hat{\gamma}_4$ is kurtosis (normal $= 3$), $T$ is number of return observations, and $\Phi$ is the standard normal CDF.
- **DSR** sets the benchmark $\text{SR}^*$ to the expected maximum Sharpe ratio achievable by pure chance across $M$ independent strategy trials tested.
- **Pass Threshold:** $\text{DSR} \ge 0.95$ (at least $95\%$ probability that true Sharpe exceeds zero after accounting for non-normality and multiple testing).

### 4.3 Complementary Risk Ratios
| Metric | Formula | What It Catches | Target Threshold |
| :--- | :--- | :--- | :--- |
| **Sortino Ratio** | $\frac{\mathbb{E}[r - r_f]}{\sigma_{\text{downside}}} \times \sqrt{N_{\text{ann}}}$ | Penalizes only downside volatility (better for positive-skew trend strategies) | $\ge 2.0$ |
| **Calmar / MAR Ratio** | $\frac{\text{CAGR}}{|\text{Max Drawdown}|}$ | Measures return relative to worst peak-to-trough pain | $\ge 1.5$ (ideally $\ge 2.5$) |
| **Profit Factor (PF)** | $\frac{\sum \text{Gross Wins}}{|\sum \text{Gross Losses}|}$ | Ratio of dollars won to dollars lost (net of costs) | $\ge 1.35$ (Mid/Low Freq) |
| **Tail Ratio** | $\frac{|\text{Percentile}_{95}(r)|}{|\text{Percentile}_{5}(r)|}$ | Asymmetry of extreme right tail vs. left tail | $\ge 1.0$ |
| **Information Ratio (IR)** | $\frac{\mathbb{E}[r_{\text{strat}} - r_{\text{bench}}]}{\sigma(r_{\text{strat}} - r_{\text{bench}})} \times \sqrt{N_{\text{ann}}}$ | Consistency of active return over benchmark (tracking error adjusted) | $\ge 0.75$ |

---

## 5. Trade-Level Economics: EV, Basis Points & Carry Attribution

Portfolio curves can hide fragile trade mechanics. Every strategy must be dissected down to the individual trade level.

### 5.1 Expected Value (EV / Expectancy)
Net expected profit per trade (in currency, percentage, or risk units $R$):
$$\text{EV} = \left( P_{\text{win}} \times \overline{W} \right) - \left( P_{\text{loss}} \times \overline{L} \right) - C_{\text{round-trip}}$$

where:
- $P_{\text{win}}$ = Win rate, $P_{\text{loss}} = 1 - P_{\text{win}}$
- $\overline{W}$ = Average winning trade, $\overline{L}$ = Average losing trade (absolute value)
- $C_{\text{round-trip}}$ = Total round-trip friction (entry/exit fees + bid-ask spread + slippage + carry/funding)

**Key Ratios:**
- **Payoff Ratio (Reward-to-Risk):** $b = \frac{\overline{W}}{\overline{L}}$
- **Breakeven Win Rate:** $P_{\text{BE}} = \frac{1}{1 + b}$
- **Edge Margin:** Strategy must maintain a comfortable buffer above $P_{\text{BE}}$ so minor execution degradation does not flip EV negative.

### 5.2 Average Trade in Basis Points ($\text{bps}$)
Express net EV per unit of notional turnover in **basis points** ($1\text{ bps} = 0.01\% = 10^{-4}$):
$$\text{EV}_{\text{bps}} = \frac{\text{Net PnL}}{\text{Total Traded Notional}} \times 10{,}000$$

**Friction Survival Rules (Heuristic Minimums):**
- **HFT / Market Making (sub-minute):** Net edge $\ge 1$ to $5\text{ bps}$ (only viable with co-location, maker rebates, and low latency).
- **Intraday / Short-Term Alpha (minutes to hours):** Net edge $\ge 10$ to $25\text{ bps}$ round-trip.
- **Swing / Multi-Day Strategies:** Net edge $\ge 40$ to $100+\text{ bps}$ round-trip.
- **Cost-to-Gross-Edge Ratio:** Total costs should consume **no more than $30\%$ to $40\%$** of gross alpha. If costs consume $> 60\%$ of gross profit, the strategy is too fragile for live slippage.

### 5.3 Basis & Carry Decomposition
Whenever a strategy trades derivatives (futures, perpetual swaps, options), cross-exchange pairs, or ETFs vs. underlyings, PnL **must** be separated into **Directional Price Alpha** and **Basis / Carry PnL**:

- **Spot-Futures Basis:**
  $$\text{Basis}_t = F_t - S_t, \qquad \text{Annualized Basis} = \left(\frac{F_t - S_t}{S_t}\right) \times \frac{365}{D_{\text{expiry}}}$$
- **Perpetual Funding / Borrow Carry:**
  $$\text{PnL}_{\text{net}} = \text{PnL}_{\text{price}} + \text{PnL}_{\text{funding/carry}} + \text{PnL}_{\text{basis convergence}} - \text{Costs}$$
- **Basis Risk Check:**
  - Verify if an apparently "alpha-rich" long-futures strategy is merely harvesting backwardation carry (or if a short-futures strategy is getting bled out by roll yield / funding).
  - For market-neutral / stat-arb / cash-and-carry strategies, measure **Basis Volatility** ($\sigma_{\text{basis}}$) and **Maximum Historical Basis Divergence** to ensure mark-to-market basis blowouts cannot trigger liquidation before convergence.

---

## 6. Statistical Significance & Distribution Diagnostics

### 6.1 Sample Size & Hypothesis Testing
- **Minimum Independent Trades:**
  - Intraday / Short-term: $N \ge 300$ trades.
  - Swing / Daily: $N \ge 100$ trades across at least $2$ to $3$ distinct market regimes (bull, bear, sideways, high-vol).
- **$t$-Statistic of Strategy Returns:**
  $$t\text{-stat} = \frac{\bar{r}}{s / \sqrt{N}} \approx \text{SR}_{\text{ann}} \times \sqrt{Y}$$
  - Single-hypothesis minimum: $t\text{-stat} \ge 2.0$ ($p < 0.05$).
  - Multiple-testing institutional hurdle (Harvey-Liu-Zhu standard): $t\text{-stat} \ge 3.0$.

### 6.2 Drawdown & Underwater Profile
Evaluate the full geometry of losses, not just the single worst point:
- **Maximum Drawdown ($\text{Max DD}$):** Worst peak-to-trough percentage drop.
- **Max Drawdown Duration:** Longest calendar duration spent underwater (from equity peak until a new high-water mark). Strategies underwater for $> 12$ to $18$ months face severe institutional redemption/abandonment risk.
- **Ulcer Index / Pain Index:** Root-mean-square depth of all drawdowns over time (captures prolonged grinding drawdowns).
- **Daily / Trade Concentration:** Check whether $> 30\%$ of total net profit comes from the single best trade or top $3$ trading days (fragile outlier reliance).

### 6.3 Return Moments (Skewness & Kurtosis)
- **Skewness ($\gamma_3$):**
  - Positive skew ($\gamma_3 > 0$): Typical of trend-following / breakout strategies (many small losses, few large wins — psychologically hard, structurally resilient).
  - Negative skew ($\gamma_3 < -0.5$): Typical of short-vol, grid, martingale, or tight-TP/wide-SL mean reversion ("picking up pennies in front of a steamroller"). Requires strict tail-risk hedging and higher Sharpe hurdle.
- **Excess Kurtosis ($\gamma_4 - 3$):** High kurtosis ($\gg 3$) signals fat tails; standard Gaussian volatility severely understates true tail risk.

### 6.4 Turnover, Holding Period & Capacity
- **Annualized Turnover:** $\frac{\text{Total Traded Volume}}{\text{Average Equity} \times Y}$. High turnover compounds execution risk.
- **Liquidity / Capacity Estimate:** Calculate average order size as a percentage of Average Daily Volume ($\text{ADV}$) or bar volume. As a baseline rule, single-bar execution should remain $< 1\%$ to $2\%$ of bar volume to prevent severe market impact.

---

## 7. Multi-Strategy Portfolio Context (Marginal Value Check)

When evaluating multiple strategies simultaneously, never rank them solely in isolation:

1. **Pairwise Return Correlation ($\rho_{i,j}$):** Compute correlation of daily returns (and **tail-period correlation** during drawdown days) against all existing live strategies.
2. **Marginal Sharpe Contribution:** A candidate strategy $j$ improves an existing portfolio $P$ if and only if:
   $$\text{SR}_j > \rho_{j, P} \times \text{SR}_P$$
   *Implication:* A strategy with $\text{SR} = 1.1$ and $\rho = 0.05$ to your existing book adds far more portfolio value than a strategy with $\text{SR} = 1.9$ and $\rho = 0.85$.
3. **Strategy Capacity & Operational Complexity:** Penalize strategies that require bespoke data feeds or brittle infrastructure unless their risk-adjusted dollar capacity justifies the engineering maintenance cost.

---

## 8. Master Evaluation Scorecard & Decision Gate

| Evaluation Dimension | Metric | Reject (Kill) | Iterate (`IMPROVE_STRAT`) | Advance (`STRESS_TEST_STRAT`) |
| :--- | :--- | :--- | :--- | :--- |
| **Risk-Adjusted** | Net OOS Sharpe ($\text{SR}$) | $< 0.8$ | $0.8 - 1.5$ | $\ge 1.5$ (and $< 4.0$ verified) |
| **Multiple Testing** | Deflated Sharpe ($\text{DSR}$) | $< 0.70$ | $0.70 - 0.94$ | $\ge 0.95$ |
| **Statistical Power** | $t$-Statistic / Trade Count | $t < 1.5$ or $N < 50$ | $1.5 \le t < 2.5$ | $t \ge 2.5$ and $N \ge 100+$ |
| **Drawdown Efficiency** | Calmar Ratio | $< 0.75$ | $0.75 - 1.5$ | $\ge 1.5$ |
| **Trade Economics** | Net EV ($\text{bps}$) / Cost Share | $\text{EV} \le 0$ or Costs $> 60\%$ gross | Positive EV, Costs $40\%-60\%$ | Healthy $\text{EV}_{\text{bps}}$, Costs $< 35\%$ |
| **Profitability** | Net Profit Factor | $< 1.15$ | $1.15 - 1.35$ | $\ge 1.35$ |
| **Concentration** | Profit from Top $5\%$ Trades | $> 60\%$ of total PnL | $40\% - 60\%$ | $< 40\%$ (consistent edge) |
| **Portfolio Fit** | Correlation to Existing Book | $\rho > 0.85$ with lower SR | $0.40 \le \rho \le 0.85$ | $\rho < 0.40$ (true diversifier) |

### Automatic Red Flags (Immediate Audit or Disqualification)
- Equity curve looks like a straight $45^\circ$ line with near-zero drawdown on low-frequency data (almost always look-ahead bias or bad fill logic).
- Strategy relies on martingale/averaging-down without a hard capital stop.
- Strategy only works on a single hand-picked asset or a single historical bull/bear year.
- Gross Sharpe is $3.0$, but net Sharpe drops below $0.5$ after realistic spread and fee assumptions.