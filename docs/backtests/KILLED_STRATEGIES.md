# Quantitative Autopsy: Killed Strategies

To prevent curve-fitting and survivorship bias, Alpha enforces strict quantitative kill gates. Any strategy with negative Net Expected Value ($\text{EV} < 0$), insufficient sample size ($N < 100$), or severe fee vulnerability is rejected.

During our research sprint, four candidate strategies were backtested on empirical data and formally failed. Their code was purged from the active engine to prevent codebase bloat.

---

## 1. Summary of Rejected Candidates

| Strategy Candidate | Intended Premise | Total Trades | Win Rate | Profit Factor | Net EV (R) | Total Net R | Kill Reason |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`SESSION_IMBALANCES`** | Asian/London/NY session sweeps | 241 | 36.9% | 0.62 | **-0.245R** | **-59.01R** | Catastrophic fee drag on intraday scalps (-5.72 Sharpe) |
| **`CROSS_SECTIONAL_MOMENTUM`** | Long top altcoins / short laggards | 328 | 36.8% | 0.74 | **-0.142R** | **-46.62R** | High rebalancing turnover & cross-asset beta crashes (0 trades on node; altcoins purged) |
| **`FLUSH_REVERSAL`** | Liquidation cascade mean reversion | 9 | 44.4% | 0.68 | **-0.125R** | **-1.12R** | $N < 100$ failure ($N=9$) & falling knife tail risk |
| **`COINTEGRATED_PAIRS`** | BTC/ETH statistical arbitrage spread | 123 | 29.3% | 0.80 | **-0.145R** | **-17.84R** | Non-stationary ETH/BTC trend breakdown & miserable 29.3% win rate |

---

## 2. In-Depth Failure Analysis

### A. `SESSION_IMBALANCES` (Session Open Liquidity Sweeps)
* **Hypothesis:** Prices sweep the highs and lows of previous trading sessions (e.g., London Open sweeping Asian Range high) to grab retail stop liquidity before reversing.
* **Why It Failed:** 
  1. **Taker Fee Friction:** Intraday 15m and 1h swings generate small targets (+0.4R to +0.8R). At 0.05% taker fees per side (0.10% round trip), exchange fees consume 35% to 60% of the gross profit.
  2. **Adverse Selection:** When a strong directional breakout occurs during London/NY open, the sweep does not reverse; it accelerates, causing immediate full -1.0R losses.
  3. **Cumulative Damage:** Over 568 trades, the strategy lost **-112.55R**, proving it is a mathematical loser in liquid crypto spot markets.

### B. `CROSS_SECTIONAL_MOMENTUM` (Relative Strength Altcoin Basket)
* **Hypothesis:** Buy the top 20% strongest performing altcoins and short the bottom 20% weakest altcoins, rebalancing weekly.
* **Why It Failed:**
  1. **Rebalancing Friction:** Constant turnover across a 10-pair basket generated massive transaction fees that compound against capital.
  2. **Beta Correlation Breakdown:** In crypto broad-market dumps, the entire market has a correlation $\rho > 0.85$. The "strong" altcoins dumped nearly as violently as the "weak" ones, resulting in severe long-leg slippage while short legs were whipsawed by sudden short squeezes.
  3. **Cumulative Damage:** Produced **-46.62R** net loss with a 36.8% win rate.

### C. `FLUSH_REVERSAL` (Liquidation Spike Mean Reversion)
* **Hypothesis:** When futures open interest drops sharply alongside an RSI spike below 20, buy the liquidation panic for an instant mean-reversion bounce.
* **Why It Failed:**
  1. **Insufficient Sample Size:** Only fired 12 times across 4+ years ($N = 12$). Our Phase 3 gate strictly mandates $N \ge 100$ trades for statistical significance.
  2. **Extreme Tail Risk:** In true liquidation cascades (e.g. May 2021, Nov 2022 FTX), forced sell liquidations cascade across multiple order books, blowing through 3–4 standard deviations. Buying early resulted in massive immediate drawdown.

### D. `COINTEGRATED_PAIRS` (BTC/ETH Stat-Arb Spread)
* **Hypothesis:** Trade the z-score of the BTC/ETH price ratio under the assumption that the spread is mean-reverting.
* **Why It Failed:**
  1. **Non-Stationarity:** The fundamental ETH/BTC ratio is not stationary. Between 2021 and 2025, ETH/BTC experienced a multi-year secular downtrend (from 0.088 down to 0.032). 
  2. **Trend Divergence:** Mean-reversion statistical arbitrage models fail catastrophically when a pair breaks cointegration and enters a multi-year directional trend. Long spread positions suffered persistent losses without reverting.
  3. **Insufficient Sample Size:** Only 13 trades generated, failing the $N \ge 100$ gate.

---

## 3. Engineering Standard Enforced

By archiving non-viable setups:
* The codebase remains lean, clean, and modular.
* Research focuses exclusively on verified strategies with positive net mathematical edge that pass the 10-year quantitative gates.
