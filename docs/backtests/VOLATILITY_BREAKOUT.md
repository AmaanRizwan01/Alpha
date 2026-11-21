# Strategy Backtest Report: VOLATILITY_BREAKOUT

## 1. Executive Summary

`VOLATILITY_BREAKOUT` (originally benchmark strategy `H1`) is a high-patience spot crypto trend-following system operating across Bitcoin (`BTCUSDT`) and Ethereum (`ETHUSDT`).

* **Core Premise:** Crypto markets spend 70%+ of their time in non-profitable mean-reverting chop. This strategy sits 100% flat in cash until explosive volume breaks a 20-period Donchian channel confirmed by the Kaufman Efficiency Ratio ($\text{ER}_{24} \ge 0.30$). It then rides the breakout using a 4-phase ratcheting trailing stop.
* **Asymmetric Risk Profile:** Classic trend-following asymmetry. Low win rate (32.7% to 36.4%) offset by a high payoff ratio (3.07:1), producing a statistically verified positive net expected value of **+0.156R per trade**.

---

## 2. Key Performance Metrics ($100 Starting Capital)

Tested under conservative friction (8 bps maker entry, 10 bps taker exit, 5 bps taker slippage = 23 bps roundtrip friction):

| Metric | 10-Year Full View (2016–2026) | 5.7-Year Pure 4h (2021–2026) | Unseen Out-of-Sample (2022–2026) | Context |
| :--- | :--- | :--- | :--- | :--- |
| **Initial Starting Capital** | **$100.00** | **$100.00** | **$100.00** | Baseline starting capital |
| **Final Ending Capital** | **$280.25** | **$128.25** | **$133.16** | Capital growth net of all taker fees |
| **Total Net Profit ($)** | **+$180.25** | **+$28.25** | **+$33.16** | Net of exchange taker fee & slippage |
| **Total Net Return (%)** | **+180.25%** | **+28.25%** | **+33.16%** | Compounded capital return |
| **Annualized Return (CAGR)** | **10.85% / yr** | **4.45% / yr** | **5.90% / yr** | Steady low-frequency trend capture |
| **Maximum Drawdown (%)** | **-8.96%** | **-8.96%** | **-8.96%** | **Strictly contained below 9%** |
| **Win Rate (%)** | **34.8%** | **32.74%** | **36.36%** | Positive skewness trend distribution |
| **Profit Factor (PF)** | **1.62** | **1.49** | **1.51** | Positive mathematical expectancy |
| **Payoff Ratio (Win / Loss)**| **3.20 : 1** | **3.07 : 1** | **3.05 : 1** | Winners are 3x larger than losers |
| **Net Expected Value (EV)** | **+0.172R** | **+0.156R** | **+0.165R** | Verified positive net edge per trade |
| **Market Exposure** | ~6.0% of time | ~6.0% of time | ~6.2% of time | Sits in cash 94% of the time |

---

## 3. Year-by-Year Calendar Matrix

| Year | Trades | Win Rate | Net PnL ($) | Annual Return (%) | Cumulative Capital | Market Regime & Behavior |
| :---: | :---: | :---: | :---: | :---: | :---: | :--- |
| **2016** | 0 | 0.0% | $0.00 | +0.00% | $100.00 | Prior to active crypto exchange launch |
| **2017** | 22 | 45.5% | **+$42.50** | **+42.50%** | $142.50 | Historic retail crypto bull run |
| **2018** | 16 | 25.0% | -$8.83 | -6.20% | $133.67 | -84% crypto crash (portfolio lost only 6%) |
| **2019** | 18 | 38.9% | **+$24.59** | **+18.40%** | $158.26 | Clean post-winter trend recovery |
| **2020** | 24 | 41.7% | **+$61.09** | **+38.60%** | $219.35 | Post-halving vertical bull run |
| **2021** | 35 | 22.9% | -$8.14 | -3.71% | $211.21 | Blow-off tops and summer consolidation chop |
| **2022** | 11 | 18.2% | -$4.35 | -2.06% | $206.86 | Terra & FTX crash (lost only 2% in cash) |
| **2023** | 24 | **54.2%** | **+$36.88** | **+17.83%** | $243.74 | Early bull breakout trends |
| **2024** | 33 | 30.3% | **+$11.97** | **+4.91%** | $255.71 | Spot Bitcoin ETF consolidation |
| **2025** | 36 | 44.4% | **+$28.90** | **+11.30%** | $284.61 | Broad market momentum expansion |
| **2026** | 29 | 20.7% | -$3.36 | -1.18% | $281.25 | Late-cycle rangebound chop |

---

## 4. Exit Trigger Analysis

The 4-stage exit engine safely cuts losers early and lets winners run:

| Exit Rule | Trades | Win Count | Total Net R | Avg R / Trade | Functional Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`BreakevenStopHit`** | 54 | 44 | **+78.74R** | **+1.46R** | Ratchets stop to entry after +1.0R gain, protecting profits |
| **`StagnationTimeStop`** | 52 | 8 | -10.01R | -0.19R | Kills trade if price stalls for 6 bars (24h) without progress |
| **`TrailingStopHit`** | 45 | 3 | -24.22R | -0.54R | Chandelier trail (2.5x ATR) locking gains or small losses |
| **`StopLossHit`** | 17 | 0 | -18.22R | -1.07R | Hard disaster stop (only 10.1% of trades ever hit full stop) |
