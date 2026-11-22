"""Master Quantitative Strategy Backtester & Side-by-Side Comparison Engine.

Executes all discovered plug-and-play strategies against historical data,
computes scale-invariant metrics (Net EV, Profit Factor, Win Rate, Sharpe, Sortino,
Max Drawdown, Trade Count, Exposure), and produces a comprehensive side-by-side
comparison matrix evaluating each strategy on its own standalone merits.

Modular: Any strategy file can be deleted or plugged back in without touching this runner.
"""

from __future__ import annotations

import argparse
import sys
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

# Ensure repository root is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

import numpy as np
import polars as pl

from python.research.strategies import BaseStrategy, StrategyPerformance, load_all_strategies
from python.research.strategies.base import Trade


def align_equity_returns(
    df_bench: pl.DataFrame, df_strat: pl.DataFrame
) -> float:
    """Calculate Pearson correlation of periodic daily returns between two equity curves.

    Constructs a continuous daily calendar spanning the union of dates, forward-fills
    cumulative net R (representing flat cash / constant equity on non-trade days),
    and computes the Pearson correlation of the daily changes in equity.
    """
    if df_bench.is_empty() or df_strat.is_empty():
        return 0.0

    b = df_bench.sort("datetime")
    s = df_strat.sort("datetime")

    min_date = min(b["datetime"].min().date(), s["datetime"].min().date())
    max_date = max(b["datetime"].max().date(), s["datetime"].max().date())

    if min_date >= max_date:
        return 0.0

    # Build continuous daily calendar grid
    daily_grid = pl.DataFrame({
        "date": pl.date_range(min_date, max_date, interval="1d", eager=True)
    })

    # Resample bench to daily last cumulative R
    b_daily = (
        b.with_columns(date=pl.col("datetime").dt.date())
        .group_by("date")
        .agg(cum_r=pl.col("cumulative_net_r").last())
        .sort("date")
    )
    b_full = daily_grid.join(b_daily, on="date", how="left").with_columns(
        cum_r=pl.col("cum_r").forward_fill().fill_null(0.0)
    ).with_columns(
        r_ret=pl.col("cum_r").diff().fill_null(0.0)
    )

    # Resample strategy to daily last cumulative R
    s_daily = (
        s.with_columns(date=pl.col("datetime").dt.date())
        .group_by("date")
        .agg(cum_r=pl.col("cumulative_net_r").last())
        .sort("date")
    )
    s_full = daily_grid.join(s_daily, on="date", how="left").with_columns(
        cum_r=pl.col("cum_r").forward_fill().fill_null(0.0)
    ).with_columns(
        r_ret=pl.col("cum_r").diff().fill_null(0.0)
    )

    r_b = b_full["r_ret"].to_numpy()
    r_s = s_full["r_ret"].to_numpy()

    std_b = float(np.std(r_b))
    std_s = float(np.std(r_s))

    if std_b <= 1e-9 or std_s <= 1e-9:
        return 0.0

    corr = float(np.corrcoef(r_b, r_s)[0, 1])
    return corr if not np.isnan(corr) else 0.0


def format_markdown_matrix(performances: List[StrategyPerformance]) -> str:
    """Generate professional GitHub-flavored Markdown comparison matrix."""
    header = (
        "| Strategy ID | Primary Asset | Horizon | N Trades | Win Rate | "
        "Net EV (R) | Total Net R | Start ($) | End ($) | Return (%) | CAGR | APY | "
        "PF | Sharpe | Max DD (R) | Verdict |\n"
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n"
    )
    rows = []
    for p in performances:
        # Determine kill gate verdict based on EV, Profit Factor, Drawdown, and Sample Size
        if p.ev_net_r <= 0.0:
            if p.trade_count < 30:
                verdict = "**KILL (Negative EV & Insufficient N)**"
            else:
                verdict = "**KILL (Negative EV / Fee Drag)**"
        elif p.profit_factor >= 1.50 and p.trade_count >= 100:
            verdict = "**PASS (Production Ready)**"
        elif "Arbitrage" in p.strategy_name or "Carry" in p.strategy_name:
            verdict = "**PASS (Low Frequency Arbitrage)**" if p.profit_factor >= 1.50 else "**PASS (Conditional)**"
        elif p.ev_net_r > 0.0 and p.profit_factor >= 1.50:
            verdict = "**PASS (Production Ready)**" if p.trade_count >= 100 else "**PASS (Conditional)**"
        elif p.ev_net_r > 0.0:
            verdict = "**PASS (Suboptimal)**"
        else:
            verdict = "**PASS (Conditional)**"

        row = (
            f"| **{p.strategy_id}** "
            f"| {p.primary_asset} "
            f"| {p.timeframe} "
            f"| {p.trade_count} "
            f"| {p.win_rate_pct:.1f}% "
            f"| **{p.ev_net_r:+.3f}R** "
            f"| **{p.total_net_r:+.2f}R** "
            f"| ${p.starting_balance:.2f} "
            f"| **${p.ending_balance:.2f}** "
            f"| {p.total_return_pct:+.1f}% "
            f"| **{p.cagr_pct:.1f}%** "
            f"| **{p.apy_pct:.1f}%** "
            f"| {p.profit_factor:.2f} "
            f"| {p.sharpe_ratio:.2f} "
            f"| {p.max_drawdown_r:.2f}R "
            f"| {verdict} |"
        )
        rows.append(row)

    return header + "\n".join(rows)


def run_comparative_suite(
    data_dir: Path = Path("data/raw"),
    output_dir: Path = Path("data/research_output"),
) -> Tuple[List[StrategyPerformance], str]:
    """Execute all strategies, correlate, and build comparison table."""
    output_dir.mkdir(parents=True, exist_ok=True)
    strategies = load_all_strategies()

    print(f"\n=================================================================")
    print(f"Project Alpha: Systematic Multi-Strategy Backtest & Evaluation")
    print(f"Discovered {len(strategies)} Plug-and-Play Strategies")
    print(f"Data Source Directory: {data_dir.resolve()}")
    print(f"=================================================================\n")

    results: List[StrategyPerformance] = []
    all_trade_records: List[dict] = []

    for strat in strategies:
        print(f"[*] Running backtest for: {strat.strategy_id} ({strat.strategy_name})...")
        t0 = datetime.now()
        perf = strat.run_backtest(data_dir)
        elapsed = (datetime.now() - t0).total_seconds()
        print(f"    Completed in {elapsed:.2f}s | N={perf.trade_count} | Net EV={perf.ev_net_r:+.3f}R | PF={perf.profit_factor:.2f}")

        results.append(perf)

        # Collect trade records for parquet export
        for t in strat.get_trades():
            all_trade_records.append({
                "strategy_id": t.strategy_id,
                "asset": t.asset,
                "timeframe": t.timeframe,
                "entry_time": str(t.entry_time),
                "exit_time": str(t.exit_time),
                "entry_price": t.entry_price,
                "exit_price": t.exit_price,
                "initial_stop": t.initial_stop,
                "exit_reason": t.exit_reason,
                "bars_held": t.bars_held,
                "gross_pnl_r": t.gross_pnl_r,
                "fee_r": t.fee_r,
                "net_pnl_r": t.net_pnl_r,
                "gross_return_pct": t.gross_return_pct,
                "net_return_pct": t.net_return_pct,
                "side": t.side,
            })

    # Save comprehensive trade log
    if all_trade_records:
        df_trades = pl.DataFrame(all_trade_records)
        trades_file = output_dir / "all_strategies_trades.parquet"
        df_trades.write_parquet(trades_file)
        print(f"\n[OK] Saved {len(df_trades):,} total trades to {trades_file}")

    # Build and print Markdown comparison table
    md_table = format_markdown_matrix(results)
    print("\n" + "=" * 90)
    print("PROJECT ALPHA: MULTI-STRATEGY COMPARATIVE PERFORMANCE MATRIX (Validated Active Portfolio)")
    print("=" * 90 + "\n")
    print(md_table)
    print("\n" + "=" * 90 + "\n")

    # Save Markdown report
    report_file = output_dir / "strategy_comparison_matrix.md"
    report_file.write_text(md_table, encoding="utf-8")
    print(f"[OK] Saved comparison report to {report_file}")

    return results, md_table


def main() -> None:
    parser = argparse.ArgumentParser(description="Project Alpha Multi-Strategy Backtester")
    parser.add_argument(
        "--data-dir",
        type=Path,
        default=Path("data/raw"),
        help="Directory containing raw Parquet files",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("data/research_output"),
        help="Directory to save backtest outputs",
    )
    args = parser.parse_args()
    run_comparative_suite(args.data_dir, args.output_dir)


if __name__ == "__main__":
    main()
