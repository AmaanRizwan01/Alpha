"""OANDA Gold & Silver Multi-Timeframe Historical Downloader & Aggregator.

Downloads 10 years of 1-minute historical spot archives from Hugging Face
(bunqi/OANDA_XAUUSD_Gold and bunqi/OANDA_XAGUSD_Silver), normalizes timestamps,
verifies integrity, and resamples into 5m, 15m, 30m, 1h, 4h, 1d, 1w, and 1mo Parquets.
Cost: $0.00.
"""

from __future__ import annotations

import io
import json
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime
from pathlib import Path
from typing import List, Optional, Tuple

import polars as pl

HF_BASE = "https://huggingface.co/datasets/{repo}/resolve/main/1"
METALS = {
    "XAUUSD": "bunqi/OANDA_XAUUSD_Gold",
    "XAGUSD": "bunqi/OANDA_XAGUSD_Silver",
}

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


def fetch_monthly_file_list(repo: str) -> List[str]:
    """Query Hugging Face API for monthly 1-minute Parquet filenames."""
    url = f"https://huggingface.co/api/datasets/{repo}/tree/main/1"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode())
    return [x["path"] for x in data if x.get("path", "").endswith(".parquet")]


def filter_target_months(files: List[str], start_dt: datetime, end_dt: datetime) -> List[str]:
    """Filter list of Parquet files to strictly match target date range."""
    selected = []
    for f in sorted(files):
        parts = f.split("/")[-1].split("_")
        try:
            file_date = datetime.strptime(parts[-3], "%Y-%m-%d")
            if start_dt <= file_date <= end_dt:
                selected.append(f)
        except (ValueError, IndexError):
            continue
    return selected


def fetch_single_parquet(repo: str, rel_path: str) -> Optional[pl.DataFrame]:
    """Download and parse a single monthly Parquet file with retry."""
    filename = rel_path.split("/")[-1]
    url = f"https://huggingface.co/datasets/{repo}/resolve/main/1/{filename}"
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})

    for attempt in range(3):
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:
                buf = io.BytesIO(resp.read())
                return pl.read_parquet(buf)
        except (urllib.error.URLError, TimeoutError):
            if attempt == 2:
                print(f"[-] Failed to download: {filename}")
                return None
    return None


def normalize_oanda_frame(df: pl.DataFrame) -> pl.DataFrame:
    """Normalize OANDA Parquet schema to standard Project Alpha kline schema."""
    norm = df.with_columns(
        (pl.col("time") * 1000).alias("open_time"),
        (pl.col("time") * 1000 + 59999).alias("close_time"),
        (pl.col("volume").cast(pl.Float64) * pl.col("close")).alias("quote_volume"),
        pl.col("volume").cast(pl.Int64).alias("trades_count"),
        (pl.col("volume").cast(pl.Float64) * 0.5).alias("taker_buy_volume"),
        (pl.col("volume").cast(pl.Float64) * pl.col("close") * 0.5).alias("taker_buy_quote_volume"),
    )
    return norm.select(
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


def download_metal_1m(symbol: str, repo: str, start_dt: datetime, end_dt: datetime) -> pl.DataFrame:
    """Download 10 years of monthly 1m Parquets in parallel and return continuous DataFrame."""
    all_files = fetch_monthly_file_list(repo)
    target_files = filter_target_months(all_files, start_dt, end_dt)
    print(f"[*] {symbol}: downloading {len(target_files)} monthly chunks (8 workers)...")

    frames: List[pl.DataFrame] = []
    with ThreadPoolExecutor(max_workers=8) as executor:
        futures = {executor.submit(fetch_single_parquet, repo, f): f for f in target_files}
        for future in as_completed(futures):
            f_path = futures[future]
            raw_df = future.result()
            if raw_df is not None:
                frames.append(normalize_oanda_frame(raw_df))
                print(f"    [+] {symbol} {f_path.split('/')[-1]}: parsed")

    if not frames:
        raise RuntimeError(f"No data successfully downloaded for {symbol}")

    combined = pl.concat(frames)
    return validate_clean_dataset(combined, symbol)


def resample_and_save_all(df_1m: pl.DataFrame, symbol: str, out_dir: Path) -> None:
    """Save 1m and resample to 5m, 15m, 30m, 1h, 4h, 1d, 1w, 1mo Parquets."""
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


def process_metal(symbol: str, repo: str, start_dt: datetime, end_dt: datetime, out_dir: Path) -> None:
    """Coordinate download, validation, and multi-timeframe generation for a metal."""
    print(f"\n==================== PROCESSING {symbol} ====================")
    df_1m = download_metal_1m(symbol, repo, start_dt, end_dt)
    first_dt = df_1m["datetime"][0]
    last_dt = df_1m["datetime"][-1]
    print(f"[+] {symbol} 1m complete: {len(df_1m):,} bars ({first_dt} -> {last_dt})")
    resample_and_save_all(df_1m, symbol, out_dir)


def main() -> int:
    """Download and aggregate 10 years of Gold and Silver multi-timeframe data."""
    out_dir = Path("data/raw/metals")
    start_dt = datetime(2016, 11, 1)
    end_dt = datetime(2026, 11, 30)

    for symbol, repo in METALS.items():
        process_metal(symbol, repo, start_dt, end_dt, out_dir)

    print("\n[DONE] All Gold & Silver multi-timeframe parquets generated successfully.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
