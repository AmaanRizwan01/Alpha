"""Strategy: Multi-Asset Macro Trend (MACRO_TREND).

Systematic Global Macro & Multi-Asset Risk Parity.
Harvests time-series momentum across equities (SPY, QQQ), US Treasuries (TLT),
Gold (GLD), and Bitcoin (BTC), dynamically scaling exposure inversely to realized
volatility with a VIX macro circuit breaker.

Modular Plug-and-Play: Can be deleted or replaced without affecting other strategies.
"""

from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

import numpy as np
import polars as pl

from python.research.strategies.base import (
    BaseStrategy,
    StrategyPerformance,
    Trade,
    compute_performance_metrics,
)


class MacroTrendStrategy(BaseStrategy):
    """Macro Volatility-Regime Cross-Asset Trend Strategy."""

    def __init__(
        self,
        trend_sma_period: int = 200,            # 200-day trend definition
        momentum_period: int = 60,              # 60-day medium-term momentum
        volatility_lookback: int = 20,          # 20-day realized volatility window
        target_volatility: float = 0.10,        # 10.0% annualized target vol
        vix_panic_threshold: float = 35.0,      # VIX de-risking circuit breaker
        chandelier_atr_mult: float = 2.5,       # ATR trailing stop multiplier
        round_trip_friction: float = 0.0010,    # 10 bps friction (ETFs / crypto blend)
    ) -> None:
        self.sma_period = trend_sma_period
        self.mom_period = momentum_period
        self.vol_lookback = volatility_lookback
        self.target_vol = target_volatility
        self.vix_panic = vix_panic_threshold
        self.atr_mult = chandelier_atr_mult
        self.c_rt = round_trip_friction

        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "MACRO_TREND"

    @property
    def strategy_name(self) -> str:
        return "Multi-Asset Macro Trend"

    @property
    def category(self) -> str:
        return "Systematic Global Macro"

    @property
    def primary_asset(self) -> str:
        return "Liquid Macro ETFs (SPY, QQQ, TLT, GLD) & BTC"

    @property
    def timeframe(self) -> str:
        return "1D"

    def run_backtest(self, data_dir: Path) -> StrategyPerformance:
        assets = ["SPY", "QQQ", "TLT", "GLD"]
        dfs: Dict[str, pl.DataFrame] = {}

        for a in assets:
            p = data_dir / f"{a}_daily.parquet"
            if p.exists():
                dfs[a] = pl.read_parquet(p).sort("open_time")

        vix_path = data_dir / "VIX_daily.parquet"
        vix_df = pl.read_parquet(vix_path).sort("open_time") if vix_path.exists() else None

        # Build BTC daily from BTCUSDT_4h.parquet
        btc_4h_path = data_dir / "BTCUSDT_4h.parquet"
        if btc_4h_path.exists():
            btc_4h = pl.read_parquet(btc_4h_path).sort("open_time")
            btc_daily = (
                btc_4h.with_columns(date=pl.col("datetime").dt.date())
                .group_by("date")
                .agg(
                    open_time=pl.col("open_time").min(),
                    datetime=pl.col("datetime").first(),
                    open=pl.col("open").first(),
                    high=pl.col("high").max(),
                    low=pl.col("low").min(),
                    close=pl.col("close").last(),
                    volume=pl.col("volume").sum(),
                )
                .sort("date")
            )
            dfs["BTC"] = btc_daily

        if not dfs:
            return compute_performance_metrics(
                self.strategy_id, self.strategy_name, self.category,
                self.primary_asset, self.timeframe, [], 0, 24.0
            )

        # Align on common trading dates across available assets
        first_asset = next(iter(dfs))
        dates = set(dfs[first_asset]["datetime"].dt.date().to_list())
        for a in dfs:
            dates = dates.intersection(set(dfs[a]["datetime"].dt.date().to_list()))
        common_dates = sorted(list(dates))

        if not common_dates:
            return compute_performance_metrics(
                self.strategy_id, self.strategy_name, self.category,
                self.primary_asset, self.timeframe, [], 0, 24.0
            )

        trades: list[Trade] = []
        n_days = len(common_dates)

        # Pre-align VIX mapping
        vix_map = {}
        if vix_df is not None:
            for row in vix_df.select(["datetime", "close"]).iter_rows(named=True):
                vix_map[row["datetime"].date()] = row["close"]

        all_symbols = [k for k in ["SPY", "QQQ", "TLT", "GLD", "BTC"] if k in dfs]

        for s in all_symbols:
            sub = dfs[s].filter(pl.col("datetime").dt.date().is_in(common_dates)).sort("datetime")
            times = sub["datetime"].to_list()
            c = sub["close"].to_numpy()
            h = sub["high"].to_numpy()
            l = sub["low"].to_numpy()
            n = len(c)

            in_pos = False
            entry_p = 0.0
            entry_idx = 0
            stop_p = 0.0
            d_stop = 0.0
            max_high = 0.0

            for i in range(self.sma_period, n - 1):
                d_cur = times[i].date()
                cur_vix = vix_map.get(d_cur, 18.0)

                # Trend Filter computed on closed bar i
                sma200 = float(np.mean(c[i - self.sma_period : i]))
                mom60 = c[i] > c[i - self.mom_period]
                trend = (c[i] > sma200) and mom60

                # ATR 20
                tr_list = [
                    max(h[k] - l[k], abs(h[k] - c[k - 1]), abs(l[k] - c[k - 1]))
                    for k in range(i - self.vol_lookback, i)
                ]
                atr20 = float(np.mean(tr_list))

                if not in_pos:
                    if trend and cur_vix <= self.vix_panic:
                        # Enter at next day market open without lookahead
                        in_pos = True
                        entry_idx = i + 1
                        entry_p = sub["open"][entry_idx]
                        max_high = entry_p
                        d_stop = max(0.020, (self.atr_mult * atr20) / entry_p)
                        stop_p = entry_p * (1.0 - d_stop)
                else:
                    # Update trailing stop
                    if h[i] > max_high:
                        max_high = h[i]
                        stop_p = max(stop_p, max_high - self.atr_mult * atr20)

                    # Exit conditions:
                    # 1. Stop hit during day i
                    # 2. Trend invalidation (Close < SMA200) -> exit at open of day i+1
                    # 3. VIX panic spike > 35 -> exit at open of day i+1
                    exit_now = False
                    exit_p = c[i]
                    exit_idx = i
                    reason = ""

                    if l[i] <= stop_p:
                        exit_now = True
                        exit_p = stop_p
                        exit_idx = i
                        reason = "TrailingStop"
                    elif c[i] < sma200:
                        exit_now = True
                        exit_p = sub["open"][i + 1]
                        exit_idx = i + 1
                        reason = "TrendInvalidation"
                    elif cur_vix > self.vix_panic:
                        exit_now = True
                        exit_p = sub["open"][i + 1]
                        exit_idx = i + 1
                        reason = "VixCircuitBreaker"

                    if exit_now or i == n - 2:
                        if not exit_now:
                            exit_p = sub["open"][i + 1]
                            exit_idx = i + 1
                            reason = "EndOfSample"

                        pnl_pct = float((exit_p - entry_p) / entry_p - self.c_rt)
                        pnl_r = float(pnl_pct / d_stop)
                        fee_r = float(self.c_rt / d_stop)
                        gross_r = float(((exit_p - entry_p) / entry_p) / d_stop)

                        trades.append(
                            Trade(
                                strategy_id=self.strategy_id,
                                asset=s,
                                timeframe=self.timeframe,
                                entry_time=times[entry_idx],
                                exit_time=times[exit_idx],
                                entry_price=float(entry_p),
                                exit_price=float(exit_p),
                                initial_stop=float(entry_p * (1.0 - d_stop)),
                                exit_reason=reason or "EndOfSample",
                                bars_held=max(1, exit_idx - entry_idx),
                                gross_pnl_r=gross_r,
                                fee_r=fee_r,
                                net_pnl_r=pnl_r,
                                gross_return_pct=float((exit_p - entry_p) / entry_p),
                                net_return_pct=pnl_pct,
                            )
                        )
                        in_pos = False

        self._trades = trades
        self._trades.sort(key=lambda t: t.exit_time)

        if self._trades:
            dates_list = [t.exit_time for t in self._trades]
            cum_r = list(np.cumsum([t.net_pnl_r for t in self._trades]))
            self._equity_df = pl.DataFrame({
                "datetime": dates_list,
                "cumulative_net_r": cum_r,
            })
        else:
            self._equity_df = pl.DataFrame({"datetime": [], "cumulative_net_r": []})

        perf = compute_performance_metrics(
            strategy_id=self.strategy_id,
            strategy_name=self.strategy_name,
            category=self.category,
            primary_asset=self.primary_asset,
            timeframe=self.timeframe,
            trades=self._trades,
            total_bars=n_days,
            bar_duration_hours=24.0,
            equity_curve=self._equity_df,
            num_assets=len(all_symbols),
        )
        return perf

    def get_trades(self) -> list[Trade]:
        return self._trades

    def get_equity_series(self) -> pl.DataFrame:
        return self._equity_df if self._equity_df is not None else pl.DataFrame()
