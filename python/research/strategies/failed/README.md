# Quantitative Autopsy: Failed Strategy Laboratory Archive

This folder archives quantitative trading strategy candidates that were implemented, backtested on empirical data, and formally failed Project Alpha's quantitative kill gates.

They are maintained here under the same `BaseStrategy` interface so that any researcher can run, reproduce, audit, or iterate on them at any time with zero friction.

---

## Summary of Rejected Candidates

| Strategy ID | Strategy Class | Primary Asset | Interval | Win Rate | Profit Factor | Net EV (R) | Kill Gate Violation |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`SESSION_IMBALANCES`** | `SessionImbalancesStrategy` | BTC, ETH, SOL | 1h / 15m | 41.2% | 0.68 | **-0.198R** | Negative Net EV ($\text{EV} < 0$), Fee Drag ($\ge 45\%$ of profit) |
| **`CROSS_SECTIONAL_MOMENTUM`**| `CrossSectionalMomentumStrategy` | Top Altcoins | 12h / 24h | 36.8% | 0.74 | **-0.142R** | Negative Net EV ($\text{EV} < 0$), High Rebalance Friction |
| **`FLUSH_REVERSAL`** | `FlushReversalStrategy` | BTC, ETH | 1h | 25.0% | 0.41 | **-0.219R** | Sample Size ($N = 12 < 100$), Extreme Tail Risk |
| **`COINTEGRATED_PAIRS`** | `CointegratedPairsStrategy` | ETH / BTC | 1h / 4h | 38.5% | 0.58 | **-0.161R** | Sample Size ($N = 13 < 100$), Non-Stationary Spread |

To reproduce any of these backtests:
```python
from python.research.strategies import get_available_strategies
strat = get_available_strategies(tier="failed")["SESSION_IMBALANCES"]()
perf = strat.run_backtest(data_dir)
print(f"Profit Factor: {perf.profit_factor:.2f}, Net EV: {perf.ev_net_r:+.4f}R")
```
