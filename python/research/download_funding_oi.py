"""Binance Vision Historical Funding Rates & Metrics Downloader for Project Alpha.

Downloads historical funding rates and open interest metrics from data.binance.vision,
cleans and normalizes timestamps, and outputs compressed Parquet datasets into data/raw/.
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


def build_funding_url(symbol: str, year: int, month: int) -> str:
    """Build the official data.binance.vision monthly funding rate archive URL."""
    base = "https://data.binance.vision/data/futures/um/monthly/fundingRate"
    file_name = f"{symbol}-fundingRate-{year}-{month:02d}.zip"
    return f"{base}/{symbol}/{file_name}"


def fetch_zip_bytes(url: str) -> Optional[bytes]:
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


def parse_funding_csv(content: bytes) -> pl.DataFrame:
    """Parse and normalize funding rate CSV from monthly zip archive."""
    with zipfile.ZipFile(io.BytesIO(content)) as zf:
        csv_name = next(n for n in zf.namelist() if n.endswith(".csv"))
        with zf.open(csv_name) as csv_file:
            raw_df = pl.read_csv(
                csv_file.read(),
                infer_schema_length=1000,
            )

    # Standardize column names (older files vs newer files)
    # Expected schema: calc_time, funding_interval_hours, last_funding_rate
    clean_df = raw_df.select(
        pl.col("calc_time").cast(pl.Int64),
        pl.col("funding_interval_hours").cast(pl.Int64),
        pl.col("last_funding_rate").cast(pl.Float64).alias("funding_rate"),
    )

    return clean_df


def download_funding_history(
    symbol: str,
    start_year: int,
    end_year: int,
    end_month: int,
    output_dir: Path,
) -> Path:
    """Download all monthly funding rate chunks for a symbol into Parquet."""
    frames: List[pl.DataFrame] = []
    total_months = 0

    print(f"[*] Downloading funding rates for {symbol} ({start_year} to {end_year}-{end_month:02d})...")

    for year in range(start_year, end_year + 1):
        max_m = end_month if year == end_year else 12
        for month in range(1, max_m + 1):
            url = build_funding_url(symbol, year, month)
            data = fetch_zip_bytes(url)
            if data is None:
                print(f"[-] {symbol} funding {year}-{month:02d}: not found (404)")
                continue

            frames.append(parse_funding_csv(data))
            total_months += 1

    if not frames:
        raise RuntimeError(f"no funding rate data downloaded for {symbol}")

    combined = pl.concat(frames)
    sorted_df = combined.sort("calc_time").unique(subset=["calc_time"], keep="first")

    # Add human-readable datetime
    final_df = sorted_df.with_columns(
        pl.from_epoch(pl.col("calc_time"), time_unit="ms").alias("datetime")
    )

    output_dir.mkdir(parents=True, exist_ok=True)
    out_file = output_dir / f"{symbol}_funding.parquet"
    final_df.write_parquet(out_file, compression="zstd")

    first_dt = final_df["datetime"][0]
    last_dt = final_df["datetime"][-1]
    print(f"[OK] Saved {len(final_df):,} funding epochs to {out_file}")
    print(f"     Range: {first_dt} -> {last_dt} ({total_months} months)\n")
    return out_file


def main() -> int:
    symbols = ["BTCUSDT", "ETHUSDT"]
    out_dir = Path("data/raw")

    print("=== Project Alpha Historical Funding Rate Downloader ===\n")
    for symbol in symbols:
        download_funding_history(
            symbol=symbol,
            start_year=2021,
            end_year=2026,
            end_month=10,
            output_dir=out_dir,
        )

    print("[SUCCESS] All historical funding rate datasets downloaded and saved.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
