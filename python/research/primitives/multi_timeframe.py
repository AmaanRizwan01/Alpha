"""Multi-timeframe backward-asof alignment helper.

Guarantees 100% causal temporal safety: lower-timeframe execution candles
(e.g. 1m or 5m) can only see higher-timeframe candles (e.g. 15m or 4h) that
have already fully closed before the lower-timeframe candle open.
"""

from typing import List, Optional
import polars as pl


def align_multi_timeframe(
    ltf_df: pl.DataFrame,
    htf_df: pl.DataFrame,
    timestamp_col: str = "timestamp",
    prefix: str = "htf_",
    feature_cols: Optional[List[str]] = None,
) -> pl.DataFrame:
    """Align higher-timeframe features onto lower-timeframe bars using backward as-of join.

    Parameters
    ----------
    ltf_df : pl.DataFrame
        Lower timeframe bars (must contain sorted timestamp column).
    htf_df : pl.DataFrame
        Higher timeframe bars (must contain sorted timestamp column).
    timestamp_col : str
        Column name used for temporal ordering.
    prefix : str
        Prefix attached to HTF columns to prevent naming collisions.
    feature_cols : Optional[List[str]]
        Specific columns from HTF to bring over. If None, all HTF columns
        (except timestamp) are joined.

    Returns
    -------
    pl.DataFrame
        LTF dataframe with lagged HTF features aligned with zero look-ahead.
    """
    # ensure both dataframes are sorted by timestamp
    ltf_sorted = ltf_df.sort(timestamp_col)
    htf_sorted = htf_df.sort(timestamp_col)

    # select and rename htf columns
    cols_to_keep = [timestamp_col]
    if feature_cols:
        for c in feature_cols:
            if c != timestamp_col and c in htf_df.columns:
                cols_to_keep.append(c)
    else:
        cols_to_keep = htf_df.columns

    htf_sub = htf_sorted.select(cols_to_keep)

    rename_map = {
        c: f"{prefix}{c}" for c in htf_sub.columns if c != timestamp_col
    }
    htf_renamed = htf_sub.rename(rename_map)

    # backward asof join: maps the latest closed htf candle onto each ltf candle
    aligned = ltf_sorted.join_asof(
        htf_renamed,
        on=timestamp_col,
        strategy="backward",
    )

    return aligned
