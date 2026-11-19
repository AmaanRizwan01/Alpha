from datetime import datetime, timedelta
import polars as pl
import pytest
import alpha_py
from python.research.walkforward import (
    add_months,
    generate_walkforward_windows,
    prepare_features,
    simulate_window_trades,
)


def test_add_months():
    dt = datetime(2021, 1, 1, 0, 0)
    assert add_months(dt, 3) == datetime(2021, 4, 1, 0, 0)
    assert add_months(dt, 12) == datetime(2022, 1, 1, 0, 0)
    assert add_months(dt, 15) == datetime(2022, 4, 1, 0, 0)


def test_generate_walkforward_windows():
    start = datetime(2021, 1, 1)
    holdout = datetime(2026, 5, 1)
    windows = generate_walkforward_windows(start, holdout, train_months=12, test_months=3)

    assert len(windows) == 18
    assert windows[0].fold_idx == 0
    assert windows[0].train_start == datetime(2021, 1, 1)
    assert windows[0].train_end == datetime(2022, 1, 1)
    # 48h purge gap
    assert windows[0].test_start == datetime(2022, 1, 3, 0, 0)
    assert windows[0].test_end == datetime(2022, 4, 1)

    # Final fold touches holdout boundary
    assert windows[-1].test_end == holdout


def test_prepare_features_atr():
    df = pl.read_parquet("data/raw/BTCUSDT_1h.parquet").head(100)
    df_feat = prepare_features(df)

    assert "atr" in df_feat.columns
    assert "tr" in df_feat.columns
    # Nulls are filled with 0.0
    assert df_feat["atr"].null_count() == 0
    # Values after warmup period are strictly positive
    assert (df_feat["atr"].slice(14) > 0.0).all()


def test_scale_invariant_fee_in_r():
    # Structural stop at 2.50% (0.025), all-in roundtrip friction 23 bps (0.0023)
    entry = 100.0
    stop = 97.50
    stop_dist_pct = (entry - stop) / entry  # 0.025
    c_rt = 0.0008 + 0.0010 + 0.0005  # 0.0023
    fee_r = c_rt / stop_dist_pct
    assert pytest.approx(fee_r, rel=1e-4) == 0.092  # 0.092R fee drag


def test_simulate_window_trades_smoke():
    df = pl.read_parquet("data/raw/BTCUSDT_1h.parquet").head(15000)
    df = prepare_features(df)

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

    windows = generate_walkforward_windows(
        datetime(2021, 1, 1), datetime(2026, 5, 1), train_months=12, test_months=3
    )

    trades = simulate_window_trades(
        df=df,
        signals=signals,
        window=windows[0],
        asset="BTCUSDT",
        timeframe="1h",
        exit_policy="E0",
    )

    assert len(trades) > 0, "Expected at least 1 trade in Fold 0"
    for t in trades:
        assert t.entry_price > 0.0
        assert t.exit_price > 0.0
        assert t.exit_reason in [
            "StopLossHit",
            "TrailingStopHit",
            "StagnationTimeStop",
            "BreakevenStopHit",
            "Open",
        ]
        assert t.fee_r > 0.0
        assert pytest.approx(t.net_pnl_r, rel=1e-4) == t.gross_pnl_r - t.fee_r
