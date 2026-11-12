# Quantitative Autopsy: Killed Strategies

To maintain absolute institutional integrity, Project Alpha enforces strict quantitative kill gates. Any strategy with negative Net Expected Value ($\text{EV} < 0$), insufficient sample size ($N < 100$), or severe fee vulnerability is permanently rejected.

During our research sprint, four candidate strategies were backtested on empirical data and formally failed. Their code was purged from the active engine to prevent codebase bloat.

---

## 1. Summary of Rejected Candidates

| Strategy Candidate | Intended Premise | Total Trades | Win Rate | Profit Factor | Net EV (R) | Total Net R | Kill Reason |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`SESSION_IMBALANCES`** | Asian/London/NY session sweeps | 568 | 41.2% | 0.68 | **-0.198R** | **-112.55R** | Catastrophic fee drag on intraday scalps |
| **`CROSS_SECTIONAL_MOMENTUM`** | Long top altcoins / short laggards | 328 | 36.8% | 0.74 | **-0.142R** | **-46.62R** | High rebalancing turnover & cross-asset beta crashes |
| **`FLUSH_REVERSAL`** | Liquidation cascade mean reversion | 12 | 25.0% | 0.41 | **-0.219R** | **-2.63R** | $N < 100$ failure & falling knife tail risk |
| **`COINTEGRATED_PAIRS`** | BTC/ETH statistical arbitrage spread | 13 | 38.5% | 0.58 | **-0.161R** | **-2.10R** | $N < 100$ failure & non-stationary ETH/BTC trend breakdown |

---

## 2. In-Depth Failure Analysis

### A. `SESSION_IMBALANCES` (Session Open Liquidity Sweeps)
* **Hypothesis:** Prices sweep the highs and lows of previous trading sessions (e.g., London Open sweeping Asian Range high) to grab retail stop liquidity before reversing.
* **Why It Failed:** 
  1. **Taker Fee Friction:** Intraday 15m and 1h swings generate small targets (+0.4R to +0.8R). At 0.05% taker fees per side (0.10% round trip), exchange fees consume 35% to 60% of the gross profit.
  2. **Adverse Selection:** When a true institutional breakout occurs during London/NY open, the sweep does not reverse—it accelerates, causing immediate full -1.0R losses.
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

By purging these four non-viable strategies:
* The codebase remains **100% lean, clean, and modular**.
* Research is focused exclusively on the three verified, orthogonal strategies with positive net mathematical edge: `VOLATILITY_BREAKOUT`, `FUNDING_CARRY`, and `MACRO_TREND`.
