# Quantitative Stress Testing & Adversarial Validation Protocol (`STRESS_TEST_STRAT.md`)

## 1. Philosophy: Adversarial Red-Teaming

Once a strategy passes Evaluation and Improvement, the quant switches roles from **builder** to **adversary**. The sole mission of stress testing is to **break the strategy** and uncover hidden failure modes before real capital is exposed.

Backtests measure performance on *one single historical realization* under clean assumptions. Real markets deliver regime shifts, liquidity droughts, basis blowouts, latency spikes, and $6\sigma$ fat-tail events.

> **Core Rule:** Never ask *"How high can the APR go?"* Ask *"What exact market condition will wipe this strategy out, and how plausible is it?"*

---

## 2. Execution, Friction & Microstructure Degradation Tests

### 2.1 Cost Multiplier & Break-Even Death Point
Re-run the full strategy while scaling total transaction costs (fees + spread + slippage) by $1.5\times$, $2.0\times$, $3.0\times$, and $5.0\times$:
- **Break-Even Cost Multiplier ($C_{\text{BE}}$):** Find the exact cost multiplier (or extra slippage in $\text{bps}$ per side) where Net Sharpe drops to $0.0$ (or Net EV $= 0$).
- **Pass Criteria:**
  - Strategy must remain solidly profitable ($\text{SR} \ge 1.0$, $\text{EV} > 0$) at **$2.0\times$ baseline realistic costs**.
  - Cushion in $\text{bps}$ must absorb realistic live spread expansion during volatile periods.

### 2.2 Latency, Fill Delay & Worst-Case Intra-Bar Fills
- **Execution Lag Injection:** Delay order execution by $+1$ tick, $+500\text{ ms}$, $+1$ bar, and $+2$ bars (scaled to strategy frequency). If a single bar of delay cuts Sharpe by $> 50\%$ on a daily/hourly strategy, the backtest is likely suffering from microstructure illusion or subtle look-ahead leakage.
- **Pessimistic OHLC Bar Assumptions:** For bar-level backtests, force worst-case fill assumptions:
  - Entries execute at a penalizing fraction toward bar High (buys) / bar Low (sells).
  - If both Stop-Loss and Take-Profit lie inside the same bar's High-Low range, assume **Stop-Loss triggers first $100\%$ of the time**.
- **Adverse Selection Stress (Limit Orders):** For passive strategies, simulate $20\%$ to $50\%$ missed fills on winning trades (price touches limit and immediately runs away) while keeping $100\%$ fill rates on losing trades (price slices right through limit).

### 2.3 Liquidity & Volume Participation Caps
- Cap executable size per bar at $1\%$, $2\%$, and $5\%$ of actual historical bar volume.
- Measure **Capacity Decay Curve:** Plot Net Sharpe and CAGR as portfolio AUM scales from $\$100\text{k} \to \$1\text{M} \to \$10\text{M} \to \$100\text{M}$ using square-root market impact modeling. Identify the exact AUM ceiling where marginal alpha hits zero.

---

## 3. Statistical Resampling & Monte Carlo Stress Suites

Run at least $10{,}000$ synthetic iterations for each statistical test.

### 3.1 Trade-Sequence Bootstrap (Path Dependency & Risk of Ruin)
Randomly resample trade returns with replacement (or shuffle trade order) $10{,}000$ times:
- Extract the **95th percentile** and **99th percentile** of:
  - Maximum Drawdown ($\text{Max DD}_{95\%}, \text{Max DD}_{99\%}$)
  - Longest Consecutive Losing Streak
  - Longest Underwater Duration
- **Risk of Ruin (RoR):** Probability of equity falling below a critical ruin threshold (e.g., $-25\%$ or $-40\%$ drawdown).
- **Rule of Thumb:** Expect live Max Drawdown to reach $1.5\times$ to $2.0\times$ historical backtest Max DD; size leverage so $\text{Max DD}_{99\%}$ remains survivable.

### 3.2 Stationary / Block Bootstrap (Preserving Volatility Clustering)
Standard shuffling breaks serial correlation and volatility regimes. Use **Circular/Stationary Block Bootstrap** (sampling contiguous blocks of length $L \sim \text{Geometric}(p)$ days, e.g., average block $10$ to $20$ days) to preserve crisis clustering and drawdown persistence.

### 3.3 Monte Carlo Permutation Test (Signal Significance / Monkey Test)
Destroy the link between signals and future returns while preserving asset return distributions and strategy trade frequency/exposure:
- Generate $1{,}000+$ randomized strategies (randomly shuffled entry dates or circularly shifted signal vectors).
- Compute the empirical $p$-value:
  $$p = \frac{1 + \sum_{k=1}^{K} \mathbb{I}(\text{SR}_{\text{rand}, k} \ge \text{SR}_{\text{actual}})}{K + 1}$$
- **Pass Criteria:** $p < 0.01$ (strategy beats $99\%$ of random permutations with identical long/short exposure bias).

### 3.4 Data Noise Injection & Missed-Trade Stress
- **Synthetic Price Jitter:** Add Gaussian noise ($\pm 0.1\sigma$ to $\pm 0.5\sigma$ of bar range) to OHLCV inputs across $100$ runs. If small price perturbations radically alter entries or collapse Sharpe, the strategy is overfit to exact historical ticks.
- **Random Trade Omission:** Randomly drop $10\%$ and $20\%$ of trades (simulating exchange outages, API timeouts, or margin constraints) across $1{,}000$ runs. Verify the 5th percentile Sharpe remains robust.

---

## 4. Parameter, Outlier & Cross-Market Fragility Tests

### 4.1 Simultaneous Worst-Case Parameter Perturbation
Perturb **all** free parameters simultaneously by $\pm 10\%$, $\pm 20\%$, and $\pm 30\%$ (testing the corners of the hypercube, not just one parameter at a time):
- **Pass Criteria:** At least $80\%$ of perturbed parameter sets remain profitable, and average perturbed Sharpe stays within $75\%$ of baseline.

### 4.2 Outlier Trimming Test (Windfall Dependency)
Remove the top $1\%$, $5\%$, and $10\%$ most profitable trades (and separately, the best $5$ calendar days) and recompute Total Return, APR, EV, and Sharpe:
- **Pass Criteria:** Strategy must remain **net profitable ($\text{EV} > 0$, $\text{SR} > 0.7$)** after removing the top $5\%$ of trades. (Note: Trend-following strategies naturally rely on right-tail winners, so evaluate trimming relative to strategy archetype—ensure profits are spread across multiple trend episodes, not one historic bubble.)

### 4.3 Out-of-Universe & Synthetic Regime Transfer
- **Unseen Asset Test:** Run the exact same logic and parameters (zero re-tuning) on adjacent unseen assets (e.g., train on BTC/ETH, test on SOL/AVAX; train on S&P 500 constituents, test on European/Japanese equities or mid-caps).
- **Inverted Series Test:** Run symmetric long/short strategies on inverted price series ($P'_t = 1 / P_t$) to verify there is no hidden directional bull-market bias disguised as mean reversion or momentum.

---

## 5. Historical Crisis Replay & Synthetic Shock Scenarios

### 5.1 Historical Stress Replay
Isolate performance during major historical market dislocations (relevant to the asset class):
- **TradFi / Macro:** 2008 GFC, May 2010 Flash Crash, Jan 2015 CHF Unpeg, Feb 2018 Volmageddon, Mar 2020 COVID Liquidity Freeze, 2022 Rate-Hike / Inflation Bear Market, Aug 2024 Yen Carry Unwind.
- **Crypto / Digital Assets:** Mar 2020 Black Thursday, May 2021 Deleveraging, May 2022 Terra/LUNA Collapse, Nov 2022 FTX Contagion, Oct/Aug Flash Wicks & Stablecoin De-pegs.

Measure for every crisis window: Max Drawdown, peak margin utilization, slippage amplification, and recovery speed.

### 5.2 Synthetic Extremes & Structural Breaks
Inject artificial stress scenarios directly into historical price streams:
1. **Overnight Gap / Flash Crash Shock:** Insert instantaneous $\pm 15\%$ to $\pm 30\%$ price gaps directly across open stop-losses (stops fill at the post-gap price, not the trigger price).
2. **Volatility Explosion:** Multiply return variance by $3\times$ to $5\times$ over a 30-day window to test whether volatility targeting and stops adapt fast enough.
3. **Prolonged Adverse Regime:**
   - For Trend strategies: Inject a 12-month choppy, mean-reverting whipsaw regime.
   - For Mean-Reversion / Grid strategies: Inject an uninterrupted $50\%$ one-way runaway trend with zero pullbacks.
4. **Correlation-to-One Shock:** In multi-asset or multi-strategy portfolios, force pairwise return correlations to $\rho \to +0.85$ and $+1.0$ during drawdown weeks to estimate worst-case simultaneous loss.

---

## 6. Basis, Funding, Tail-Risk & Leverage Stress Tests

### 6.1 Basis Dislocation & Carry Flip Stress
Essential for futures, perpetuals, ETFs, pair trading, and cash-and-carry strategies:
- **Basis Blowout Simulation:** Shock the spot-futures basis by $\pm 5\sigma$ (e.g., sudden $-5\%$ to $-15\%$ backwardation or $+20\%$ annualized contango spike). Verify mark-to-market unrealized losses do not trigger forced liquidation before convergence.
- **Funding & Borrow Squeeze:**
  - Flip perpetual funding rates to extreme sustained levels (e.g., $-0.10\%$ to $-0.30\%$ per 8 hours against open positions for $7$ consecutive days).
  - For equity shorts, model hard-to-borrow rates spiking to $50\%+$ annualized APR plus forced share recalls.
- **Cross-Exchange / Stablecoin De-Peg Risk:** For cross-venue arbitrage, model temporary withdrawal halts and collateral de-pegging ($1\text{ USDT/USDC} \to \$0.92$).

### 6.2 Extreme Tail Risk & Margin Survival
Compute formal tail-risk metrics on daily and intraday returns:
- **Value at Risk ($\text{VaR}_{99\%}$):** Loss threshold exceeded only $1\%$ of periods.
- **Conditional Value at Risk ($\text{CVaR}_{99\%}$ / Expected Shortfall):** Average loss given that the loss exceeds $\text{VaR}_{99\%}$:
  $$\text{CVaR}_{\alpha} = \mathbb{E}\left[ -r \;\middle|\; -r \ge \text{VaR}_{\alpha} \right]$$
- **Liquidation Distance Test:** Under maximum strategy leverage, verify that a simultaneous $8\sigma$ to $10\sigma$ adverse gap across correlated positions leaves equity strictly above Maintenance Margin (zero forced liquidation / Auto-Deleveraging risk).

---

## 7. Operational & Infrastructure Failure Mode Testing

Even a mathematically flawless alpha model can blow up from software or venue failures. Test explicit responses to:
- **Feed & Connectivity Loss:** Stale quotes, frozen WebSockets, or $5$-minute API blackouts while holding leveraged positions (must trigger fail-safe order cancellation or broker-side hard stops).
- **Position Reconciliation Drift:** Partial fills or dropped execution confirmations causing internal state to diverge from exchange state (requires automated periodic state reconciliation).
- **Runaway Loop / Fat-Finger Guardrails:** Hard pre-trade risk limits independent of strategy code:
  - Max single-order notional cap
  - Max orders per second / minute (rate-limit circuit breaker)
  - Max gross/net exposure cap
  - Hard Daily Loss Limit (e.g., halt trading for 24h if daily PnL $\le -3\sigma_{\text{daily}}$)

---

## 8. Final Stress-Test Sign-Off & Live Capital Ramp

### 8.1 Master Stress-Test Pass/Fail Gate

| Stress Test Category | Stress Condition | Mandatory Survival Standard |
| :--- | :--- | :--- |
| **Cost Degradation** | $2.0\times$ realistic fees + slippage | Net $\text{SR} \ge 1.0$ and $\text{EV}_{\text{bps}} > 0$ |
| **Execution Lag** | $+1$ bar / realistic latency delay | Retains $\ge 65\%$ of baseline Sharpe |
| **Monte Carlo Bootstrap** | $10{,}000$ paths (99th percentile DD) | $\text{Max DD}_{99\%} \le$ Mandate Risk Limit; $\text{RoR} < 1\%$ |
| **Signal Permutation** | $1{,}000+$ random signal shuffles | Empirical $p$-value $< 0.01$ |
| **Parameter Corners** | All parameters shifted $\pm 20\%$ | $\ge 80\%$ profitable; mean $\text{SR} \ge 0.75 \times \text{SR}_{\text{base}}$ |
| **Outlier Trimming** | Remove top $5\%$ best trades | Net $\text{EV} > 0$ and $\text{SR} > 0.7$ |
| **Gap / Tail Shock** | Instantaneous $8\sigma$ adverse gap | Zero margin call / zero account liquidation |
| **Basis & Carry Shock** | $5\sigma$ basis blowout + extreme funding | Survives mark-to-market drawdown without forced exit |

### 8.2 Staged Live Deployment & Live Decay Kill-Switches
A strategy that passes all stress tests **still never receives full capital on Day 1**:

1. **Stage 1 — Shadow / Paper Execution ($2$ to $4$ weeks):** Run live data feeds in real time; compare live simulated fills against backtest engine predictions to catch implementation discrepancies.
2. **Stage 2 — Seed Capital ($1\%$ to $5\%$ target AUM):** Verify real-world slippage, maker fill rates, funding/borrow costs, and basis tracking against backtest assumptions.
3. **Stage 3 — Scaled Production ($25\% \to 50\% \to 100\%$):** Ramp only when realized live EV and slippage stay within expected confidence intervals.
4. **Live Alpha Decay & Kill-Switch Protocol:**
   - Monitor rolling live **Implementation Shortfall** ($\text{Live Slippage} - \text{Modeled Slippage}$).
   - Run a statistical **CUSUM / SPRT (Sequential Probability Ratio Test)** on live trade returns against expected $\mu$ and $\sigma$.
   - **Automatic De-Risking / Kill Trigger:** Pause strategy and trigger full forensic review if live drawdown breaches $1.5\times$ backtest Max DD, or if rolling realized Sharpe falls below the $95\%$ lower confidence bound of historical expectations.