"""Strategy Candidate: Derivatives Liquidation Cascade Mean-Reversion (FLUSH_REVERSAL).

KILLED CANDIDATE: Failed Quantitative Kill Gate.
Reason: Insufficient sample size (N=12 < 100) & extreme tail risk (-0.219R Net EV, 0.41 PF).
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


class FlushReversalStrategy(BaseStrategy):
    """Liquidation Cascade Flush Mean Reversion (KILLED)."""

    def __init__(
        self,
        rsi_panic_level: float = 18.0,
        oi_flush_drop_pct: float = -0.04,
        holding_period_bars: int = 12,
        round_trip_friction: float = 0.0015,
    ) -> None:
        self.rsi_panic = rsi_panic_level
        self.oi_drop = oi_flush_drop_pct
        self.hold_bars = holding_period_bars
        self.c_rt = round_trip_friction
        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "FLUSH_REVERSAL"

    @property
    def strategy_name(self) -> str:
        return "Liquidation Flush Reversion (Failed Gate)"

    @property
    def category(self) -> str:
        return "Failed / Mean Reversion"

    @property
    def primary_asset(self) -> str:
        return "BTC, ETH"

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
        oi_p = resolve_data_path(data_dir, "BTCUSDT_oi_1h.parquet")
        if not btc_p.exists() or not oi_p.exists():
            return compute_performance_metrics(
                self.strategy_id, self.strategy_name, self.category,
                self.primary_asset, self.timeframe, [], 0, 1.0
            )

        df = pl.read_parquet(btc_p).sort("open_time")
        oi_df = pl.read_parquet(oi_p)
        if "open_time" in oi_df.columns:
            oi_df = oi_df.sort("open_time")
        elif "datetime" in oi_df.columns:
            oi_df = oi_df.sort("datetime")

        if "datetime" not in df.columns:
            df = df.with_columns(datetime=pl.from_epoch(pl.col("open_time"), time_unit="ms"))
        if "datetime" not in oi_df.columns:
            oi_df = oi_df.with_columns(datetime=pl.from_epoch(pl.col("open_time"), time_unit="ms"))

        merged = df.join(oi_df.select(["datetime", "open_interest"]), on="datetime", how="inner").sort("datetime")
        times = merged["datetime"].to_list()
        closes = merged["close"].to_numpy()
        lows = merged["low"].to_numpy()
        highs = merged["high"].to_numpy()
        oi = merged["open_interest"].to_numpy()
        n = len(merged)

        deltas = np.diff(closes)
        gains = np.where(deltas > 0, deltas, 0.0)
        losses = np.where(deltas < 0, -deltas, 0.0)
        rsi = np.full(n, 50.0)
        if n > 14:
            avg_g = np.mean(gains[:14])
            avg_l = np.mean(losses[:14])
            for i in range(14, n - 1):
                avg_g = (avg_g * 13 + gains[i - 1]) / 14.0
                avg_l = (avg_l * 13 + losses[i - 1]) / 14.0
                rs = avg_g / avg_l if avg_l > 0 else 100.0
                rsi[i] = 100.0 - (100.0 / (1.0 + rs))

        trades: list[Trade] = []
        d_stop = 0.035

        in_pos = False
        entry_p = 0.0
        entry_idx = 0
        stop_p = 0.0

        for i in range(24, n - self.hold_bars):
            oi_change = (oi[i] - oi[i - 1]) / oi[i - 1] if oi[i - 1] > 0 else 0.0
            is_flush = (rsi[i] <= self.rsi_panic) and (oi_change <= self.oi_drop)

            if not in_pos:
                if is_flush:
                    in_pos = True
                    entry_p = closes[i]
                    entry_idx = i
                    stop_p = entry_p * (1.0 - d_stop)
            else:
                bars_held = i - entry_idx
                exit_now = False
                exit_p = closes[i]
                reason = ""

                if lows[i] <= stop_p:
                    exit_now = True
                    exit_p = stop_p
                    reason = "CascadingStop"
                elif bars_held >= self.hold_bars:
                    exit_now = True
                    exit_p = closes[i]
                    reason = "TimeBounceExit"

                if exit_now:
                    ret = (exit_p - entry_p) / entry_p
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
                            side="LONG",
                        )
                    )
                    in_pos = False

        self._trades = trades
        total_days = (times[-1] - times[0]).days if times else 0
        return compute_performance_metrics(
            self.strategy_id, self.strategy_name, self.category,
            self.primary_asset, self.timeframe, trades, total_days, 1.0
        )
