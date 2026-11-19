"""Core Strategy: Crypto Volatility Breakout (VOLATILITY_BREAKOUT).

Formerly referred to as H1 Breakout.
Captures trend-following momentum post-consolidation across 4h BTC/ETH spot.
Implements Kaufman Efficiency Ratio (ER) chop gating, Donchian upper band breakout,
taker volume expansion, and E0+E1+E2 exit policy (Chandelier trailing stop,
stagnation cut, delayed breakeven).

Modular Plug-and-Play: Can be deleted or replaced without affecting other strategies.
"""

from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import List, Optional

import numpy as np
import polars as pl
import alpha_py

from python.research.strategies.base import (
    BaseStrategy,
    StrategyPerformance,
    Trade,
    compute_performance_metrics,
)


class VolatilityBreakoutStrategy(BaseStrategy):
    """Regime-Gated Donchian Volatility Breakout strategy for BTC and ETH spot."""

    def __init__(
        self,
        donchian_period: int = 20,
        er_period: int = 24,
        er_threshold: float = 0.30,
        volume_sma_period: int = 20,
        volume_multiplier: float = 1.25,
        min_taker_buy_ratio: float = 0.53,
        k_stop: float = 2.0,
        k_trail: float = 2.5,
        min_stop_pct: float = 0.022,
        stagnation_bars: int = 3,
        maker_fee: float = 0.0008,
        taker_fee: float = 0.0010,
        slippage: float = 0.0005,
    ) -> None:
        self.donchian_period = donchian_period
        self.er_period = er_period
        self.er_threshold = er_threshold
        self.volume_sma_period = volume_sma_period
        self.volume_multiplier = volume_multiplier
        self.min_taker_buy_ratio = min_taker_buy_ratio
        self.k_stop = k_stop
        self.k_trail = k_trail
        self.min_stop_pct = min_stop_pct
        self.stagnation_bars = stagnation_bars
        self.c_rt = maker_fee + taker_fee + slippage

        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "VOLATILITY_BREAKOUT"

    @property
    def strategy_name(self) -> str:
        return "Crypto Volatility Breakout"

    @property
    def category(self) -> str:
        return "Trend-Following Breakout"

    @property
    def primary_asset(self) -> str:
        return "BTC/USDT, ETH/USDT"

    @property
    def timeframe(self) -> str:
        return "4h"

    def _prepare_features(self, df: pl.DataFrame) -> pl.DataFrame:
        """Compute True Range and ATR."""
        return (
            df.with_columns(
                prev_c=pl.col("close").shift(1),
            )
            .with_columns(
                tr=pl.max_horizontal(
                    pl.col("high") - pl.col("low"),
                    (pl.col("high") - pl.col("prev_c")).abs(),
                    (pl.col("low") - pl.col("prev_c")).abs(),
                )
            )
            .with_columns(atr=pl.col("tr").rolling_mean(14).fill_null(0.0))
        )

    def _simulate_asset(
        self, asset: str, df: pl.DataFrame, signals: list[bool]
    ) -> list[Trade]:
        """Simulate sequential non-overlapping trade executions."""
        trades: list[Trade] = []
        n = len(df)
        times = df["datetime"].to_list()
        opens = df["open"].to_list()
        highs = df["high"].to_list()
        lows = df["low"].to_list()
        closes = df["close"].to_list()
        atrs = df["atr"].to_list()

        i = 0
        while i < n - 1:
            if not signals[i]:
                i += 1
                continue

            entry_idx = i + 1
            entry_price = opens[entry_idx]
            entry_dt = times[entry_idx]
            atr_val = atrs[i]

            if atr_val is None or atr_val <= 0.0:
                i += 1
                continue

            initial_stop = alpha_py.calc_initial_stop(
                entry_price, atr_val, self.k_stop, self.min_stop_pct
            )

            exit_res = alpha_py.simulate_trade_exit(
                entry_price=entry_price,
                initial_stop=initial_stop,
                highs=highs[entry_idx:],
                lows=lows[entry_idx:],
                closes=closes[entry_idx:],
                atrs=atrs[entry_idx:],
                k_stop=self.k_stop,
                k_trail=self.k_trail,
                min_stop_distance_pct=self.min_stop_pct,
                enable_e1_stagnation=True,
                stagnation_bars=self.stagnation_bars,
                enable_e2_breakeven=True,
            )

            exit_idx = entry_idx + exit_res.exit_bar
            exit_dt = times[min(exit_idx, n - 1)]

            stop_dist_pct = (entry_price - initial_stop) / entry_price
            fee_r = self.c_rt / stop_dist_pct if stop_dist_pct > 0 else 0.0
            net_pnl_r = exit_res.pnl_r - fee_r
            net_ret_pct = exit_res.return_pct - self.c_rt

            trades.append(
                Trade(
                    strategy_id=self.strategy_id,
                    asset=asset,
                    timeframe=self.timeframe,
                    entry_time=entry_dt,
                    exit_time=exit_dt,
                    entry_price=entry_price,
                    exit_price=exit_res.exit_price,
                    initial_stop=initial_stop,
                    exit_reason=exit_res.reason,
                    bars_held=exit_res.bars_held,
                    gross_pnl_r=exit_res.pnl_r,
                    fee_r=fee_r,
                    net_pnl_r=net_pnl_r,
                    gross_return_pct=exit_res.return_pct,
                    net_return_pct=net_ret_pct,
                    side="LONG",
                )
            )

            i = max(entry_idx + 1, exit_idx + 1)

        return trades

    def run_backtest(self, data_dir: Path) -> StrategyPerformance:
        """Run backtest on BTCUSDT and ETHUSDT 4h data."""
        self._trades = []
        assets = ["BTCUSDT", "ETHUSDT"]
        total_bars = 0

        for asset in assets:
            path = data_dir / f"{asset}_4h.parquet"
            if not path.exists():
                continue
            raw_df = pl.read_parquet(path)
            df = self._prepare_features(raw_df)
            total_bars = max(total_bars, len(df))

            signals = alpha_py.scan_h1_signals(
                highs=df["high"].to_list(),
                lows=df["low"].to_list(),
                closes=df["close"].to_list(),
                volumes=df["volume"].to_list(),
                taker_volumes=df["taker_buy_volume"].to_list(),
                er_period=self.er_period,
                er_threshold=self.er_threshold,
                donchian_period=self.donchian_period,
                volume_sma_period=self.volume_sma_period,
                volume_multiplier=self.volume_multiplier,
                min_taker_buy_ratio=self.min_taker_buy_ratio,
            )

            trades = self._simulate_asset(asset, df, signals)
            self._trades.extend(trades)

        self._trades.sort(key=lambda t: t.exit_time)

        # Build equity series
        if self._trades:
            dates = [t.exit_time for t in self._trades]
            cum_r = list(np.cumsum([t.net_pnl_r for t in self._trades]))
            self._equity_df = pl.DataFrame({
                "datetime": dates,
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
            total_bars=total_bars,
            bar_duration_hours=4.0,
            equity_curve=self._equity_df,
            num_assets=len(assets),
        )
        perf.correlation_to_h1 = 1.00  # Benchmark self-correlation
        return perf

    def get_trades(self) -> list[Trade]:
        return self._trades

    def get_equity_series(self) -> pl.DataFrame:
        return self._equity_df if self._equity_df is not None else pl.DataFrame()


# Alias for backward compatibility
H1BreakoutStrategy = VolatilityBreakoutStrategy
