"""
Purged Walk-Forward Simulation Runner for Project Alpha v3.
Simulates H1 breakouts across BTC/ETH (1h & 4h) with E0..E2 exit ablations.
"""

from dataclasses import asdict
from datetime import datetime
from pathlib import Path
import polars as pl
import alpha_py

from python.research.walkforward import (
    TradeRecord,
    WalkForwardWindow,
    generate_walkforward_windows,
    prepare_features,
    simulate_window_trades,
)


def calc_max_drawdown_r(pnl_r_list: list[float]) -> float:
    """Calculate peak-to-trough cumulative drawdown in R-multiples."""
    if not pnl_r_list:
        return 0.0
    peak = 0.0
    cum = 0.0
    max_dd = 0.0
    for r in pnl_r_list:
        cum += r
        if cum > peak:
            peak = cum
        dd = peak - cum
        if dd > max_dd:
            max_dd = dd
    return max_dd


def compute_summary(
    trades: list[TradeRecord], asset: str, tf: str, policy: str
) -> dict:
    """Compute aggregate performance metrics for a configuration."""
    n = len(trades)
    if n == 0:
        return {
            "asset": asset, "tf": tf, "policy": policy, "n": 0,
            "win_rate": 0.0, "ev_gross_r": 0.0, "fee_drag_r": 0.0,
            "ev_net_r": 0.0, "total_net_r": 0.0, "pf": 0.0, "max_dd_r": 0.0,
        }
    pnl_net = [t.net_pnl_r for t in trades]
    wins = [r for r in pnl_net if r > 0.0]
    losses = [abs(r) for r in pnl_net if r < 0.0]
    sum_wins = sum(wins)
    sum_losses = sum(losses)
    pf = sum_wins / sum_losses if sum_losses > 0 else (99.0 if sum_wins > 0 else 0.0)

    return {
        "asset": asset,
        "tf": tf,
        "policy": policy,
        "n": n,
        "win_rate": len(wins) / n * 100.0,
        "ev_gross_r": sum(t.gross_pnl_r for t in trades) / n,
        "fee_drag_r": sum(t.fee_r for t in trades) / n,
        "ev_net_r": sum(pnl_net) / n,
        "total_net_r": sum(pnl_net),
        "pf": pf,
        "max_dd_r": calc_max_drawdown_r(pnl_net),
    }


def load_and_scan(asset: str, tf: str) -> tuple[pl.DataFrame, list[bool]]:
    """Load parquet, compute features, and scan H1 signals with alpha_py."""
    df = prepare_features(pl.read_parquet(f"data/raw/{asset}_{tf}.parquet"))
    signals = alpha_py.scan_h1_signals(
        highs=df["high"].to_list(),
        lows=df["low"].to_list(),
        closes=df["close"].to_list(),
        volumes=df["volume"].to_list(),
        taker_volumes=df["taker_buy_volume"].to_list(),
        er_period=24,
        er_threshold=0.30,
        donchian_period=20,
        volume_sma_period=20,
        volume_multiplier=1.25,
        min_taker_buy_ratio=0.53,
    )
    return df, signals


def simulate_dataset(
    asset: str,
    tf: str,
    policies: list[dict],
    windows: list[WalkForwardWindow],
) -> tuple[list[TradeRecord], list[dict]]:
    """Scan H1 signals and simulate all exit policies across all walk-forward folds."""
    df, signals = load_and_scan(asset, tf)
    all_trades = []
    summaries = []
    stagnation_bars = 6 if tf == "1h" else 3

    for pol in policies:
        pol_trades = []
        for w in windows:
            trades = simulate_window_trades(
                df=df,
                signals=signals,
                window=w,
                asset=asset,
                timeframe=tf,
                exit_policy=pol["name"],
                k_stop=2.0,
                k_trail=2.5,
                min_stop_pct=0.022,
                enable_e1=pol["e1"],
                stagnation_bars=stagnation_bars,
                enable_e2=pol["e2"],
            )
            pol_trades.extend(trades)

        all_trades.extend(pol_trades)
        summaries.append(compute_summary(pol_trades, asset, tf, pol["name"]))

    return all_trades, summaries


def format_markdown_table(summaries: list[dict]) -> str:
    """Generate clean markdown table of walk-forward results."""
    header = (
        "| Asset | TF | Exit Policy | N Trades | Win Rate | "
        "EV Gross (R) | Fee Drag (R) | EV Net (R) | Total Net R | PF | Max DD (R) |\n"
        "|---|---|---|---|---|---|---|---|---|---|---|\n"
    )
    rows = []
    for s in summaries:
        row = (
            f"| {s['asset']} | {s['tf']} | {s['policy']} | {s['n']} | "
            f"{s['win_rate']:.1f}% | {s['ev_gross_r']:+.3f}R | "
            f"{s['fee_drag_r']:.3f}R | {s['ev_net_r']:+.3f}R | "
            f"{s['total_net_r']:+.1f}R | {s['pf']:.2f} | {s['max_dd_r']:.2f}R |"
        )
        rows.append(row)
    return header + "\n".join(rows)


def main() -> None:
    """Execute purged walk-forward matrix and save trade records."""
    start = datetime(2021, 1, 1)
    holdout = datetime(2026, 5, 1)
    windows = generate_walkforward_windows(start, holdout, 12, 3, 48)

    policies = [
        {"name": "E0", "e1": False, "e2": False},
        {"name": "E0_E1", "e1": True, "e2": False},
        {"name": "E0_E2", "e1": False, "e2": True},
        {"name": "E0_E1_E2", "e1": True, "e2": True},
    ]

    datasets = [
        ("BTCUSDT", "1h"),
        ("BTCUSDT", "4h"),
        ("ETHUSDT", "1h"),
        ("ETHUSDT", "4h"),
    ]

    all_trades = []
    all_summaries = []

    for asset, tf in datasets:
        print(f"--> Simulating {asset} {tf} across 18 walk-forward folds...")
        trades, summaries = simulate_dataset(asset, tf, policies, windows)
        all_trades.extend(trades)
        all_summaries.extend(summaries)

    trade_dicts = [asdict(t) for t in all_trades]
    df_trades = pl.DataFrame(trade_dicts)
    df_trades.write_parquet("data/walkforward_trades.parquet")

    print(f"\n[Project Alpha] Purged Walk-Forward Simulation Complete")
    print(f"Total Out-Of-Sample Trades Logged: {len(df_trades)}")
    print(f"Saved: data/walkforward_trades.parquet\n")
    print(format_markdown_table(all_summaries))


if __name__ == "__main__":
    main()
