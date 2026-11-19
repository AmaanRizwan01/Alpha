"""Binance Vision Historical Klines Downloader for Project Alpha.

Downloads monthly spot kline archives from data.binance.vision,
extracts and cleans klines with taker buy volume, normalizes timestamp
precision (auto-detecting ms vs us), verifies temporal integrity,
and saves out partitioned Parquet datasets.
"""

from __future__ import annotations

import argparse
import io
import sys
import urllib.error
import urllib.request
import zipfile
from pathlib import Path
from typing import List, Optional

import polars as pl

KLINE_COLUMNS = [
    "open_time",
    "open",
    "high",
    "low",
    "close",
    "volume",
    "close_time",
    "quote_volume",
    "trades_count",
    "taker_buy_volume",
    "taker_buy_quote_volume",
    "ignore",
]


def build_monthly_url(symbol: str, interval: str, year: int, month: int) -> str:
    """Build the official data.binance.vision archive URL."""
    base = "https://data.binance.vision/data/spot/monthly/klines"
    file_name = f"{symbol}-{interval}-{year}-{month:02d}.zip"
    return f"{base}/{symbol}/{interval}/{file_name}"


def fetch_monthly_zip(url: str) -> Optional[bytes]:
    """Download zip archive bytes from Binance Vision, returning None on 404."""
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "ProjectAlpha-Research/1.0"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            if resp.status == 200:
                return resp.read()
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            return None
        raise
    return None


def parse_csv_from_zip(content: bytes) -> pl.DataFrame:
    """Extract, parse, and normalize klines CSV from an in-memory zip archive."""
    with zipfile.ZipFile(io.BytesIO(content)) as zf:
        csv_name = next(n for n in zf.namelist() if n.endswith(".csv"))
        with zf.open(csv_name) as csv_file:
            raw_df = pl.read_csv(
                csv_file.read(),
                has_header=False,
                new_columns=KLINE_COLUMNS,
                infer_schema_length=1000,
            )

    # Filter out header if archive contained one
    clean_df = raw_df.filter(pl.col("open_time").cast(pl.Utf8) != "open_time")

    # Normalize timestamp precision (older files use ms [13 digits], newer use us [16 digits])
    normalized = clean_df.with_columns(
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

    return normalized.select(
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


def validate_dataset(df: pl.DataFrame, symbol: str, interval: str) -> pl.DataFrame:
    """Verify temporal causality, strictly monotonic timestamps, and valid OHLC."""
    if df.is_empty():
        raise ValueError(f"dataset for {symbol} {interval} is empty")

    sorted_df = df.sort("open_time").unique(subset=["open_time"], keep="first")

    # Time monotonicity check
    time_diffs = sorted_df["open_time"].diff().drop_nulls()
    if (time_diffs <= 0).any():
        raise ValueError(f"detected non-monotonic open_time for {symbol} {interval}")

    # OHLC validity checks
    invalid_ohlc = sorted_df.filter(
        (pl.col("low") > pl.col("high"))
        | (pl.col("low") > pl.col("open"))
        | (pl.col("low") > pl.col("close"))
        | (pl.col("high") < pl.col("open"))
        | (pl.col("high") < pl.col("close"))
        | (pl.col("open") <= 0)
        | (pl.col("close") <= 0)
    )
    if not invalid_ohlc.is_empty():
        raise ValueError(f"found {len(invalid_ohlc)} bars with invalid OHLC geometry")

    return sorted_df.with_columns(
        pl.from_epoch(pl.col("open_time"), time_unit="ms").alias("datetime")
    )


def download_symbol_history(
    symbol: str,
    interval: str,
    start_year: int,
    end_year: int,
    end_month: int,
    output_dir: Path,
) -> Path:
    """Download all monthly chunks for a symbol and interval into Parquet."""
    frames: List[pl.DataFrame] = []
    total_months = 0

    for year in range(start_year, end_year + 1):
        max_m = end_month if year == end_year else 12
        for month in range(1, max_m + 1):
            url = build_monthly_url(symbol, interval, year, month)
            data = fetch_monthly_zip(url)
            if data is None:
                print(f"[-] {symbol} {interval} {year}-{month:02d}: not found (404)")
                continue

            frames.append(parse_csv_from_zip(data))
            total_months += 1
            print(f"[+] {symbol} {interval} {year}-{month:02d}: downloaded & parsed")

    if not frames:
        raise RuntimeError(f"no data downloaded for {symbol} {interval}")

    combined = pl.concat(frames)
    validated = validate_dataset(combined, symbol, interval)

    output_dir.mkdir(parents=True, exist_ok=True)
    out_file = output_dir / f"{symbol}_{interval}.parquet"
    validated.write_parquet(out_file, compression="zstd")

    first_dt = validated["datetime"][0]
    last_dt = validated["datetime"][-1]
    print(f"\n[OK] Saved {len(validated):,} bars to {out_file}")
    print(f"     Range: {first_dt} -> {last_dt} ({total_months} months)\n")
    return out_file


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Binance Vision Klines Downloader")
    parser.add_argument("--symbols", default="BTCUSDT,ETHUSDT", help="Comma-separated symbols")
    parser.add_argument("--intervals", default="1h,4h", help="Comma-separated intervals")
    parser.add_argument("--start-year", type=int, default=2021)
    parser.add_argument("--end-year", type=int, default=2026)
    parser.add_argument("--end-month", type=int, default=10)
    parser.add_argument("--output-dir", default="data/raw", help="Target output directory")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    symbols = [s.strip().upper() for s in args.symbols.split(",") if s.strip()]
    intervals = [i.strip().lower() for i in args.intervals.split(",") if i.strip()]
    out_dir = Path(args.output_dir)

    print(f"=== Project Alpha Binance Vision Downloader ===")
    print(f"Symbols: {symbols} | Intervals: {intervals}")
    print(f"Window: {args.start_year}-01 to {args.end_year}-{args.end_month:02d}\n")

    for symbol in symbols:
        for interval in intervals:
            download_symbol_history(
                symbol=symbol,
                interval=interval,
                start_year=args.start_year,
                end_year=args.end_year,
                end_month=args.end_month,
                output_dir=out_dir,
            )

    print("[SUCCESS] All historical datasets downloaded and verified.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
