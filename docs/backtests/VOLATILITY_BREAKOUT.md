# Strategy Backtest Report: VOLATILITY_BREAKOUT

## 1. Executive Summary

`VOLATILITY_BREAKOUT` (formerly designated as benchmark strategy `H1`) is a high-patience spot crypto trend-following system operating on 4-hour bars across Bitcoin (`BTCUSDT`) and Ethereum (`ETHUSDT`).

* **Core Premise:** Crypto markets spend 70%+ of their time in non-profitable mean-reverting chop. This strategy sits 100% flat in cash until explosive volatility breaks a 20-period Donchian channel with Kaufman Efficiency Ratio confirmation, then rides the trend with a multi-phase ratcheting trailing stop.
* **Asymmetric Risk Profile:** Classic trend-following asymmetry. Low win rate (32.7%) offset by a high payoff ratio (3.07:1), producing a statistically verified positive net expected value of **+0.156R per trade**.

---

## 2. Key Performance Metrics

| Metric | Empirical Value | Description |
| :--- | :--- | :--- |
| **Asset Universe** | `BTCUSDT`, `ETHUSDT` | Spot pairs only (zero leverage, zero liquidation risk) |
| **Bar Timeframe** | 4-Hour (`4h`) | High signal-to-noise ratio; immune to sub-minute HFT front-running |
| **Test Period** | 2021-02-06 to 2026-10-22 | 2,083 calendar days (5.70 years) |
| **Total Trades** | 168 | ~29.5 trades per calendar year across both pairs |
| **Win Rate** | **32.74%** (55 wins) | 67.26% losses (113 losses) |
| **Payoff Ratio (Win / Loss)** | **3.07** | Average win (+1.45R) is 3x larger than average loss (-0.47R) |
| **Profit Factor (PF)** | **1.49** | Gross profit / gross loss |
| **Net Expected Value (EV)** | **+0.1564R** / trade | Net of Binance spot maker entry (0.00%) & taker exit fees (0.05%) |
| **Total Net R** | **+26.28R** | Cumulative edge across 168 trades |
| **Average Hold Duration** | 9.1 bars (~36.4 hours) | Swift execution; capital is freed up quickly |
| **Market Exposure** | ~6.0% of all time | Sits in cash 94% of the time, avoiding catastrophic market crashes |

---

## 3. $100 Capital Backtest Performance

Tested using fractional compounding with fixed 1% risk per trade ($r = 0.010$, 1R = 1% of account equity):

| Balance Metric | Value on $100 Account |
| :--- | :--- |
| **Starting Balance** | **$100.00** |
| **Ending Balance** | **$128.25** |
| **Total Net Profit** | **+$28.25 (+28.25%)** |
| **Maximum Drawdown ($)** | **$11.75** |
| **Maximum Drawdown (%)** | **8.96%** |
| **Maximum Drawdown (R)** | **9.35R** |
| **Largest Winning Trade** | **+$6.64 (+5.84R)** |
| **Largest Losing Trade** | **-$1.42 (-1.10R)** |

---

## 4. Time-Based Earnings Breakdown ($100 Base)

| Time Frame | Profit ($) | Return (%) | Context |
| :--- | :--- | :--- | :--- |
| **Yearly Average** | **+$4.95 / year** | **+4.95% / yr** | Steady long-term trend capture |
| **Monthly Average** | **+$0.41 / month** | **+0.41% / mo** | Averaged across all 68.4 months |
| **Weekly Average** | **+$0.09 / week** | **+0.09% / wk** | Averaged across 297.6 weeks |
| **Daily Average** | **+$0.014 / day** | **+0.014% / day** | ~1.4 cents per day on $100 |

*Note on lumpiness:* Trend following returns are non-linear. The strategy can experience 2–3 months of small chop cuts followed by a single breakout generating +5R to +15R in a few days.

---

## 5. Year-by-Year Calendar Breakdown

| Year | Trades | Wins | Losses | Win Rate | Net R | Net PnL ($) | Cumulative Capital | Market Regime |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **2021** | 35 | 8 | 27 | 22.9% | -3.65R | -$3.71 | $96.29 | Heavy blow-off top whipsaws |
| **2022** | 11 | 2 | 9 | 18.2% | -2.06R | -$1.98 | $94.31 | Bear market crash (preserved 94% capital) |
| **2023** | 24 | 13 | 11 | **54.2%** | **+16.80R** | **+$16.82** | $111.13 | Clean early bull breakout trends |
| **2024** | 33 | 10 | 23 | 30.3% | +5.04R | +$5.46 | $116.59 | ETF consolidation & breakouts |
| **2025** | 36 | 16 | 20 | **44.4%** | **+11.05R** | **+$13.18** | $129.77 | Institutional trend expansion |
| **2026** | 29 | 6 | 23 | 20.7% | -0.90R | -$1.53 | $128.25 | Late-cycle rangebound chop |

---

## 6. Asset Breakdown

Both Bitcoin and Ethereum contributed virtually identical, balanced positive expectancy:

| Asset | Trades | Wins | Losses | Win Rate | Total Net R | Avg Net R / Trade |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`BTCUSDT`** | 88 | 27 | 61 | 30.7% | **+12.99R** | +0.148R |
| **`ETHUSDT`** | 80 | 28 | 52 | 35.0% | **+13.29R** | +0.166R |

---

## 7. Exit Trigger Analysis

The 4-stage exit engine safely cuts losers early and lets winners run:

| Exit Rule | Trades | Win Count | Total Net R | Avg R / Trade | Functional Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`BreakevenStopHit`** | 54 | 44 | **+78.74R** | **+1.46R** | Ratchets stop to entry after +1.0R gain, protecting profits |
| **`StagnationTimeStop`** | 52 | 8 | -10.01R | -0.19R | Kills trade if price stalls for 6 bars (24h) without progress |
| **`TrailingStopHit`** | 45 | 3 | -24.22R | -0.54R | Chandelier trail (2.5x ATR) locking gains or small losses |
| **`StopLossHit`** | 17 | 0 | -18.22R | -1.07R | Hard disaster stop (only 10.1% of trades ever hit full stop) |

---

## 8. Quantitative Strengths & Weaknesses

### Strengths
1. **Low Full-Stop Rate:** Only 17 out of 168 trades (10.1%) hit the full catastrophic stop loss. The stagnation timer and breakeven ratchets cut 90% of trades before a full loss occurs.
2. **Zero Liquidation Risk:** Runs exclusively on spot crypto. No funding rate costs, no margin calls, no exchange liquidation engine.
3. **Severe Bear Market Resilience:** In 2022, when buy-and-hold BTC/ETH crashed -75%, this strategy lost less than -$2.00 (-2.0%) because the Kaufman filter rejected choppy downtrends.

### Weaknesses
1. **Chop Drag in Transition Regimes:** Protracted consolidation ranges (like 2021 mid-year chop) cause clusters of small 0.2R to 0.5R friction losses.
2. **Low Trade Frequency:** With only ~30 trades a year, it requires extreme patience and discipline.
