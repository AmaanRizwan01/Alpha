"""
Walk-forward simulation engine for Project Alpha v3.
Implements rolling 12m train / 3m out-of-sample test slices with 48h purge/embargo.
"""

from dataclasses import asdict, dataclass
from datetime import datetime, timedelta
import polars as pl
import alpha_py


@dataclass(frozen=True)
class WalkForwardWindow:
    fold_idx: int
    train_start: datetime
    train_end: datetime
    test_start: datetime
    test_end: datetime


@dataclass(frozen=True)
class TradeRecord:
    asset: str
    timeframe: str
    exit_policy: str
    fold_idx: int
    entry_bar_idx: int
    entry_time: str
    exit_time: str
    entry_price: float
    initial_stop: float
    exit_price: float
    exit_reason: str
    bars_held: int
    gross_pnl_r: float
    fee_r: float
    net_pnl_r: float
    gross_return_pct: float
    net_return_pct: float


def add_months(dt: datetime, months: int) -> datetime:
    """Add calendar months without third-party dependencies."""
    total_months = dt.year * 12 + (dt.month - 1) + months
    year = total_months // 12
    month = (total_months % 12) + 1
    return datetime(year, month, dt.day, dt.hour, dt.minute, dt.second)


def generate_walkforward_windows(
    start_dt: datetime,
    holdout_start: datetime,
    train_months: int = 12,
    test_months: int = 3,
    purge_hours: int = 48,
) -> list[WalkForwardWindow]:
    """
    Generate rolling 12m train / 3m out-of-sample test windows with a 48h purge gap.
    Evaluation in test set begins 48h after train_end to eliminate boundary leakage.
    """
    windows = []
    fold_idx = 0
    curr_train_start = start_dt

    while True:
        train_end = add_months(curr_train_start, train_months)
        test_start = train_end + timedelta(hours=purge_hours)
        test_end = add_months(train_end, test_months)

        if test_end > holdout_start:
            if train_end < holdout_start:
                final_train_start = add_months(holdout_start, -(train_months + test_months))
                final_train_end = add_months(holdout_start, -test_months)
                final_test_start = final_train_end + timedelta(hours=purge_hours)
                windows.append(
                    WalkForwardWindow(
                        fold_idx=fold_idx,
                        train_start=final_train_start,
                        train_end=final_train_end,
                        test_start=final_test_start,
                        test_end=holdout_start,
                    )
                )
            break

        windows.append(
            WalkForwardWindow(
                fold_idx=fold_idx,
                train_start=curr_train_start,
                train_end=train_end,
                test_start=test_start,
                test_end=test_end,
            )
        )
        fold_idx += 1
        curr_train_start = add_months(curr_train_start, test_months)

    return windows


def prepare_features(df: pl.DataFrame) -> pl.DataFrame:
    """Compute true range and rolling 14-period ATR using Polars."""
    return (
        df.with_columns(prev_c=pl.col("close").shift(1))
        .with_columns(
            tr=pl.max_horizontal(
                pl.col("high") - pl.col("low"),
                (pl.col("high") - pl.col("prev_c")).abs(),
                (pl.col("low") - pl.col("prev_c")).abs(),
            )
        )
        .with_columns(atr=pl.col("tr").rolling_mean(14).fill_null(0.0))
    )


def simulate_window_trades(
    df: pl.DataFrame,
    signals: list[bool],
    window: WalkForwardWindow,
    asset: str,
    timeframe: str,
    exit_policy: str,
    k_stop: float = 2.0,
    k_trail: float = 2.5,
    min_stop_pct: float = 0.022,
    enable_e1: bool = False,
    stagnation_bars: int = 6,
    enable_e2: bool = False,
    maker_fee: float = 0.0008,
    taker_fee: float = 0.0010,
    slippage: float = 0.0005,
) -> list[TradeRecord]:
    """
    Simulate sequential non-overlapping trade executions in the out-of-sample window.
    Uses alpha_py compiled Rust functions for initial stop and trade exit simulation.
    """
    trades = []
    n = len(df)
    times = df["datetime"].to_list()
    opens = df["open"].to_list()
    highs = df["high"].to_list()
    lows = df["low"].to_list()
    closes = df["close"].to_list()
    atrs = df["atr"].to_list()

    i = 0
    while i < n - 1:
        dt = times[i]
        if dt < window.test_start:
            i += 1
            continue
        if dt >= window.test_end:
            break

        if not signals[i]:
            i += 1
            continue

        entry_idx = i + 1
        entry_price = opens[entry_idx]
        entry_time = str(times[entry_idx])
        atr_val = atrs[i]

        if atr_val is None or atr_val <= 0.0:
            i += 1
            continue

        initial_stop = alpha_py.calc_initial_stop(
            entry_price, atr_val, k_stop, min_stop_pct
        )

        exit_res = alpha_py.simulate_trade_exit(
            entry_price=entry_price,
            initial_stop=initial_stop,
            highs=highs[entry_idx:],
            lows=lows[entry_idx:],
            closes=closes[entry_idx:],
            atrs=atrs[entry_idx:],
            k_stop=k_stop,
            k_trail=k_trail,
            min_stop_distance_pct=min_stop_pct,
            enable_e1_stagnation=enable_e1,
            stagnation_bars=stagnation_bars,
            enable_e2_breakeven=enable_e2,
        )

        exit_idx = entry_idx + exit_res.exit_bar
        exit_time = str(times[min(exit_idx, n - 1)])

        stop_dist_pct = (entry_price - initial_stop) / entry_price
        c_rt = maker_fee + taker_fee + slippage
        fee_r = c_rt / stop_dist_pct if stop_dist_pct > 0 else 0.0

        net_pnl_r = exit_res.pnl_r - fee_r
        net_ret_pct = exit_res.return_pct - c_rt

        trades.append(
            TradeRecord(
                asset=asset,
                timeframe=timeframe,
                exit_policy=exit_policy,
                fold_idx=window.fold_idx,
                entry_bar_idx=entry_idx,
                entry_time=entry_time,
                exit_time=exit_time,
                entry_price=entry_price,
                initial_stop=initial_stop,
                exit_price=exit_res.exit_price,
                exit_reason=exit_res.reason,
                bars_held=exit_res.bars_held,
                gross_pnl_r=exit_res.pnl_r,
                fee_r=fee_r,
                net_pnl_r=net_pnl_r,
                gross_return_pct=exit_res.return_pct,
                net_return_pct=net_ret_pct,
            )
        )

        i = max(entry_idx + 1, exit_idx + 1)

    return trades
