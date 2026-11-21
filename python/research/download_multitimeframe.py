"""Binance Vision Multi-Timeframe Historical Downloader & Aggregator.

Downloads 1-minute historical spot archives from data.binance.vision for BTC,
ETH, and SOL, normalizes timestamps, verifies integrity, and resamples into
5m, 15m, 30m, 1h, 4h, 1d, 1w, and 1M Parquets using Polars.
"""

from __future__ import annotations

import io
import sys
import urllib.error
import urllib.request
import zipfile
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import List, Optional, Tuple

import polars as pl

KLINE_COLUMNS = [
    "open_time", "open", "high", "low", "close", "volume",
    "close_time", "quote_volume", "trades_count",
    "taker_buy_volume", "taker_buy_quote_volume", "ignore",
]

RESAMPLE_SPECS: List[Tuple[str, str]] = [
    ("5m", "5m"),
    ("15m", "15m"),
    ("30m", "30m"),
    ("1h", "1h"),
    ("4h", "4h"),
    ("1d", "1d"),
    ("1w", "1w"),
    ("1mo", "1mo"),
]


def fetch_monthly_zip(symbol: str, year: int, month: int) -> Optional[bytes]:
    """Fetch monthly zip bytes from data.binance.vision with retries."""
    base = "https://data.binance.vision/data/spot/monthly/klines"
    url = f"{base}/{symbol}/1m/{symbol}-1m-{year}-{month:02d}.zip"
    req = urllib.request.Request(url, headers={"User-Agent": "ProjectAlpha/2.0"})

    for _ in range(3):
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                if resp.status == 200:
                    return resp.read()
        except urllib.error.HTTPError as exc:
            if exc.code == 404:
                return None
        except Exception:
            pass
    return None


def parse_zip_content(content: bytes) -> pl.DataFrame:
    """Parse in-memory zip CSV and normalize timestamp precision."""
    with zipfile.ZipFile(io.BytesIO(content)) as zf:
        csv_name = next(n for n in zf.namelist() if n.endswith(".csv"))
        with zf.open(csv_name) as csv_file:
            raw_df = pl.read_csv(
                csv_file.read(),
                has_header=False,
                new_columns=KLINE_COLUMNS,
                infer_schema_length=500,
            )

    clean_df = raw_df.filter(pl.col("open_time").cast(pl.Utf8) != "open_time")
    norm_df = clean_df.with_columns(
        pl.col("open_time").cast(pl.Int64),
        pl.col("close_time").cast(pl.Int64),
    ).with_columns(
        pl.when(pl.col("open_time") > 10**14)
        .then(pl.col("open_time") // 1000)
        .otherwise(pl.col("open_time"))
        .alias("open_time"),
        pl.when(pl.col("close_time") > 10**14)
        .then(pl.col("close_time") // 1000)
        .otherwise(pl.col("close_time"))
        .alias("close_time"),
    )

    return norm_df.select(
        pl.col("open_time").cast(pl.Int64),
        pl.col("open").cast(pl.Float64),
        pl.col("high").cast(pl.Float64),
        pl.col("low").cast(pl.Float64),
        pl.col("close").cast(pl.Float64),
        pl.col("volume").cast(pl.Float64),
        pl.col("close_time").cast(pl.Int64),
        pl.col("quote_volume").cast(pl.Float64),
        pl.col("trades_count").cast(pl.Int64),
        pl.col("taker_buy_volume").cast(pl.Float64),
        pl.col("taker_buy_quote_volume").cast(pl.Float64),
    )


def validate_clean_dataset(df: pl.DataFrame, symbol: str) -> pl.DataFrame:
    """Sort, deduplicate, validate monotonicity and OHLC geometry."""
    sorted_df = df.sort("open_time").unique(subset=["open_time"], keep="first")
    diffs = sorted_df["open_time"].diff().drop_nulls()
    if (diffs <= 0).any():
        raise ValueError(f"Non-monotonic open_time detected for {symbol}")

    invalid = sorted_df.filter(
        (pl.col("low") > pl.col("high"))
        | (pl.col("low") > pl.col("open"))
        | (pl.col("low") > pl.col("close"))
        | (pl.col("high") < pl.col("open"))
        | (pl.col("high") < pl.col("close"))
        | (pl.col("open") <= 0)
        | (pl.col("close") <= 0)
    )
    if not invalid.is_empty():
        raise ValueError(f"Found {len(invalid)} geometry violations in {symbol}")

    return sorted_df.with_columns(
        pl.from_epoch(pl.col("open_time"), time_unit="ms").alias("datetime")
    )


def download_symbol_1m(
    symbol: str, start_year: int, start_month: int, end_year: int, end_month: int
) -> pl.DataFrame:
    """Download all monthly chunks in parallel and return continuous 1m DataFrame."""
    tasks: List[Tuple[int, int]] = []
    for y in range(start_year, end_year + 1):
        s_m = start_month if y == start_year else 1
        e_m = end_month if y == end_year else 12
        for m in range(s_m, e_m + 1):
            tasks.append((y, m))

    print(f"[*] {symbol}: fetching {len(tasks)} monthly archives with 8 workers...")
    frames: List[pl.DataFrame] = []

    with ThreadPoolExecutor(max_workers=8) as executor:
        futures = {
            executor.submit(fetch_monthly_zip, symbol, y, m): (y, m)
            for y, m in tasks
        }
        for future in as_completed(futures):
            y, m = futures[future]
            data = future.result()
            if data is not None:
                frames.append(parse_zip_content(data))
                print(f"    [+] {symbol} {y}-{m:02d}: downloaded & parsed")
            else:
                print(f"    [-] {symbol} {y}-{m:02d}: 404 not found (skipped)")

    if not frames:
        raise RuntimeError(f"No data downloaded for {symbol}")

    combined = pl.concat(frames)
    return validate_clean_dataset(combined, symbol)


def resample_and_save_all(df_1m: pl.DataFrame, symbol: str, out_dir: Path) -> None:
    """Save 1m and resample to 5m, 15m, 30m, 1h, 4h, 1d, 1w, 1M Parquets."""
    out_dir.mkdir(parents=True, exist_ok=True)
    p_1m = out_dir / f"{symbol}_1m.parquet"
    df_1m.write_parquet(p_1m, compression="zstd")
    print(f"[OK] Saved {len(df_1m):,} bars to {p_1m.name}")

    for tf_label, polars_rule in RESAMPLE_SPECS:
        resampled = df_1m.group_by_dynamic(
            "datetime", every=polars_rule, closed="left"
        ).agg([
            pl.col("open_time").first().alias("open_time"),
            pl.col("open").first().alias("open"),
            pl.col("high").max().alias("high"),
            pl.col("low").min().alias("low"),
            pl.col("close").last().alias("close"),
            pl.col("volume").sum().alias("volume"),
            pl.col("close_time").last().alias("close_time"),
            pl.col("quote_volume").sum().alias("quote_volume"),
            pl.col("trades_count").sum().alias("trades_count"),
            pl.col("taker_buy_volume").sum().alias("taker_buy_volume"),
            pl.col("taker_buy_quote_volume").sum().alias("taker_buy_quote_volume"),
        ])
        p_tf = out_dir / f"{symbol}_{tf_label}.parquet"
        resampled.write_parquet(p_tf, compression="zstd")
        print(f"[OK] Saved {len(resampled):,} bars to {p_tf.name} ({tf_label})")


def process_symbol(symbol: str, start_year: int, start_month: int, end_year: int, end_month: int, out_dir: Path) -> None:
    """Coordinate download, validation, and multi-timeframe generation."""
    print(f"\n==================== PROCESSING {symbol} ====================")
    df_1m = download_symbol_1m(symbol, start_year, start_month, end_year, end_month)
    first_dt = df_1m["datetime"][0]
    last_dt = df_1m["datetime"][-1]
    print(f"[+] {symbol} 1m complete: {len(df_1m):,} bars ({first_dt} -> {last_dt})")
    resample_and_save_all(df_1m, symbol, out_dir)


def main() -> int:
    """Main pipeline entrypoint for BTC, ETH, and SOL."""
    out_dir = Path("data/raw/crypto")
    end_year, end_month = 2026, 10

    # Symbol configs: (symbol, start_year, start_month)
    configs = [
        ("BTCUSDT", 2017, 8),
        ("ETHUSDT", 2017, 8),
        ("SOLUSDT", 2020, 8),
    ]

    for symbol, s_year, s_month in configs:
        process_symbol(symbol, s_year, s_month, end_year, end_month, out_dir)

    print("\n[SUCCESS] All multi-timeframe datasets downloaded, validated, and saved.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
