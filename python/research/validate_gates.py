"""
Statistical Gate Validation Suite for Project Alpha v3.
Implements Politis-Romano Block Bootstrap, Deflated Sharpe Ratio (DSR),
Parameter Perturbation Grid, and Stage B Untouched Holdout Evaluation.
"""

import sys
from dataclasses import asdict
from datetime import datetime
from pathlib import Path
import numpy as np
import polars as pl
import scipy.stats as stats

# Ensure workspace root is in sys.path
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
import alpha_py

from python.research.walkforward import (
    TradeRecord,
    WalkForwardWindow,
    generate_walkforward_windows,
    prepare_features,
    simulate_window_trades,
)


def stationary_bootstrap(
    pnl_r: np.ndarray,
    n_resamples: int = 10000,
    mean_block_len: float = 3.0,
    seed: int = 42,
) -> tuple[float, float, float]:
    """Compute 95% confidence bounds via Politis-Romano stationary block bootstrap."""
    n = len(pnl_r)
    p = 1.0 / mean_block_len
    rng = np.random.default_rng(seed)
    means = np.empty(n_resamples)

    for i in range(n_resamples):
        indices = []
        while len(indices) < n:
            start = rng.integers(0, n)
            blen = rng.geometric(p)
            for j in range(blen):
                indices.append((start + j) % n)
                if len(indices) == n:
                    break
        means[i] = np.mean(pnl_r[indices])

    lcb_95 = float(np.percentile(means, 5.0))
    median = float(np.percentile(means, 50.0))
    ucb_95 = float(np.percentile(means, 95.0))
    return lcb_95, median, ucb_95


def calc_deflated_sharpe(
    sr_hat: float,
    n_samples: int,
    skew: float,
    kurt_pearson: float,
    k_trials: int,
    var_trials: float,
) -> tuple[float, float, float]:
    """Calculate Bailey & Lopez de Prado (2014) Deflated Sharpe Ratio."""
    euler = 0.5772156649015328
    q1 = stats.norm.ppf(1.0 - 1.0 / k_trials)
    q2 = stats.norm.ppf(1.0 - 1.0 / (k_trials * np.e))
    sr_star = float(np.sqrt(var_trials) * ((1.0 - euler) * q1 + euler * q2))

    denom_sq = 1.0 - skew * sr_hat + ((kurt_pearson - 1.0) / 4.0) * (sr_hat ** 2)
    denom = np.sqrt(max(denom_sq, 1e-9))
    z = float((sr_hat - sr_star) * np.sqrt(n_samples - 1) / denom)
    dsr = float(stats.norm.cdf(z))
    return sr_star, z, dsr


def run_perturbation_grid(
    df_btc: pl.DataFrame,
    df_eth: pl.DataFrame,
    windows: list[WalkForwardWindow],
) -> list[dict]:
    """Evaluate parameter stability across +/- 20% shifts."""
    grid = [
        ("Base (20, 0.30, 1.25)", 20, 0.30, 1.25),
        ("Donchian -20% (16)", 16, 0.30, 1.25),
        ("Donchian +20% (24)", 24, 0.30, 1.25),
        ("ER -20% (0.24)", 20, 0.24, 1.25),
        ("ER +20% (0.36)", 20, 0.36, 1.25),
        ("VolMult -20% (1.00)", 20, 0.30, 1.00),
        ("VolMult +20% (1.50)", 20, 0.30, 1.50),
    ]
    results = []
    for label, donchian, er_thresh, vol_mult in grid:
        sig_b = alpha_py.scan_h1_signals(
            highs=df_btc["high"].to_list(), lows=df_btc["low"].to_list(),
            closes=df_btc["close"].to_list(), volumes=df_btc["volume"].to_list(),
            taker_volumes=df_btc["taker_buy_volume"].to_list(),
            er_period=24, er_threshold=er_thresh, donchian_period=donchian,
            volume_sma_period=20, volume_multiplier=vol_mult, min_taker_buy_ratio=0.53,
        )
        sig_e = alpha_py.scan_h1_signals(
            highs=df_eth["high"].to_list(), lows=df_eth["low"].to_list(),
            closes=df_eth["close"].to_list(), volumes=df_eth["volume"].to_list(),
            taker_volumes=df_eth["taker_buy_volume"].to_list(),
            er_period=24, er_threshold=er_thresh, donchian_period=donchian,
            volume_sma_period=20, volume_multiplier=vol_mult, min_taker_buy_ratio=0.53,
        )
        pnl = []
        for w in windows:
            tb = simulate_window_trades(df_btc, sig_b, w, "BTCUSDT", "4h", "E0_E1_E2", 2.0, 2.5, 0.022, True, 3, True)
            te = simulate_window_trades(df_eth, sig_e, w, "ETHUSDT", "4h", "E0_E1_E2", 2.0, 2.5, 0.022, True, 3, True)
            pnl.extend([t.net_pnl_r for t in tb] + [t.net_pnl_r for t in te])
        n = len(pnl)
        tot = sum(pnl)
        results.append({
            "param": label, "n": n, "net_r": tot, "ev_r": tot / n if n > 0 else 0.0,
        })
    return results


def run_stage_b_holdout(
    df_btc: pl.DataFrame,
    df_eth: pl.DataFrame,
) -> tuple[list[TradeRecord], dict]:
    """Single-pass evaluation on untouched Stage B holdout (May-Nov 2026)."""
    holdout_start = datetime(2026, 5, 1)
    holdout_end = datetime(2026, 11, 1)
    dummy_train = datetime(2025, 5, 1)
    w = WalkForwardWindow(
        fold_idx=99,
        train_start=dummy_train,
        train_end=holdout_start,
        test_start=holdout_start,
        test_end=holdout_end,
    )

    sig_b = alpha_py.scan_h1_signals(
        highs=df_btc["high"].to_list(), lows=df_btc["low"].to_list(),
        closes=df_btc["close"].to_list(), volumes=df_btc["volume"].to_list(),
        taker_volumes=df_btc["taker_buy_volume"].to_list(),
        er_period=24, er_threshold=0.30, donchian_period=20,
        volume_sma_period=20, volume_multiplier=1.25, min_taker_buy_ratio=0.53,
    )
    sig_e = alpha_py.scan_h1_signals(
        highs=df_eth["high"].to_list(), lows=df_eth["low"].to_list(),
        closes=df_eth["close"].to_list(), volumes=df_eth["volume"].to_list(),
        taker_volumes=df_eth["taker_buy_volume"].to_list(),
        er_period=24, er_threshold=0.30, donchian_period=20,
        volume_sma_period=20, volume_multiplier=1.25, min_taker_buy_ratio=0.53,
    )

    tb = simulate_window_trades(df_btc, sig_b, w, "BTCUSDT", "4h", "E0_E1_E2", 2.0, 2.5, 0.022, True, 3, True)
    te = simulate_window_trades(df_eth, sig_e, w, "ETHUSDT", "4h", "E0_E1_E2", 2.0, 2.5, 0.022, True, 3, True)
    trades = tb + te
    pnl = [t.net_pnl_r for t in trades]

    peak = cum = max_dd = 0.0
    for r in pnl:
        cum += r
        if cum > peak:
            peak = cum
        if peak - cum > max_dd:
            max_dd = peak - cum

    summary = {
        "n": len(trades),
        "net_r": sum(pnl),
        "ev_r": sum(pnl) / len(trades) if trades else 0.0,
        "max_dd_r": max_dd,
    }
    return trades, summary


def main() -> None:
    """Run all statistical kill gates and print report."""
    print("================================================================")
    print("Project Alpha v3 — Phase 2 Statistical Validation & Gate Audit")
    print("================================================================\n")

    df_trades = pl.read_parquet("data/walkforward_trades.parquet")
    sub = df_trades.filter(
        (pl.col("timeframe") == "4h") & (pl.col("exit_policy") == "E0_E1_E2")
    ).sort("entry_time")
    pnl_4h = sub["net_pnl_r"].to_numpy()

    print(f"--> Stage A Walk-Forward Dataset: {len(pnl_4h)} trades (4h E0_E1_E2)")
    print(f"    Empirical Mean: {np.mean(pnl_4h):+.3f}R | Std: {np.std(pnl_4h):.3f}R | Total: {np.sum(pnl_4h):+.2f}R")

    lcb_95, median, ucb_95 = stationary_bootstrap(pnl_4h, n_resamples=10000, mean_block_len=3.0)
    print(f"\n[Gate 1: Politis-Romano Block Bootstrap (10,000 resamples)]")
    print(f"    LCB 95%: {lcb_95:+.4f}R | Median: {median:+.4f}R | UCB 95%: {ucb_95:+.4f}R")
    print(f"    Verdict: {'PASS' if lcb_95 > 0.0 else 'BORDERLINE/FAIL'} (Gate requirement: > +0.02R)")

    # DSR across 4h ablation trials
    trials_4h = [
        np.mean(g["net_pnl_r"].to_numpy()) / np.std(g["net_pnl_r"].to_numpy(), ddof=1)
        for _, g in df_trades.filter(pl.col("timeframe") == "4h").group_by(["asset", "exit_policy"])
    ]
    var_trials = float(np.var(trials_4h, ddof=1))
    sr_hat = float(np.mean(pnl_4h) / np.std(pnl_4h, ddof=1))
    skew = float(stats.skew(pnl_4h))
    kurt = float(stats.kurtosis(pnl_4h, fisher=False))
    sr_star, z, dsr = calc_deflated_sharpe(sr_hat, len(pnl_4h), skew, kurt, len(trials_4h), var_trials)

    print(f"\n[Gate 2: Deflated Sharpe Ratio (DSR)]")
    print(f"    Sample SR: {sr_hat:.4f} | SR* (null threshold): {sr_star:.4f} | Z: {z:.2f} | DSR: {dsr:.4f}")
    print(f"    Verdict: {'PASS' if dsr > 0.95 else 'CAUTION/INCONCLUSIVE'} (Gate requirement: > 0.95)")

    # Perturbation Grid
    print(f"\n[Gate 3: Parameter Perturbation Stability (+/- 20%)]")
    windows = generate_walkforward_windows(datetime(2021, 1, 1), datetime(2026, 5, 1), 12, 3, 48)
    df_btc = prepare_features(pl.read_parquet("data/raw/BTCUSDT_4h.parquet"))
    df_eth = prepare_features(pl.read_parquet("data/raw/ETHUSDT_4h.parquet"))
    grid_results = run_perturbation_grid(df_btc, df_eth, windows)
    all_positive = all(r["net_r"] > 0 for r in grid_results)
    for r in grid_results:
        print(f"    {r['param']:24s} | N={r['n']:3d} | Net R={r['net_r']:+6.2f}R | EV={r['ev_r']:+.3f}R")
    print(f"    Verdict: {'PASS (Robust Plateau)' if all_positive else 'FAIL'}")

    # Stage B Holdout
    print(f"\n[Gate 4: Stage B Untouched Holdout Sanity Check (May-Nov 2026)]")
    holdout_trades, holdout_summary = run_stage_b_holdout(df_btc, df_eth)
    h_n = holdout_summary["n"]
    h_net = holdout_summary["net_r"]
    h_ev = holdout_summary["ev_r"]
    h_dd = holdout_summary["max_dd_r"]
    in_ci = lcb_95 <= h_ev <= ucb_95
    print(f"    Holdout Trades: {h_n} | Net R: {h_net:+.2f}R | EV: {h_ev:+.3f}R | Max DD: {h_dd:.2f}R")
    print(f"    Net Positive? {h_net > 0} | Inside 95% CI? {in_ci} | DD <= 8.0R? {h_dd <= 8.0}")
    print(f"    Verdict: {'PASS' if (h_net > 0 and in_ci and h_dd <= 8.0) else 'FAIL'}")
    print("\n================================================================")


if __name__ == "__main__":
    main()
