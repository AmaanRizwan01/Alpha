"""Base Strategy Protocol and Shared Analytics for Project Alpha Research.

Enforces scale-invariant fee modeling, R-multiple accounting, and standardized
performance metrics across all quantitative strategies.

Plug-and-play design: Any strategy file can be dropped into or deleted from the
strategies directory without breaking other strategies or the framework.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Tuple, Dict, Any

import numpy as np
import polars as pl


@dataclass
class Trade:
    strategy_id: str
    asset: str
    timeframe: str
    entry_time: datetime
    exit_time: datetime
    entry_price: float
    exit_price: float
    initial_stop: float
    exit_reason: str
    bars_held: int
    gross_pnl_r: float
    fee_r: float
    net_pnl_r: float
    gross_return_pct: float
    net_return_pct: float
    side: str = "LONG"  # "LONG" or "SHORT"
    notional: float = 0.0


@dataclass
class StrategyPerformance:
    strategy_id: str
    strategy_name: str
    category: str
    primary_asset: str
    timeframe: str
    trade_count: int
    win_rate_pct: float
    ev_net_r: float
    ev_gross_r: float
    fee_drag_r: float
    total_net_r: float
    profit_factor: float
    sharpe_ratio: float
    sortino_ratio: float
    max_drawdown_r: float
    max_drawdown_pct: float
    market_exposure_pct: float
    avg_bars_held: float
    annualized_return_pct: float
    correlation_to_h1: float = 0.0
    correlation_to_benchmark: float = 0.0
    extra_metrics: Dict[str, Any] = field(default_factory=dict)


def compute_max_drawdown_r(pnl_r_series: list[float]) -> float:
    """Calculate peak-to-trough maximum drawdown in R-multiples."""
    if not pnl_r_series:
        return 0.0
    peak = 0.0
    cum = 0.0
    max_dd = 0.0
    for r in pnl_r_series:
        cum += r
        if cum > peak:
            peak = cum
        dd = peak - cum
        if dd > max_dd:
            max_dd = dd
    return max_dd


def compute_performance_metrics(
    strategy_id: str,
    strategy_name: str,
    category: str,
    primary_asset: str,
    timeframe: str,
    trades: list[Trade],
    total_bars: int,
    bar_duration_hours: float,
    start_time: Optional[datetime] = None,
    end_time: Optional[datetime] = None,
    equity_curve: Optional[pl.DataFrame] = None,
    num_assets: int = 1,
    extra_metrics: Optional[Dict[str, Any]] = None,
) -> StrategyPerformance:
    """Compute standardized performance statistics across trade records."""
    n = len(trades)
    if n == 0:
        return StrategyPerformance(
            strategy_id=strategy_id,
            strategy_name=strategy_name,
            category=category,
            primary_asset=primary_asset,
            timeframe=timeframe,
            trade_count=0,
            win_rate_pct=0.0,
            ev_net_r=0.0,
            ev_gross_r=0.0,
            fee_drag_r=0.0,
            total_net_r=0.0,
            profit_factor=0.0,
            sharpe_ratio=0.0,
            sortino_ratio=0.0,
            max_drawdown_r=0.0,
            max_drawdown_pct=0.0,
            market_exposure_pct=0.0,
            avg_bars_held=0.0,
            annualized_return_pct=0.0,
            extra_metrics=extra_metrics or {},
        )

    net_r = [t.net_pnl_r for t in trades]
    gross_r = [t.gross_pnl_r for t in trades]
    fee_r = [t.fee_r for t in trades]
    net_ret = [t.net_return_pct for t in trades]

    wins = [r for r in net_r if r > 0.0]
    losses = [abs(r) for r in net_r if r < 0.0]
    sum_wins = sum(wins)
    sum_losses = sum(losses)

    win_rate = len(wins) / n * 100.0
    ev_net = sum(net_r) / n
    ev_gross = sum(gross_r) / n
    fee_drag = sum(fee_r) / n
    total_net_r = sum(net_r)
    pf = (sum_wins / sum_losses) if sum_losses > 0 else (99.0 if sum_wins > 0 else 0.0)
    max_dd_r = compute_max_drawdown_r(net_r)

    total_bars_held = sum(t.bars_held for t in trades)
    avg_bars_held = total_bars_held / n if n > 0 else 0.0
    effective_universe_bars = max(1, total_bars * max(1, num_assets))
    market_exposure = min(100.0, (total_bars_held / effective_universe_bars) * 100.0)

    # Annualization factor based on bar duration
    # 1 year ~ 8760 hours
    bars_per_year = 8760.0 / bar_duration_hours if bar_duration_hours > 0 else 2190.0

    # Sharpe & Sortino (per-trade and annualized)
    r_arr = np.array(net_r)
    std_r = float(np.std(r_arr, ddof=1)) if len(r_arr) > 1 else 1e-6
    if std_r <= 1e-9:
        std_r = 1e-6
    per_trade_sharpe = float(ev_net / std_r)
    trades_per_year = n / max(0.2, (total_bars / bars_per_year))
    ann_sharpe = per_trade_sharpe * np.sqrt(max(1.0, trades_per_year))

    # Downside std for Sortino
    downside = r_arr[r_arr < 0.0]
    if len(downside) > 1:
        downside_std = float(np.std(downside, ddof=1))
    else:
        downside_std = std_r
    if downside_std <= 1e-9:
        downside_std = 1e-6
    ann_sortino = (ev_net / downside_std) * np.sqrt(max(1.0, trades_per_year))

    # Annualized return pct (assuming standard 1.0% risk per trade: return ~ total_net_r * 0.01)
    years = max(0.5, total_bars / bars_per_year)
    ann_return_pct = (total_net_r * 1.0) / years  # 1.0% risk per R

    # Max Drawdown in %: max_dd_r * 1.0%
    max_dd_pct = max_dd_r * 1.0

    return StrategyPerformance(
        strategy_id=strategy_id,
        strategy_name=strategy_name,
        category=category,
        primary_asset=primary_asset,
        timeframe=timeframe,
        trade_count=n,
        win_rate_pct=win_rate,
        ev_net_r=ev_net,
        ev_gross_r=ev_gross,
        fee_drag_r=fee_drag,
        total_net_r=total_net_r,
        profit_factor=pf,
        sharpe_ratio=ann_sharpe,
        sortino_ratio=ann_sortino,
        max_drawdown_r=max_dd_r,
        max_drawdown_pct=max_dd_pct,
        market_exposure_pct=market_exposure,
        avg_bars_held=avg_bars_held,
        annualized_return_pct=ann_return_pct,
        extra_metrics=extra_metrics or {},
    )


class BaseStrategy(ABC):
    """Abstract Base Class for all plug-and-play quantitative strategies."""

    @property
    @abstractmethod
    def strategy_id(self) -> str:
        """Unique uppercase identifier, e.g. 'VOLATILITY_BREAKOUT', 'FUNDING_CARRY', 'MACRO_TREND'."""
        pass

    @property
    @abstractmethod
    def strategy_name(self) -> str:
        """Human-readable descriptive name."""
        pass

    @property
    @abstractmethod
    def category(self) -> str:
        """Strategy classification category."""
        pass

    @property
    @abstractmethod
    def primary_asset(self) -> str:
        """Primary asset or basket description."""
        pass

    @property
    @abstractmethod
    def timeframe(self) -> str:
        """Primary bar timeframe, e.g. '1h', '4h', '8h', '1D'."""
        pass

    @abstractmethod
    def run_backtest(self, data_dir: Path) -> StrategyPerformance:
        """Execute the strategy backtest over available historical data."""
        pass

    @abstractmethod
    def get_trades(self) -> list[Trade]:
        """Return list of all executed trades from the most recent run."""
        pass

    @abstractmethod
    def get_equity_series(self) -> pl.DataFrame:
        """Return a DataFrame with datetime and cumulative_net_r for correlation analysis."""
        pass
