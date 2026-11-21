"""Strategy Candidate: Cross-Sectional Crypto Momentum & Dispersion (CROSS_SECTIONAL_MOMENTUM).

KILLED CANDIDATE: Failed Quantitative Kill Gate.
Reason: High rebalancing turnover & cross-asset beta crashes (-0.142R Net EV, 0.74 PF).
Maintained in failed/ archive for full research reproducibility.
"""

from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional
import numpy as np
import polars as pl

from python.research.data_utils import resolve_data_path
from python.research.strategies.base import (
    BaseStrategy,
    StrategyPerformance,
    Trade,
    compute_performance_metrics,
)


class CrossSectionalMomentumStrategy(BaseStrategy):
    """Cross-Sectional Altcoin Momentum Strategy (KILLED)."""

    def __init__(
        self,
        lookback_bars: int = 42,           # 7 days on 4h bars
        rebalance_interval: int = 42,      # Weekly rebalance
        round_trip_friction: float = 0.0020, # 20 bps turnover
    ) -> None:
        self.lookback = lookback_bars
        self.rebalance_interval = rebalance_interval
        self.c_rt = round_trip_friction
        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "CROSS_SECTIONAL_MOMENTUM"

    @property
    def strategy_name(self) -> str:
        return "Cross-Sectional Altcoin Momentum (Failed Gate)"

    @property
    def category(self) -> str:
        return "Failed / Cross-Sectional"

    @property
    def primary_asset(self) -> str:
        return "Top 10 Altcoin Basket"

    @property
    def timeframe(self) -> str:
        return "4h"

    def get_trades(self) -> list[Trade]:
        return self._trades

    def get_equity_series(self) -> pl.DataFrame:
        if self._equity_df is not None:
            return self._equity_df
        if not self._trades:
            return pl.DataFrame({"datetime": [], "cumulative_net_r": []})
        sorted_trades = sorted(self._trades, key=lambda t: t.exit_time)
        times = [t.exit_time for t in sorted_trades]
        cum_r = np.cumsum([t.net_pnl_r for t in sorted_trades])
        self._equity_df = pl.DataFrame({"datetime": times, "cumulative_net_r": cum_r})
        return self._equity_df

    def run_backtest(self, data_dir: Path) -> StrategyPerformance:
        assets = ["ETHUSDT", "SOLUSDT", "BNBUSDT", "ADAUSDT", "AVAXUSDT", "LINKUSDT", "DOGEUSDT"]
        dfs: Dict[str, pl.DataFrame] = {}

        for a in assets:
            p = resolve_data_path(data_dir, f"{a}_4h.parquet")
            if p.exists():
                df = pl.read_parquet(p).sort("open_time")
                if "datetime" not in df.columns:
                    df = df.with_columns(datetime=pl.from_epoch(pl.col("open_time"), time_unit="ms"))
                dfs[a] = df

        if len(dfs) < 3:
            return compute_performance_metrics(
                self.strategy_id, self.strategy_name, self.category,
                self.primary_asset, self.timeframe, [], 0, 4.0
            )

        common_times = set(dfs[next(iter(dfs))]["datetime"].to_list())
        for a in dfs:
            common_times = common_times.intersection(set(dfs[a]["datetime"].to_list()))
        times = sorted(list(common_times))
        n = len(times)

        trades: list[Trade] = []
        d_stop = 0.05

        for i in range(self.lookback, n - self.rebalance_interval, self.rebalance_interval):
            rets = {}
            for a in dfs:
                sub = dfs[a].filter(pl.col("datetime").is_in(times[i - self.lookback : i + 1])).sort("datetime")
                if len(sub) > self.lookback:
                    c = sub["close"].to_numpy()
                    rets[a] = (c[-1] - c[0]) / c[0]

            if not rets:
                continue

            sorted_assets = sorted(rets.items(), key=lambda x: x[1], reverse=True)
            top_asset = sorted_assets[0][0]

            t_entry = times[i]
            t_exit = times[i + self.rebalance_interval]

            sub_top = dfs[top_asset].filter(pl.col("datetime").is_in(times[i : i + self.rebalance_interval + 1])).sort("datetime")
            p_in_top = sub_top["close"][0]
            p_out_top = sub_top["close"][-1]
            ret_top = (p_out_top - p_in_top) / p_in_top

            gross_r = ret_top / d_stop
            fee_r = self.c_rt / d_stop
            net_r = gross_r - fee_r

            trades.append(
                Trade(
                    strategy_id=self.strategy_id,
                    asset=top_asset,
                    timeframe=self.timeframe,
                    entry_time=t_entry,
                    exit_time=t_exit,
                    entry_price=p_in_top,
                    exit_price=p_out_top,
                    initial_stop=p_in_top * (1.0 - d_stop),
                    exit_reason="RebalanceRotate",
                    bars_held=self.rebalance_interval,
                    gross_pnl_r=gross_r,
                    fee_r=fee_r,
                    net_pnl_r=net_r,
                    gross_return_pct=ret_top * 100.0,
                    net_return_pct=(ret_top - self.c_rt) * 100.0,
                    side="LONG",
                )
            )

        self._trades = trades
        total_days = (times[-1] - times[0]).days if times else 0
        return compute_performance_metrics(
            self.strategy_id, self.strategy_name, self.category,
            self.primary_asset, self.timeframe, trades, total_days, 4.0
        )
