"""Strategy: Spot-Perpetual Funding Rate Carry Arbitrage (FUNDING_CARRY).

Delta-neutral cash-and-carry harvesting structural retail perpetual financing premiums.
Allocates 2/3 capital to Spot Long and 1/3 capital as isolated margin at 2.0x Perp Short,
achieving delta neutrality (Net Delta = 0.0) and collecting 8h funding rate cash flows.

Modular Plug-and-Play: Can be deleted or replaced without affecting other strategies.
"""

from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import List, Optional

import numpy as np
import polars as pl

from python.research.strategies.base import (
    BaseStrategy,
    StrategyPerformance,
    Trade,
    compute_performance_metrics,
)


def _make_carry_trade(
    strategy_id: str,
    asset: str,
    timeframe: str,
    entry_time: datetime,
    exit_time: datetime,
    bars_held: int,
    gross_pct: float,
    fee_pct_total: float,
    r_unit: float,
    exit_reason: str,
) -> Trade:
    """Construct a standardized Trade object for carry arbitrage."""
    net_pct = gross_pct - fee_pct_total
    return Trade(
        strategy_id=strategy_id,
        asset=asset,
        timeframe=timeframe,
        entry_time=entry_time,
        exit_time=exit_time,
        entry_price=1.0,
        exit_price=1.0 + net_pct,
        initial_stop=1.0 - r_unit,
        exit_reason=exit_reason,
        bars_held=bars_held,
        gross_pnl_r=gross_pct / r_unit,
        fee_r=fee_pct_total / r_unit,
        net_pnl_r=net_pct / r_unit,
        gross_return_pct=gross_pct,
        net_return_pct=net_pct,
        side="DELTA_NEUTRAL",
    )


def _calc_funding_apr(rates: np.ndarray, alpha: float) -> np.ndarray:
    """Calculate EMA-smoothed annualized funding rate (APR)."""
    n = len(rates)
    ema = np.zeros(n)
    ema[0] = rates[0]
    for i in range(1, n):
        ema[i] = alpha * rates[i] + (1.0 - alpha) * ema[i - 1]
    return ema * 3.0 * 365.0


def _build_equity_df(trades: list[Trade]) -> pl.DataFrame:
    """Build cumulative equity curve dataframe from trade sequence."""
    if not trades:
        return pl.DataFrame({"datetime": [], "cumulative_net_r": []})
    dates = [t.exit_time for t in trades]
    cum_r = list(np.cumsum([t.net_pnl_r for t in trades]))
    return pl.DataFrame({"datetime": dates, "cumulative_net_r": cum_r})


class FundingCarryStrategy(BaseStrategy):
    """Delta-Neutral Spot-Perpetual Funding Carry Arbitrage Strategy."""

    def __init__(
        self,
        entry_apr_threshold: float = 0.120,     # 12.0% APR hurdle
        exit_apr_threshold: float = 0.022,      # 2.2% APR funding decay threshold
        ema_alpha: float = 0.20,                # 9-period smoothing EMA
        inversion_stop_rate: float = -0.00015,  # -0.015% funding inversion stop
        round_trip_friction: float = 0.0032,    # 32 bps round-trip friction
        r_unit_basis_pct: float = 0.010,        # 1.0% equity baseline per R unit
        fee_hurdle_mult: float = 1.0,           # Fee-hurdle multiplier (1.0 = 100% round-trip fee)
    ) -> None:
        self.entry_apr = entry_apr_threshold
        self.exit_apr = exit_apr_threshold
        self.ema_alpha = ema_alpha
        self.inversion_stop = inversion_stop_rate
        self.c_rt = round_trip_friction
        self.r_unit = r_unit_basis_pct
        self.fee_hurdle_mult = fee_hurdle_mult

        self._trades: list[Trade] = []
        self._equity_df: Optional[pl.DataFrame] = None

    @property
    def strategy_id(self) -> str:
        return "FUNDING_CARRY"

    @property
    def strategy_name(self) -> str:
        return "Spot-Perp Funding Carry"

    @property
    def category(self) -> str:
        return "Delta-Neutral Structural Arbitrage"

    @property
    def primary_asset(self) -> str:
        return "BTC/USDT, ETH/USDT (Spot vs Perpetual)"

    @property
    def timeframe(self) -> str:
        return "8h"

    def _simulate_symbol_carry(
        self, symbol: str, f_df: pl.DataFrame
    ) -> tuple[list[Trade], int]:
        """Simulate delta-neutral carry execution across 8h epochs."""
        rates = f_df["funding_rate"].to_numpy()
        times = f_df["datetime"].to_list()
        n = len(rates)
        apr = _calc_funding_apr(rates, self.ema_alpha)

        trades: list[Trade] = []
        in_pos = False
        entry_idx = 0
        decay_consecutive = 0
        perp_fraction = 2.0 / 3.0
        fee_pct_total = perp_fraction * self.c_rt

        for i in range(n):
            if not in_pos:
                if apr[i] >= self.entry_apr:
                    in_pos, entry_idx, decay_consecutive = True, i, 0
                continue

            # 1. Inversion stop: exit immediately if funding rate flips negative
            if rates[i] < self.inversion_stop:
                gross_pct = perp_fraction * float(np.sum(rates[entry_idx : i + 1]))
                trades.append(_make_carry_trade(
                    self.strategy_id, symbol, self.timeframe, times[entry_idx],
                    times[i], i - entry_idx + 1, gross_pct, fee_pct_total,
                    self.r_unit, "FundingInversionStop",
                ))
                in_pos = False
                continue

            # 2. Funding decay exit: gated by fee-hurdle lock
            if apr[i] > self.exit_apr:
                decay_consecutive = 0
                continue

            decay_consecutive += 1
            if decay_consecutive < 3:
                continue

            gross_pct = perp_fraction * float(np.sum(rates[entry_idx : i + 1]))
            if gross_pct < fee_pct_total * self.fee_hurdle_mult:
                continue

            trades.append(_make_carry_trade(
                self.strategy_id, symbol, self.timeframe, times[entry_idx],
                times[i], i - entry_idx + 1, gross_pct, fee_pct_total,
                self.r_unit, "FundingDecayExit",
            ))
            in_pos = False

        if in_pos:
            gross_pct = perp_fraction * float(np.sum(rates[entry_idx:n]))
            trades.append(_make_carry_trade(
                self.strategy_id, symbol, self.timeframe, times[entry_idx],
                times[-1], n - entry_idx, gross_pct, fee_pct_total,
                self.r_unit, "EndOfSample",
            ))

        return trades, n

    def run_backtest(self, data_dir: Path) -> StrategyPerformance:
        symbols = ["BTCUSDT", "ETHUSDT"]
        self._trades = []
        total_epochs = 0

        for s in symbols:
            p = data_dir / f"{s}_funding.parquet"
            if not p.exists():
                continue
            df = pl.read_parquet(p).sort("calc_time")
            trades, epochs = self._simulate_symbol_carry(s, df)
            self._trades.extend(trades)
            total_epochs = max(total_epochs, epochs)

        self._trades.sort(key=lambda t: t.exit_time)
        self._equity_df = _build_equity_df(self._trades)

        return compute_performance_metrics(
            strategy_id=self.strategy_id,
            strategy_name=self.strategy_name,
            category=self.category,
            primary_asset=self.primary_asset,
            timeframe=self.timeframe,
            trades=self._trades,
            total_bars=total_epochs,
            bar_duration_hours=8.0,
            equity_curve=self._equity_df,
            num_assets=2,
            extra_metrics={
                "total_net_yield_pct": sum(t.net_return_pct for t in self._trades) * 100.0,
                "mean_trade_holding_days": (
                    float(np.mean([t.bars_held * 8.0 / 24.0 for t in self._trades]))
                    if self._trades
                    else 0.0
                ),
            },
        )

    def get_trades(self) -> list[Trade]:
        return self._trades

    def get_equity_series(self) -> pl.DataFrame:
        return self._equity_df if self._equity_df is not None else pl.DataFrame()


# Alias for backward compatibility
CarryArbitrageStrategy = FundingCarryStrategy
