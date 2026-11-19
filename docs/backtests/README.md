# Alpha: Strategy Lab & Backtesting Framework

This directory contains the testing framework, fee evaluation models, and the public Strategy Cemetery.

---

## 1. How Strategies Are Evaluated

Every strategy in Alpha is stress-tested against realistic market friction:
* **Exchange Taker Fees:** 0.05% - 0.10% per fill.
* **Slippage & Spread:** 0.02% - 0.05% against the top of the order book.
* **Funding & Carrying Cost:** Multi-year interest and borrow fees.
* **Execution Gaps:** Realistic simulation of gap-downs on stop-loss orders.

---

## 2. The Strategy Lifecycle (Public Cemetery vs Private Vault)

```text
Draft Strategy (from YouTube or text prompt)
       │
       ▼
10-Year Historical Stress Test
       │
   ┌───┴───┐
   ▼       ▼
[FAIL]   [PASS]
   │       │
   │       └─► Stored in python/research/strategies/passed/
   │           Backtest report saved to docs/backtests/passed/
   │           (100% PRIVATE & GITIGNORED by default)
   │
   └─────────► Moved to python/research/strategies/failed/
               Autopsy logged to docs/backtests/KILLED_STRATEGIES.md
               (PUBLIC on GitHub for community learning)
```

---

## 3. Strategy Failure Modes

See [`KILLED_STRATEGIES.md`](./KILLED_STRATEGIES.md) for post-mortems of popular retail strategies that failed 10-year quantitative gates, along with exact reasons why they lost money (fee drag, liquidation risk, or statistical breakdown).
