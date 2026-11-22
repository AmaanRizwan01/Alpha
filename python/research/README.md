# Alpha: Quantitative Research Laboratory (`python/research`)

> **The empirical research, simulation, and statistical validation laboratory of Alpha.**

---

## 1. Laboratory Purpose & Architecture

`python/research/` serves as Alpha's dedicated quantitative laboratory. It houses all data ingestion pipelines, walk-forward simulation engines, statistical gate validators, and the plug-and-play strategy research framework.

### Why `python/research/` and not a disconnected root `laboratory/`?
In modern quantitative architectures combining Rust engines with Python data research:
1. **Single Module Root:** Python packages, `pyproject.toml`, Maturin C-FFI bindings (`alpha_py`), and `pytest` discovery require a unified namespace (`from python.research...`).
2. **AST Static Analysis:** Our CI enforcement tests (`python/tests/test_multi_strategy.py`) parse the Python AST to guarantee zero cross-strategy imports and complete modular isolation.
3. **C-FFI Parity:** Pure Rust math in `alpha-core` is compiled via `alpha-py` directly into the `.venv`, allowing research scripts in `python/research/` to invoke exact compiled Rust logic with microsecond speed and zero drift between backtest and production execution.

---

## 2. Directory Structure & File Map

```
python/research/
├── README.md                      # This laboratory documentation and operating guide
├── strategies/                    # Plug-and-play quantitative strategies across evaluation tiers
│   ├── __init__.py                # Tiered strategy registry (load_all_strategies, get_available_strategies)
│   ├── base.py                    # BaseStrategy abstract specification & Trade / Performance schemas
│   ├── passed/                    # Production-ready active strategies (Private & gitignored)
│   ├── suboptimal/                # Suboptimal strategies (Passed gates, lower profit factor)
│   │   └── volatility_breakout/   # 4h Crypto Donchian + KER Breakout (Suboptimal)
│   └── failed/                    # Archived candidates for research & reproducibility (Public)
│       ├── README.md              # Quantitative autopsy & kill reasons summary
│       ├── session_imbalances/    # Asian/London/NY session sweeps (killed: fee drag)
│       ├── cross_sectional_momentum/ # Altcoin relative momentum basket (killed: rebalance drag)
│       ├── flush_reversal/        # Liquidation cascade mean reversion (killed: N<100, tail risk)
│       └── cointegrated_pairs/    # ETH/BTC stat-arb spread (killed: N<100, non-stationary)
├── lab/                           # Strategy Lab (YouTube rule extractor & 10-year fee tester)
│   ├── cli.py                     # CLI entrypoint (extract, test, cemetery commands)
│   ├── extractor.py               # YouTube transcript parser & prompt-to-rule converter
│   └── tester.py                  # Automated 10-year purged walk-forward fee backtester
├── primitives/                    # Multi-timeframe vectorized data primitives
│   └── multi_timeframe.py         # Polars multi-timeframe resampler & alignment
├── download_history.py            # Historical spot klines downloader (data.binance.vision)
├── download_funding_oi.py         # Historical funding rate & open interest downloader
├── download_macro.py              # Macro ETF & volatility index historical downloader
├── download_oi_metrics.py         # Hourly open interest downsampler & cleaner
├── walkforward.py                 # Purged walk-forward engine (rolling train/test + 48h purge)
├── run_walkforward.py             # Walk-forward CLI runner for Volatility Breakout ($H_1$)
├── validate_gates.py              # Statistical kill gates (10,000-resample bootstrap, DSR, holdout)
└── compare_strategies.py          # Master multi-strategy backtester & correlation matrix generator
```

---

## 3. The 5-Stage Quantitative Research Pipeline

Every strategy candidate investigated in the laboratory must strictly traverse five verification stages before being considered for live execution:

```
[1. Economic Hypothesis & Mathematical Specification]
                          │
                          ▼
[2. Free Historical Data Ingestion & Sanitization]
    (download_history.py, download_funding_oi.py, download_macro.py)
                          │
                          ▼
[3. Modular Strategy Implementation (strategies/)]
    (Subclasses BaseStrategy, implements compute_signals & simulate_trades)
                          │
                          ▼
[4. Purged Walk-Forward Backtesting & Fee-Drag Proof]
    (Fee-to-stop ratio: Fee_R = C_rt / D_stop; strictly verified on 4h / 1D)
                          │
                          ▼
[5. Statistical Kill Gates & Master Comparison]
    (Stationary block bootstrap LCB_95 > +0.02R, DSR > 0.95, Stage B Holdout)
```

---

## 4. Plug-and-Play Strategy Development

To add a new experimental strategy to the laboratory:
1. Create a new file in `python/research/strategies/<strategy_slug>.py`.
2. Inherit from `BaseStrategy` defined in [`python/research/strategies/base.py`](file:///c:/Coding/alpha/python/research/strategies/base.py).
3. Implement `run_backtest(self, data_dir: Path) -> StrategyPerformance`.
4. Ensure zero external dependencies outside `polars`, `numpy`, and `alpha_py`.
5. Run the master comparator:
   ```bash
   python python/research/compare_strategies.py
   ```
6. If the strategy fails quantitative gates, document the failure in [`docs/backtests/KILLED_STRATEGIES.md`](file:///c:/Coding/alpha/docs/backtests/KILLED_STRATEGIES.md) and delete the file.

---

## 5. Public Suboptimal Reference Results

| Strategy ID | Class / File | Asset Universe | Horizon | Net EV (R) | Profit Factor | Status |
|---|---|---|---|---|---|---|
| **VOLATILITY_BREAKOUT** | `VolatilityBreakoutStrategy` | `BTC/USDT`, `ETH/USDT` | 4h | **+0.156R** | 1.49 | **PASS (Suboptimal)** |

For detailed financial breakdowns ($100 capital trajectory, win rates, yearly/monthly metrics, loss attribution), see [`docs/backtests/`](file:///c:/Coding/alpha/docs/backtests/).
