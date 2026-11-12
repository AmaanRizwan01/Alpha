# Strategy Backtest Report: MACRO_TREND

## 1. Executive Summary

`MACRO_TREND` is a daily cross-asset trend-following momentum strategy designed to capture structural capital flows across global financial markets.

* **Core Premise:** Financial capital rotates between asset classes based on macro liquidity, interest rates, and inflation regimes. By trading liquid ETFs representing Equities, Tech, Gold, Government Bonds, and Bitcoin, the strategy automatically rides the winning macro wave while exiting underperforming assets.
* **Macro Risk Filter:** Integrates an automated volatility filter (VIX Circuit Breaker). When market-wide systemic fear spikes (VIX > 30), new long entries are immediately suppressed, shielding capital from macro market crashes.

---

## 2. Key Performance Metrics

| Metric | Empirical Value | Description |
| :--- | :--- | :--- |
| **Asset Universe** | `SPY`, `QQQ`, `GLD`, `TLT`, `BTC` | US Equities, Tech, Gold, 20+ Year Treasuries, Bitcoin |
| **Bar Timeframe** | Daily (`1d`) | End-of-day execution; zero intraday noise or HFT slippage |
| **Test Period** | 2022-12-01 to 2026-10-30 | 1,428 calendar days (3.91 years) |
| **Total Trades** | 216 | ~55 trades per year across all 5 assets |
| **Win Rate** | **45.83%** (99 wins) | 54.17% losses (117 losses) |
| **Payoff Ratio (Win / Loss)** | **2.29** | Average win (+1.16R) is more than double the average loss (-0.51R) |
| **Profit Factor (PF)** | **1.94** | Strong institutional profitability ratio |
| **Net Expected Value (EV)** | **+0.2570R** / trade | Net of ETF broker commissions and slippage |
| **Total Net R** | **+55.52R** | Cumulative edge across 216 trades |
| **Average Hold Duration** | 13.1 bars (~13.1 trading days) | Multi-week momentum hold duration |
| **Market Exposure** | ~45.5% of all time | Capital is deployed selectively when clear trends exist |

---

## 3. $100 Capital Backtest Performance

Tested using fractional compounding with fixed 1% risk per trade ($r = 0.010$, 1R = 1% of equity):

| Balance Metric | Value on $100 Account |
| :--- | :--- |
| **Starting Balance** | **$100.00** |
| **Ending Balance** | **$171.65** |
| **Total Net Profit** | **+$71.65 (+71.65%)** |
| **Maximum Drawdown ($)** | **$8.62** |
| **Maximum Drawdown (%)** | **5.36%** |
| **Maximum Drawdown (R)** | **5.48R** |
| **Largest Winning Trade** | **+$10.14 (+6.05R on Gold)** |
| **Largest Losing Trade** | **-$1.77 (-1.20R on Treasuries)** |

---

## 4. Time-Based Earnings Breakdown ($100 Base)

| Time Frame | Profit ($) | Return (%) | Context |
| :--- | :--- | :--- | :--- |
| **Yearly Average** | **+$18.33 / year** | **+18.33% / yr** | High-consistency multi-asset momentum |
| **Monthly Average** | **+$1.53 / month** | **+1.53% / mo** | Averaged across 46.9 calendar months |
| **Weekly Average** | **+$0.35 / week** | **+0.35% / wk** | Averaged across 204.0 calendar weeks |
| **Daily Average** | **+$0.050 / day** | **+0.050% / day** | ~5.0 cents per day on $100 |

---

## 5. Year-by-Year Calendar Breakdown

| Year | Trades | Wins | Losses | Win Rate | Net R | Net PnL ($) | Cumulative Capital | Macro Context |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **2022 (Dec)** | 4 | 2 | 2 | 50.0% | +0.49R | +$0.47 | $100.47 | Early test sample initialization |
| **2023** | 57 | 22 | 35 | 38.6% | +7.83R | +$7.84 | $108.31 | Tech rebound lead by AI & mega-caps |
| **2024** | 72 | 37 | 35 | **51.4%** | **+20.99R** | **+$24.69** | $133.00 | Broad equity expansion and Gold breakout |
| **2025** | 48 | 25 | 23 | **52.1%** | **+30.27R** | **+$45.91** | $178.91 | Massive commodity & Gold supercycle |
| **2026** | 35 | 13 | 22 | 37.1% | -4.06R | -$7.28 | $171.65 | Late-stage macro rotation & rate adjustments |

---

## 6. Asset-by-Asset Contribution Analysis

The power of macro diversification is evident in asset-level returns:

| Asset | Category | Trades | Wins | Losses | Win Rate | Total Net R | Avg Net R / Trade |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`GLD`** | Physical Gold | 48 | 28 | 20 | **58.3%** | **+31.56R** | **+0.658R** (Top Performer) |
| **`QQQ`** | Nasdaq 100 | 51 | 26 | 25 | **51.0%** | **+13.28R** | +0.260R |
| **`SPY`** | S&P 500 | 52 | 24 | 28 | 46.2% | **+11.82R** | +0.227R |
| **`BTC`** | Bitcoin Macro | 44 | 17 | 27 | 38.6% | **+2.62R** | +0.060R |
| **`TLT`** | 20Y+ Treasuries | 21 | 4 | 17 | 19.0% | -3.77R | -0.179R (Underperformer) |

### Key Insights
1. **Gold (`GLD`) Was the Engine:** Generated over 56% of total strategy profits (+31.56R) due to a persistent secular monetary hedge trend.
2. **Treasuries (`TLT`) Dragged:** Aggressive Federal Reserve rate hikes caused long-term bonds to suffer persistent drawdowns. However, because each trade was capped at 1R risk, total damage was constrained to -3.77R over nearly 4 years.
3. **Crypto Smoothness:** Bitcoin trades on daily bars avoided intraday stop-outs, contributing +2.62R net.

---

## 7. Exit Trigger Analysis

| Exit Rule | Trades | Win Count | Total Net R | Avg R / Trade | Functional Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`TrailingStop`** | 192 | 95 | **+60.62R** | **+0.316R** | 3.0x ATR Chandelier trailing stop securing major trend legs |
| **`TrendInvalidation`** | 20 | 2 | -7.36R | -0.368R | Moving average cross reversal cutting stale positions |
| **`VixCircuitBreaker`** | 2 | 2 | **+2.94R** | **+1.472R** | Forced emergency exit when market volatility breached safe thresholds |
| **`EndOfSample`** | 2 | 0 | -0.69R | -0.343R | Test sample boundary termination |

---

## 8. Strengths & Production Considerations

### Strengths
1. **Low Volatility & Small Drawdown:** Max drawdown was just 5.36% ($8.62 on $100), significantly lower than crypto-only systems.
2. **True Cross-Asset Non-Correlation:** Equities and Gold trends move on different macro cycles, smoothing the aggregate equity curve.
3. **Execution Simplicity:** 1-day bars require only 1 check per day at 00:00 UTC or US market close, minimizing server and API overhead.
