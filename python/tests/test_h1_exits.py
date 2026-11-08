import polars as pl
import pytest
import alpha_py


def test_h1_synthetic_signal():
    # 25 flat bars: high 102, low 98, close 100, vol 10, taker 5
    highs = [102.0] * 25
    lows = [98.0] * 25
    closes = [100.0] * 25
    vols = [10.0] * 25
    taker_vols = [5.0] * 25

    # Breakout bar: high 109, low 99, close 108 (> 102), vol 20 (> 1.25 * 10), taker 14 (70% > 53%)
    highs.append(109.0)
    lows.append(99.0)
    closes.append(108.0)
    vols.append(20.0)
    taker_vols.append(14.0)

    res = alpha_py.evaluate_h1(
        highs=highs,
        lows=lows,
        closes=closes,
        volumes=vols,
        taker_volumes=taker_vols,
        er_period=10,
        er_threshold=0.20,
        donchian_period=10,
        volume_sma_period=10,
        volume_multiplier=1.25,
        min_taker_buy_ratio=0.53,
    )

    assert res.is_signal is True
    assert res.verdict == "Signal"
    assert res.close == 108.0
    assert res.upper_band == 102.0
    assert res.volume == 20.0
    assert res.taker_buy_ratio == 0.70


def test_h1_historical_scan_btc():
    df = pl.read_parquet("data/raw/BTCUSDT_1h.parquet").head(1000)
    highs = df["high"].to_list()
    lows = df["low"].to_list()
    closes = df["close"].to_list()
    vols = df["volume"].to_list()
    taker_vols = df["taker_buy_volume"].to_list()

    signals = alpha_py.scan_h1_signals(
        highs=highs,
        lows=lows,
        closes=closes,
        volumes=vols,
        taker_volumes=taker_vols,
        er_period=24,
        er_threshold=0.30,
        donchian_period=20,
        volume_sma_period=20,
        volume_multiplier=1.25,
        min_taker_buy_ratio=0.53,
    )

    assert len(signals) == len(closes)
    # Ensure signals exist in historical data
    signal_count = sum(signals)
    assert signal_count > 0, "Expected at least 1 H1 signal in first 1000 BTC 1h bars"


def test_calc_initial_stop_enforces_floor():
    # 2.20% minimum floor
    entry = 100.0
    tiny_atr = 0.5  # 2.0 * 0.5 = 1.0 (1.0% stop, too tight)
    stop = alpha_py.calc_initial_stop(
        entry_price=entry,
        atr=tiny_atr,
        k_stop=2.0,
        min_stop_distance_pct=0.022,
    )
    # Should be clamped to 100 * (1 - 0.022) = 97.80
    assert pytest.approx(stop, rel=1e-5) == 97.80


def test_simulate_trade_exit_chandelier():
    entry = 100.0
    initial_stop = 97.0  # 1R = 3.0

    # Bar 1: surges to high 110, ATR = 2.0 => trail = 110 - 2.5 * 2 = 105.0
    # Bar 2: drops, low 104 (< 105.0) => triggers trailing stop at 105.0
    highs = [110.0, 107.0]
    lows = [100.0, 104.0]
    closes = [108.0, 104.5]
    atrs = [2.0, 2.0]

    exit_res = alpha_py.simulate_trade_exit(
        entry_price=entry,
        initial_stop=initial_stop,
        highs=highs,
        lows=lows,
        closes=closes,
        atrs=atrs,
        k_stop=2.0,
        k_trail=2.5,
        min_stop_distance_pct=0.022,
        enable_e1_stagnation=False,
        stagnation_bars=3,
        enable_e2_breakeven=False,
    )

    assert exit_res.reason == "TrailingStopHit"
    assert pytest.approx(exit_res.exit_price, rel=1e-5) == 105.0
    assert pytest.approx(exit_res.pnl_r, rel=1e-5) == (105.0 - 100.0) / 3.0
    assert exit_res.bars_held == 2
