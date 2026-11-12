"""Unit & integration tests for multi-strategy engine and funding carry safeguards."""

from datetime import datetime, timedelta
from pathlib import Path
import ast
import numpy as np
import polars as pl
import pytest

from python.research.strategies import (
    BaseStrategy, StrategyPerformance, get_available_strategies, load_all_strategies
)
from python.research.strategies.base import Trade
from python.research.compare_strategies import align_equity_returns, format_markdown_matrix

DATA_DIR = Path(__file__).resolve().parents[2] / "data" / "raw"


def test_dynamic_discovery():
    classes = get_available_strategies()
    assert set(classes.keys()) == {"VOLATILITY_BREAKOUT", "FUNDING_CARRY", "MACRO_TREND"}


def test_all_strategies_instantiate():
    strategies = load_all_strategies()
    assert len(strategies) == 3
    for s in strategies:
        assert isinstance(s, BaseStrategy) and s.strategy_id and s.strategy_name


@pytest.mark.parametrize("strat_id", ["VOLATILITY_BREAKOUT", "FUNDING_CARRY", "MACRO_TREND"])
def test_strategy_execution_integrity(strat_id: str):
    strat = get_available_strategies()[strat_id]()
    perf = strat.run_backtest(DATA_DIR)
    assert isinstance(perf, StrategyPerformance) and perf.trade_count > 0
    for val in [perf.ev_net_r, perf.profit_factor, perf.win_rate_pct, perf.sharpe_ratio, perf.max_drawdown_r]:
        assert np.isfinite(val)
    assert 0.0 <= perf.win_rate_pct <= 100.0 and 0.0 <= perf.market_exposure_pct <= 100.0


def test_plug_and_play_deletion_resilience():
    remaining = [s for s in load_all_strategies() if s.strategy_id != "FUNDING_CARRY"]
    assert len(remaining) == 2
    table = format_markdown_matrix([s.run_backtest(DATA_DIR) for s in remaining])
    assert "VOLATILITY_BREAKOUT" in table and "MACRO_TREND" in table and "FUNDING_CARRY" not in table


def test_volatility_breakout_fee_drag_accounting():
    perf = get_available_strategies()["VOLATILITY_BREAKOUT"]().run_backtest(DATA_DIR)
    assert perf.fee_drag_r > 0.0 and perf.ev_net_r < perf.ev_gross_r


def test_funding_carry_delta_neutral_scaling():
    carry = get_available_strategies()["FUNDING_CARRY"]()
    assert carry.fee_hurdle_mult == 1.0
    perf = carry.run_backtest(DATA_DIR)
    assert perf.trade_count > 0
    for t in carry.get_trades():
        assert t.gross_pnl_r > 0 and t.fee_r > 0 and t.net_pnl_r < t.gross_pnl_r
        assert t.side == "DELTA_NEUTRAL"


def test_funding_carry_fee_hurdle_lock_prevents_decay_loss():
    carry = get_available_strategies()["FUNDING_CARRY"](fee_hurdle_mult=1.0)
    carry.run_backtest(DATA_DIR)
    decay_trades = [t for t in carry.get_trades() if t.exit_reason == "FundingDecayExit"]
    assert len(decay_trades) > 0
    for t in decay_trades:
        assert t.net_pnl_r >= 0.0, f"Decay exit took net loss: {t.net_pnl_r:+.4f}R"
        assert t.net_return_pct >= 0.0 and t.gross_return_pct >= t.fee_r * carry.r_unit


def test_funding_carry_synthetic_fee_hurdle_and_inversion():
    carry_cls = get_available_strategies()["FUNDING_CARRY"]
    start = datetime(2024, 1, 1, 0, 0)
    # 1. Unlocked vs Locked when gross funding < fees
    rates = [0.00012] + [0.000005] * 12 + [-0.00020]
    times = [start + timedelta(hours=8 * i) for i in range(len(rates))]
    df = pl.DataFrame({"datetime": times, "funding_rate": rates, "calc_time": [t.timestamp() for t in times]})

    t_unlocked, _ = carry_cls(fee_hurdle_mult=0.0)._simulate_symbol_carry("BTCUSDT", df)
    assert len(t_unlocked) == 1 and t_unlocked[0].exit_reason == "FundingDecayExit"
    assert t_unlocked[0].net_pnl_r < 0.0

    locked = carry_cls(fee_hurdle_mult=1.0)
    t_locked, _ = locked._simulate_symbol_carry("BTCUSDT", df)
    assert len(t_locked) == 1 and t_locked[0].exit_reason == "FundingInversionStop"
    assert t_locked[0].exit_time == times[13]

    # 2. Locked with fees covered: unlocks FundingDecayExit with net profit
    rates_cov = [0.00040] * 10 + [0.000005] * 20
    times_cov = [start + timedelta(hours=8 * i) for i in range(len(rates_cov))]
    df_cov = pl.DataFrame({"datetime": times_cov, "funding_rate": rates_cov, "calc_time": [t.timestamp() for t in times_cov]})
    t_cov, _ = locked._simulate_symbol_carry("BTCUSDT", df_cov)
    assert len(t_cov) == 1 and t_cov[0].exit_reason == "FundingDecayExit"
    assert t_cov[0].net_pnl_r > 0.0


def test_macro_trend_cross_asset_execution():
    macro = get_available_strategies()["MACRO_TREND"]()
    perf = macro.run_backtest(DATA_DIR)
    assert perf.trade_count > 0
    valid_reasons = {"TrailingStop", "TrendInvalidation", "VixCircuitBreaker", "EndOfSample"}
    for t in macro.get_trades():
        assert t.exit_reason in valid_reasons and abs(t.gross_return_pct) < 5.0


def test_all_strategies_positive_bars_held():
    for strat in load_all_strategies():
        strat.run_backtest(DATA_DIR)
        for t in strat.get_trades():
            assert t.bars_held >= 1, f"Invalid bars_held in {strat.strategy_id}"


def test_equity_correlation_alignment():
    dt_series = [datetime(2023, 1, i) for i in range(1, 20)]
    cum_r = list(np.cumsum([float(i % 3) for i in range(1, 20)]))
    df = pl.DataFrame({"datetime": dt_series, "cumulative_net_r": cum_r})
    assert np.isclose(align_equity_returns(df, df), 1.0, atol=1e-3)


def test_sparse_correlation_alignment():
    df1 = pl.DataFrame({"datetime": [datetime(2023, 1, 1), datetime(2023, 1, 15)], "cumulative_net_r": [1.0, 2.0]})
    df2 = pl.DataFrame({"datetime": [datetime(2023, 1, 5), datetime(2023, 1, 20)], "cumulative_net_r": [0.5, 1.5]})
    assert np.isfinite(align_equity_returns(df1, df2))


def test_strategy_independence():
    classes = get_available_strategies()
    p1 = classes["VOLATILITY_BREAKOUT"]().run_backtest(DATA_DIR)
    classes["FUNDING_CARRY"]().run_backtest(DATA_DIR)
    p2 = classes["VOLATILITY_BREAKOUT"]().run_backtest(DATA_DIR)
    assert p1.trade_count == p2.trade_count and np.isclose(p1.ev_net_r, p2.ev_net_r)


def test_plug_and_play_deletion_benchmark():
    remaining = [s for s in load_all_strategies() if s.strategy_id != "VOLATILITY_BREAKOUT"]
    table = format_markdown_matrix([s.run_backtest(DATA_DIR) for s in remaining])
    assert "VOLATILITY_BREAKOUT" not in table and "FUNDING_CARRY" in table


def test_zero_cross_strategy_dependencies_ast():
    strat_dir = Path(__file__).resolve().parents[1] / "research" / "strategies"
    strat_files = [f for f in strat_dir.glob("*.py") if f.name not in ("base.py", "__init__.py")]
    strat_names = {f.stem for f in strat_files}

    for f in strat_files:
        tree = ast.parse(f.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.ImportFrom) and node.module:
                for s in strat_names:
                    assert not (s in node.module and s != f.stem), f"Cross-import in {f.name}"
            elif isinstance(node, ast.Import):
                for alias in node.names:
                    for s in strat_names:
                        assert not (s in alias.name and s != f.stem), f"Cross-import in {f.name}"


def test_macro_trend_missing_asset_graceful(tmp_path):
    macro = get_available_strategies()["MACRO_TREND"]()
    assert macro.run_backtest(tmp_path).trade_count == 0
    btc_src = DATA_DIR / "BTCUSDT_4h.parquet"
    if btc_src.exists():
        import shutil
        shutil.copy(btc_src, tmp_path / "BTCUSDT_4h.parquet")
        assert macro.run_backtest(tmp_path).trade_count >= 0


def test_all_trades_entry_exit_invariants():
    for s in load_all_strategies():
        s.run_backtest(DATA_DIR)
        for t in s.get_trades():
            assert t.entry_time <= t.exit_time and t.net_pnl_r <= t.gross_pnl_r + 1e-9
            assert t.entry_price > 0.0 and t.exit_price > 0.0


def test_format_markdown_matrix_kill_gate_enforcement():
    perf = StrategyPerformance(
        strategy_id="FAIL_STRAT", strategy_name="Failing", category="Arb",
        primary_asset="BTC", timeframe="8h", trade_count=50, win_rate_pct=20.0,
        ev_net_r=-0.150, ev_gross_r=-0.050, fee_drag_r=0.100, total_net_r=-7.50,
        profit_factor=0.40, sharpe_ratio=-1.20, sortino_ratio=-1.50,
        max_drawdown_r=15.0, max_drawdown_pct=15.0, market_exposure_pct=10.0,
        avg_bars_held=5.0, annualized_return_pct=-3.0,
    )
    table = format_markdown_matrix([perf])
    assert "KILL" in table and "PASS" not in table
