"""Unit tests for multi-timeframe backward-asof alignment primitive."""

from datetime import datetime, timedelta
import polars as pl
import pytest

from python.research.primitives.multi_timeframe import align_multi_timeframe


def test_align_multi_timeframe_causality():
    """Verify that lower-timeframe bars never see future higher-timeframe data."""
    base_time = datetime(2025, 1, 1, 0, 0)
    
    # 15m higher timeframe bars: 00:00, 00:15, 00:30, 00:45
    htf_data = {
        "timestamp": [base_time + timedelta(minutes=15 * i) for i in range(4)],
        "htf_signal": [10.0, 20.0, 30.0, 40.0],
    }
    htf_df = pl.DataFrame(htf_data)

    # 1m lower timeframe bars: 00:00 through 00:35
    ltf_data = {
        "timestamp": [base_time + timedelta(minutes=i) for i in range(36)],
        "ltf_price": [100.0 + i for i in range(36)],
    }
    ltf_df = pl.DataFrame(ltf_data)

    aligned = align_multi_timeframe(ltf_df, htf_df, prefix="htf_")

    # Bar at 00:14 should only see HTF bar from 00:00 (value 10.0)
    bar_14 = aligned.filter(pl.col("timestamp") == base_time + timedelta(minutes=14))
    assert bar_14["htf_htf_signal"][0] == 10.0

    # Bar at 00:15 sees the newly closed bar from 00:15 (value 20.0)
    bar_15 = aligned.filter(pl.col("timestamp") == base_time + timedelta(minutes=15))
    assert bar_15["htf_htf_signal"][0] == 20.0

    # Bar at 00:29 still sees value 20.0 (no forward peeking to 00:30)
    bar_29 = aligned.filter(pl.col("timestamp") == base_time + timedelta(minutes=29))
    assert bar_29["htf_htf_signal"][0] == 20.0
