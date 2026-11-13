# TradingView Integration Guide: Unified Multi-Strategy Engine

## Overview

Project Alpha provides a single, unified TradingView script ([`tradingview/alpha_portfolio.pine`](../tradingview/alpha_portfolio.pine)) compatible with all three production portfolio strategies:
1. **`VOLATILITY_BREAKOUT`** (`alphabreakout`): 4h Donchian Breakout + Kaufman Efficiency Ratio (ER) Anti-Chop Gate.
2. **`FUNDING_CARRY`** (`alphacarry`): 8h Delta-Neutral Spot-Perp Basis & Funding Rate Harvest.
3. **`MACRO_TREND`** (`alphamacro`): 1D Multi-Asset Momentum & Trend Following (SPY, QQQ, PAXG, BTC).

Rather than managing three separate indicators or complex Pine scripts, this simplified script lets you toggle between all three strategies using a single dropdown.

---

## 1. Installation in TradingView (30-Second Setup)

1. Open any chart on [TradingView](https://www.tradingview.com) (e.g. `BTCUSDT` on 4h or 1D).
2. Open the **Pine Editor** tab at the bottom of the screen.
3. Click **Open** -> **New blank indicator**.
4. Copy and paste the entire contents of [`tradingview/alpha_portfolio.pine`](../tradingview/alpha_portfolio.pine).
5. Click **Save** and **Add to chart**.

---

## 2. Strategy Switching & Settings

In the indicator settings gear icon:
- **Active Strategy**: Select `VOLATILITY_BREAKOUT`, `FUNDING_CARRY`, or `MACRO_TREND`.
- The on-screen HUD (top right) instantly updates to show:
  - Active Strategy Mode
  - Mapped OKX Sub-Account (`alphabreakout`, `alphacarry`, `alphamacro`)
  - Real-time Kaufman ER anti-chop status (`PASS` or `CHOP`)
  - ATR volatility status (`PASS` or `LOW`)
  - Regime baseline status (`BULLISH (PASS)` or `BEARISH (ABSTAIN)`)

---

## 3. Webhook Alert Format

The script includes unified alert conditions:
- **Alpha Long Entry Alert**: Triggers when all 3 gates pass and breakout occurs.
- **Alpha Stop/Exit Alert**: Triggers when the ATR trailing stop or inversion stop is touched.

### Sample Alert JSON Payload:
```json
{
  "sub_account": "alphabreakout",
  "strategy": "VOLATILITY_BREAKOUT",
  "action": "BUY",
  "ticker": "{{ticker}}",
  "price": {{close}}
}
```
This payload directly references the target OKX sub-account for isolated order execution.
