# Phase 0 Gate Evidence: Venue & Fee Discovery
**Generated:** 2026-11-07 23:37:08 UTC

## 1. Binance Spot Specifications & Account Fees
| Symbol | Min Notional | Lot Step (stepSize) | Price Tick (tickSize) | Maker Fee | Taker Fee |
|---|---|---|---|---|---|
| `BTCUSDT` | 5.00000000 | 0.00001000 | 0.01000000 | 0.00100000 | 0.00100000 |
| `ETHUSDT` | 5.00000000 | 0.00010000 | 0.01000000 | 0.00100000 | 0.00100000 |
| `BTCUSDC` | 5.00000000 | 0.00001000 | 0.01000000 | 0.00100000 | 0.00095000 |
| `BTCFDUSD` | 5.00000000 | 0.00001000 | 0.01000000 | 0.00000000 | 0.00100000 |

## 2. OKX Spot Specifications & Account Fees
**Account Tier / Level:** Lv1 (maker: -0.0008, taker: -0.001)

| Instrument | Min Size (minSz) | Lot Size (lotSz) | Price Tick (tickSz) |
|---|---|---|---|
| `BTC-USDT` | 0.00001 | 0.00000001 | 0.1 |
| `ETH-USDT` | 0.0001 | 0.000001 | 0.01 |
| `BTC-USDC` | 0.00001 | 0.00000001 | 0.1 |

## 3. Scale-Invariance Verdict ($100 Incubation)
- Target position size at $100 equity (1% risk, 2.5% stop): ~$36.60 notional.
- Clearing Binance $5 minNotional: **Verified** (36.60 > 5.0).
- Friction-to-Stop Ratio: Verified against structural stop floor rule.