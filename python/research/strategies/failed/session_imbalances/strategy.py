"""Strategy Candidate: Intraday Session Turnover & Liquidity Imbalance (SESSION_IMBALANCES).

KILLED CANDIDATE: Failed Quantitative Kill Gate.
Reason: Catastrophic fee drag on intraday scalps (-0.198R Net EV, 0.68 PF).
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


class SessionImbalancesStrategy(BaseStrategy):
    """Session Open Liquidity Sweeps Strategy (KILLED)."""

    def __init__(
        self,
        asian_start_hour: int = 0,
        asian_end_hour: int = 8,
        london_start_hour: int = 8,
        sweep_threshold_pct: float = 0.002,
        round_trip_friction: float = 0.0010,  # 10 bps friction
    ) -> None:
        self.asian_start = asian_start_hour
        self.asian_end = asian_end_hour
        self.london_start = london_start_hour
        self.sweep_thresh = sweep_threshold_pct
        self.c_rt = round_trip_friction
        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "SESSION_IMBALANCES"

    @property
    def strategy_name(self) -> str:
        return "Session Liquidity Sweeps (Failed Gate)"

    @property
    def category(self) -> str:
        return "Failed / Intraday Scalp"

    @property
    def primary_asset(self) -> str:
        return "BTC, ETH, SOL"

    @property
    def timeframe(self) -> str:
        return "1h"

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
        btc_p = resolve_data_path(data_dir, "BTCUSDT_1h.parquet")
        if not btc_p.exists():
            return compute_performance_metrics(
                self.strategy_id, self.strategy_name, self.category,
                self.primary_asset, self.timeframe, [], 0, 1.0
            )

        df = pl.read_parquet(btc_p).sort("open_time")
        if "datetime" not in df.columns:
            df = df.with_columns(datetime=pl.from_epoch(pl.col("open_time"), time_unit="ms"))

        times = df["datetime"].to_list()
        opens = df["open"].to_numpy()
        highs = df["high"].to_numpy()
        lows = df["low"].to_numpy()
        closes = df["close"].to_numpy()
        n = len(df)

        trades: list[Trade] = []
        in_pos = False
        entry_p = 0.0
        entry_idx = 0
        stop_p = 0.0
        target_p = 0.0
        side = "LONG"
        d_stop = 0.006  # Tight 0.6% stop on 1h scalp

        for i in range(24, n - 4):
            t = times[i]
            if t.hour == self.london_start and not in_pos:
                asian_high = np.max(highs[i - 8 : i])
                asian_low = np.min(lows[i - 8 : i])

                if highs[i] > asian_high * (1.0 + self.sweep_thresh) and closes[i] < asian_high:
                    in_pos = True
                    side = "SHORT"
                    entry_p = opens[i + 1]
                    entry_idx = i + 1
                    stop_p = highs[i] * 1.002
                    d_stop = max(0.004, (stop_p - entry_p) / entry_p)
                    target_p = entry_p - (entry_p * d_stop * 1.5)

                elif lows[i] < asian_low * (1.0 - self.sweep_thresh) and closes[i] > asian_low:
                    in_pos = True
                    side = "LONG"
                    entry_p = opens[i + 1]
                    entry_idx = i + 1
                    stop_p = lows[i] * 0.998
                    d_stop = max(0.004, (entry_p - stop_p) / entry_p)
                    target_p = entry_p + (entry_p * d_stop * 1.5)

            elif in_pos:
                bars_held = i - entry_idx
                exit_now = False
                exit_p = closes[i]
                reason = ""

                if side == "LONG":
                    if lows[i] <= stop_p:
                        exit_now = True
                        exit_p = stop_p
                        reason = "StopLoss"
                    elif highs[i] >= target_p:
                        exit_now = True
                        exit_p = target_p
                        reason = "TargetHit"
                    elif bars_held >= 6:
                        exit_now = True
                        exit_p = closes[i]
                        reason = "TimeStop"
                    ret = (exit_p - entry_p) / entry_p
                else:
                    if highs[i] >= stop_p:
                        exit_now = True
                        exit_p = stop_p
                        reason = "StopLoss"
                    elif lows[i] <= target_p:
                        exit_now = True
                        exit_p = target_p
                        reason = "TargetHit"
                    elif bars_held >= 6:
                        exit_now = True
                        exit_p = closes[i]
                        reason = "TimeStop"
                    ret = (entry_p - exit_p) / entry_p

                if exit_now:
                    gross_r = ret / d_stop
                    fee_r = self.c_rt / d_stop
                    net_r = gross_r - fee_r

                    trades.append(
                        Trade(
                            strategy_id=self.strategy_id,
                            asset="BTCUSDT",
                            timeframe=self.timeframe,
                            entry_time=times[entry_idx],
                            exit_time=times[i],
                            entry_price=entry_p,
                            exit_price=exit_p,
                            initial_stop=stop_p,
                            exit_reason=reason,
                            bars_held=bars_held,
                            gross_pnl_r=gross_r,
                            fee_r=fee_r,
                            net_pnl_r=net_r,
                            gross_return_pct=ret * 100.0,
                            net_return_pct=(ret - self.c_rt) * 100.0,
                            side=side,
                        )
                    )
                    in_pos = False

        self._trades = trades
        total_days = (times[-1] - times[0]).days if times else 0
        return compute_performance_metrics(
            self.strategy_id, self.strategy_name, self.category,
            self.primary_asset, self.timeframe, trades, total_days, 1.0
        )
