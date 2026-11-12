"""Binance Vision Open Interest Metrics Downloader for Project Alpha Research.

Downloads daily metrics zip files from data.binance.vision, extracts sum_open_interest
at 1-hour resolution, and saves clean Parquet files to data/raw/{symbol}_oi_1h.parquet.
"""

from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import date, timedelta
import io
from pathlib import Path
import urllib.error
import urllib.request
import zipfile
import polars as pl


def fetch_day_metrics(symbol: str, d: date) -> list[dict]:
    """Fetch and parse 1-day metrics zip for symbol, downsampled to 1h."""
    d_str = d.strftime("%Y-%m-%d")
    url = f"https://data.binance.vision/data/futures/um/daily/metrics/{symbol}/{symbol}-metrics-{d_str}.zip"
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "ProjectAlpha-Research/1.0"},
    )
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            content = resp.read()
    except Exception:
        return []

    records = []
    try:
        with zipfile.ZipFile(io.BytesIO(content)) as zf:
            name = zf.namelist()[0]
            with zf.open(name) as f:
                df = pl.read_csv(f.read())
        
        # Format: create_time is 'YYYY-MM-DD HH:MM:SS'
        # Filter for top of the hour ':00:00' to downsample from 5m to 1h
        hourly = df.filter(pl.col("create_time").str.ends_with(":00:00"))
        for row in hourly.select(["create_time", "sum_open_interest"]).iter_rows(named=True):
            records.append({
                "datetime_str": row["create_time"],
                "open_interest": float(row["sum_open_interest"]),
            })
    except Exception:
        pass

    return records


def download_symbol_oi(symbol: str, start_date: date, end_date: date, max_workers: int = 24) -> Path:
    """Download daily metrics in parallel and write hourly OI Parquet."""
    out_dir = Path("data/raw")
    out_dir.mkdir(parents=True, exist_ok=True)
    out_file = out_dir / f"{symbol}_oi_1h.parquet"

    days = []
    curr = start_date
    while curr <= end_date:
        days.append(curr)
        curr += timedelta(days=1)

    print(f"[*] Downloading {len(days)} daily metrics for {symbol} ({start_date} -> {end_date})...")
    all_records = []
    completed = 0

    with ThreadPoolExecutor(max_workers=max_workers) as executor:
        future_to_day = {executor.submit(fetch_day_metrics, symbol, d): d for d in days}
        for future in as_completed(future_to_day):
            res = future.result()
            if res:
                all_records.extend(res)
            completed += 1
            if completed % 250 == 0 or completed == len(days):
                print(f"    Progress {symbol}: {completed}/{len(days)} days ({len(all_records):,} hourly rows)")

    if not all_records:
        raise RuntimeError(f"no metrics downloaded for {symbol}")

    df = (
        pl.DataFrame(all_records)
        .with_columns(
            pl.col("datetime_str").str.to_datetime("%Y-%m-%d %H:%M:%S").alias("datetime")
        )
        .sort("datetime")
        .unique(subset=["datetime"], keep="first")
    )

    df.write_parquet(out_file)
    print(f"[OK] Saved {len(df):,} hourly OI records to {out_file}\n")
    return out_file


def main() -> None:
    # 2021-01-01 to 2026-10-31
    start = date(2021, 1, 1)
    end = date(2026, 10, 31)
    symbols = ["BTCUSDT", "ETHUSDT"]

    print("=== Downloading Historical Hourly Open Interest ===")
    for sym in symbols:
        download_symbol_oi(sym, start, end, max_workers=28)
    print("[SUCCESS] All Open Interest datasets downloaded.")


if __name__ == "__main__":
    main()
