"""Strategy Candidate: Statistical Cointegration & Pairs Trading (COINTEGRATED_PAIRS).

KILLED CANDIDATE: Failed Quantitative Kill Gate.
Reason: Insufficient sample size (N=13 < 100) & non-stationary trend breakdown (-0.161R Net EV, 0.58 PF).
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


class CointegratedPairsStrategy(BaseStrategy):
    """ETH/BTC Statistical Cointegration Pairs Strategy (KILLED)."""

    def __init__(
        self,
        zscore_lookback: int = 180,        # 30 days on 4h bars
        entry_zscore: float = 2.2,         # 2.2 std dev spread
        exit_zscore: float = 0.5,          # Exit near mean
        round_trip_friction: float = 0.0020, # 2 legs round-trip
    ) -> None:
        self.lookback = zscore_lookback
        self.entry_z = entry_zscore
        self.exit_z = exit_zscore
        self.c_rt = round_trip_friction
        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "COINTEGRATED_PAIRS"

    @property
    def strategy_name(self) -> str:
        return "ETH/BTC Cointegrated Pairs (Failed Gate)"

    @property
    def category(self) -> str:
        return "Failed / Statistical Arbitrage"

    @property
    def primary_asset(self) -> str:
        return "ETH/BTC Synthetic Spread"

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
        btc_p = resolve_data_path(data_dir, "BTCUSDT_4h.parquet")
        eth_p = resolve_data_path(data_dir, "ETHUSDT_4h.parquet")
        if not btc_p.exists() or not eth_p.exists():
            return compute_performance_metrics(
                self.strategy_id, self.strategy_name, self.category,
                self.primary_asset, self.timeframe, [], 0, 4.0
            )

        btc_df = pl.read_parquet(btc_p).sort("open_time")
        eth_df = pl.read_parquet(eth_p).sort("open_time")

        if "datetime" not in btc_df.columns:
            btc_df = btc_df.with_columns(datetime=pl.from_epoch(pl.col("open_time"), time_unit="ms"))
        if "datetime" not in eth_df.columns:
            eth_df = eth_df.with_columns(datetime=pl.from_epoch(pl.col("open_time"), time_unit="ms"))

        merged = btc_df.select(["datetime", "close"]).join(
            eth_df.select(["datetime", "close"]),
            on="datetime",
            suffix="_eth"
        ).sort("datetime")

        times = merged["datetime"].to_list()
        btc_c = merged["close"].to_numpy()
        eth_c = merged["close_eth"].to_numpy()
        n = len(merged)

        ratio = eth_c / btc_c
        trades: list[Trade] = []
        d_stop = 0.04

        in_pos = False
        entry_ratio = 0.0
        entry_idx = 0
        stop_ratio = 0.0

        for i in range(self.lookback, n - 1):
            window = ratio[i - self.lookback : i]
            mean = np.mean(window)
            std = np.std(window)
            if std <= 1e-8:
                continue

            z = (ratio[i] - mean) / std

            if not in_pos:
                if z <= -self.entry_z:
                    in_pos = True
                    entry_ratio = ratio[i]
                    entry_idx = i
                    stop_ratio = entry_ratio * (1.0 - d_stop)
            else:
                bars_held = i - entry_idx
                exit_now = False
                exit_ratio = ratio[i]
                reason = ""

                if ratio[i] <= stop_ratio:
                    exit_now = True
                    exit_ratio = stop_ratio
                    reason = "SpreadDivergenceStop"
                elif z >= -self.exit_z:
                    exit_now = True
                    exit_ratio = ratio[i]
                    reason = "MeanReversionExit"
                elif bars_held >= 120:
                    exit_now = True
                    exit_ratio = ratio[i]
                    reason = "TimeStop"

                if exit_now:
                    ret = (exit_ratio - entry_ratio) / entry_ratio
                    gross_r = ret / d_stop
                    fee_r = self.c_rt / d_stop
                    net_r = gross_r - fee_r

                    trades.append(
                        Trade(
                            strategy_id=self.strategy_id,
                            asset="ETH/BTC",
                            timeframe=self.timeframe,
                            entry_time=times[entry_idx],
                            exit_time=times[i],
                            entry_price=entry_ratio,
                            exit_price=exit_ratio,
                            initial_stop=stop_ratio,
                            exit_reason=reason,
                            bars_held=bars_held,
                            gross_pnl_r=gross_r,
                            fee_r=fee_r,
                            net_pnl_r=net_r,
                            gross_return_pct=ret * 100.0,
                            net_return_pct=(ret - self.c_rt) * 100.0,
                            side="LONG_SPREAD",
                        )
                    )
                    in_pos = False

        self._trades = trades
        total_days = (times[-1] - times[0]).days if times else 0
        return compute_performance_metrics(
            self.strategy_id, self.strategy_name, self.category,
            self.primary_asset, self.timeframe, trades, total_days, 4.0
        )
