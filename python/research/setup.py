"""Project Alpha — Master Repository Setup & Data Ingestion Pipeline.

One-command onboarding: downloads 10 years of historical data (crypto, metals,
futures, treasuries, macro), resamples 1m data to 9 timeframes, verifies
dataset integrity, and validates the system with Python and Rust test suites.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple

import polars as pl

TIMEFRAMES = ["1m", "5m", "15m", "30m", "1h", "4h", "1d", "1w", "1mo"]

ASSET_DIRS = {
    "crypto": Path("data/raw/crypto"),
    "metals": Path("data/raw/metals"),
    "futures": Path("data/raw/futures"),
    "macro": Path("data/raw/macro"),
}

ASSET_MAP = {
    "crypto": (["BTCUSDT", "ETHUSDT", "SOLUSDT"], "python.research.download_multitimeframe"),
    "metals": (["XAUUSD", "XAGUSD"], "python.research.download_metals"),
    "futures": (["ES", "NQ"], "python.research.download_futures"),
    "treasuries": (["US10Y", "US30Y"], "python.research.download_treasuries"),
}


def check_symbol_timeframes(directory: Path, symbol: str) -> bool:
    """Check if all 9 timeframes exist and are non-empty for a symbol."""
    for tf in TIMEFRAMES:
        p = directory / f"{symbol}_{tf}.parquet"
        if not p.exists() or p.stat().st_size == 0:
            return False
    return True


def check_macro_complete() -> bool:
    """Check if macro benchmarks (DXY, VIX) exist and are non-empty."""
    macro_dir = ASSET_DIRS["macro"]
    for sym in ["DXY_daily.parquet", "VIX_daily.parquet"]:
        p = macro_dir / sym
        if not p.exists() or p.stat().st_size == 0:
            return False
    return True


def run_downloader(module_name: str) -> bool:
    """Run a specific downloader module via subprocess for clean memory management."""
    print(f"[*] Executing downloader: {module_name}...")
    cmd = [sys.executable, "-m", module_name]
    res = subprocess.run(cmd, check=False)
    return res.returncode == 0


def ingest_category(cat: str, symbols: List[str], module: str, force: bool) -> bool:
    """Check category assets and invoke downloader if missing or forced."""
    target_dir = ASSET_DIRS["futures"] if cat == "treasuries" else ASSET_DIRS[cat]
    all_present = all(check_symbol_timeframes(target_dir, sym) for sym in symbols)

    if all_present and not force:
        print(f"[OK] {cat.capitalize()} datasets verified ({len(symbols) * 9} files present).")
        return True

    print(f"[!] {cat.capitalize()} datasets missing or force enabled. Downloading...")
    return run_downloader(module)


def ingest_macro(force: bool) -> bool:
    """Ensure macro benchmarks (DXY, VIX) are downloaded."""
    if check_macro_complete() and not force:
        print("[OK] Macro benchmarks verified (DXY, VIX daily present).")
        return True

    print("[!] Macro benchmarks missing or force enabled. Downloading...")
    return run_downloader("python.research.download_macro")


def verify_datasets() -> Tuple[int, int]:
    """Verify all expected parquet files across all categories."""
    print("\n" + "=" * 60)
    print("DATASET INTEGRITY & TIMEFRAME VERIFICATION")
    print("=" * 60)
    total_expected = 83  # 9 assets * 9 timeframes = 81 + 2 macro = 83
    found_count = 0

    all_symbols = [
        ("crypto", ASSET_DIRS["crypto"], ["BTCUSDT", "ETHUSDT", "SOLUSDT"]),
        ("metals", ASSET_DIRS["metals"], ["XAUUSD", "XAGUSD"]),
        ("futures", ASSET_DIRS["futures"], ["ES", "NQ", "US10Y", "US30Y"]),
    ]

    for _, path, symbols in all_symbols:
        for sym in symbols:
            tf_ok = check_symbol_timeframes(path, sym)
            status = "[OK]" if tf_ok else "[FAIL]"
            p_1d = path / f"{sym}_1d.parquet"
            row_info = ""
            if p_1d.exists():
                try:
                    df = pl.read_parquet(p_1d)
                    row_info = f"({len(df):,} daily bars)"
                    found_count += 9 if tf_ok else 0
                except Exception:
                    row_info = "(read error)"
            print(f"  {status} {sym:<10} (9 timeframes) {row_info}")

    for m_file in ["DXY_daily.parquet", "VIX_daily.parquet"]:
        p = ASSET_DIRS["macro"] / m_file
        if p.exists() and p.stat().st_size > 0:
            df = pl.read_parquet(p)
            found_count += 1
            print(f"  [OK] {m_file:<18} ({len(df):,} daily bars)")
        else:
            print(f"  [FAIL] {m_file:<18} missing")

    print(f"\nVerification status: {found_count}/{total_expected} files ready.")
    return found_count, total_expected


def run_tests() -> bool:
    """Execute Python and Rust test suites."""
    print("\n" + "=" * 60)
    print("RUNNING VERIFICATION TEST SUITES")
    print("=" * 60)

    print("[*] Running pytest suite...")
    py_res = subprocess.run([sys.executable, "-m", "pytest", "-q"], check=False)
    if py_res.returncode != 0:
        print("[!] Pytest suite failed.")
        return False
    print("[OK] Pytest suite passed 100%.")

    print("[*] Running cargo test suite...")
    cargo_res = subprocess.run(["cargo", "test", "--workspace"], check=False)
    if cargo_res.returncode != 0:
        print("[!] Cargo test suite failed.")
        return False
    print("[OK] Cargo test suite passed 100%.")

    return True


def run_setup(force: bool = False, skip_tests: bool = False) -> int:
    """Main setup coordinator."""
    for path in ASSET_DIRS.values():
        path.mkdir(parents=True, exist_ok=True)

    print("\n" + "=" * 60)
    print("ALPHA REPOSITORY ONBOARDING & DATA SETUP")
    print("=" * 60)

    for cat, (symbols, module) in ASSET_MAP.items():
        if not ingest_category(cat, symbols, module, force):
            print(f"[!] Failed to ingest {cat} data.")
            return 1

    if not ingest_macro(force):
        print("[!] Failed to ingest macro data.")
        return 1

    found, expected = verify_datasets()
    if found < expected:
        print(f"[!] Dataset verification incomplete: {found}/{expected} files.")
        return 1

    if not skip_tests:
        if not run_tests():
            return 1

    print("\n" + "=" * 60)
    print("[SUCCESS] Project Alpha repository setup completed cleanly.")
    print("=" * 60 + "\n")
    return 0


def main() -> int:
    """Parse CLI arguments and run setup."""
    parser = argparse.ArgumentParser(description="Project Alpha One-Command Setup")
    parser.add_argument("--force", action="store_true", help="Force re-download all data")
    parser.add_argument("--skip-tests", action="store_true", help="Skip running test suites")
    args = parser.parse_args()
    return run_setup(force=args.force, skip_tests=args.skip_tests)


if __name__ == "__main__":
    sys.exit(main())
