"""Macro ETF and Volatility Index Downloader for Project Alpha (Strategy 4).

Downloads historical daily bars for SPY, QQQ, TLT, GLD, and ^VIX from Yahoo Finance API,
normalizes columns to match Project Alpha conventions, and saves as Parquet in data/raw/.
"""

import json
from pathlib import Path
import urllib.request
import polars as pl
from datetime import datetime


def fetch_daily_chart(symbol: str, range_str: str = "5y") -> pl.DataFrame:
    """Download daily OHLCV from Yahoo Finance chart endpoint."""
    url = f"https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?interval=1d&range={range_str}"
    req = urllib.request.Request(
        url,
        headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"},
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read().decode("utf-8"))

    res = data["chart"]["result"][0]
    timestamps = res["timestamp"]
    indicators = res["indicators"]["quote"][0]
    adjclose = res["indicators"].get("adjclose", [{}])[0].get("adjclose", indicators["close"])

    rows = []
    for i in range(len(timestamps)):
        t = timestamps[i]
        o = indicators["open"][i]
        h = indicators["high"][i]
        l = indicators["low"][i]
        c = indicators["close"][i]
        v = indicators["volume"][i]
        ac = adjclose[i] if adjclose else c

        # skip days with nulls (e.g. market holidays)
        if None in (o, h, l, c):
            continue

        rows.append({
            "open_time": t * 1000,
            "datetime": datetime.utcfromtimestamp(t),
            "open": float(o),
            "high": float(h),
            "low": float(l),
            "close": float(c),
            "adj_close": float(ac) if ac is not None else float(c),
            "volume": float(v) if v is not None else 0.0,
        })

    df = pl.DataFrame(rows).sort("open_time")
    return df


def main() -> None:
    symbols = {
        "SPY": "SPY",
        "QQQ": "QQQ",
        "TLT": "TLT",
        "GLD": "GLD",
        "^VIX": "VIX",
    }
    out_dir = Path("data/raw")
    out_dir.mkdir(parents=True, exist_ok=True)

    print("=== Downloading Macro Asset Daily History ===")
    for yf_sym, out_sym in symbols.items():
        print(f"[*] Fetching {yf_sym} -> {out_sym}...")
        df = fetch_daily_chart(yf_sym, range_str="5y")
        out_path = out_dir / f"{out_sym}_daily.parquet"
        df.write_parquet(out_path)
        first_dt = df["datetime"][0]
        last_dt = df["datetime"][-1]
        print(f"[OK] Saved {len(df):,} bars to {out_path} ({first_dt} -> {last_dt})")

    print("[SUCCESS] All macro assets downloaded successfully.")


if __name__ == "__main__":
    main()
