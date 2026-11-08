import os
import polars as pl
import pytest
import alpha_py


def test_kaufman_er_monotonic():
    # perfectly monotonic prices must yield ER = 1.0
    prices = [100.0 + float(i) for i in range(30)]
    er = alpha_py.kaufman_er(prices, 24)
    assert abs(er - 1.0) < 1e-4


def test_donchian_channel():
    highs = [10.0 + float(i) for i in range(25)]
    lows = [5.0 + float(i) for i in range(25)]
    hh, ll = alpha_py.donchian(highs, lows, 20)
    assert hh == 34.0
    assert ll == 10.0


def test_atr_positive():
    highs = [105.0] * 20
    lows = [95.0] * 20
    closes = [100.0] * 20
    val = alpha_py.atr(highs, lows, closes, 14)
    assert val > 0.0
    assert abs(val - 10.0) < 1e-4


def test_calc_position_size_scale_invariant():
    # $100 capital, 1% risk ($1.00 risk budget)
    # entry 50,000, stop 48,500 (1,500 stop distance = 3%)
    # fees: 0 maker, 0.001 taker, 0.0005 slippage
    # filter: tick=0.01, step=0.00001, min_qty=0.00001, min_notional=5.0
    size = alpha_py.calc_position_size(
        equity=100.0,
        risk_fraction=0.01,
        raw_entry=50000.0,
        raw_stop=48500.0,
        maker_fee=0.0,
        taker_fee=0.001,
        slippage=0.0005,
        tick_size=0.01,
        step_size=0.00001,
        min_qty=0.00001,
        min_notional=5.0,
        min_stop_pct=0.022,
    )
    assert size.qty > 0.0
    assert size.notional >= 5.0
    assert size.stop_distance_pct >= 0.022
    assert size.risk_quote <= 1.05


def test_evaluate_chop_trend_pass():
    # trend with large moves and high ER should pass
    closes = [100.0 + float(i * 2) for i in range(35)]
    highs = [c + 1.0 for c in closes]
    lows = [c - 1.0 for c in closes]
    verdict = alpha_py.evaluate_chop(
        highs=highs,
        lows=lows,
        closes=closes,
        er_period=24,
        er_threshold=0.30,
        atr_period=14,
        min_atr_pct=0.009,
    )
    assert verdict is None  # Pass


def test_parquet_data_integration():
    parquet_path = "data/raw/BTCUSDT_1h.parquet"
    if not os.path.exists(parquet_path):
        pytest.skip(f"{parquet_path} not found")

    df = pl.read_parquet(parquet_path)
    assert len(df) > 1000
    closes = df["close"].head(100).to_list()
    highs = df["high"].head(100).to_list()
    lows = df["low"].head(100).to_list()

    er = alpha_py.kaufman_er(closes, 24)
    assert 0.0 <= er <= 1.0

    atr_val = alpha_py.atr(highs, lows, closes, 14)
    assert atr_val > 0.0
