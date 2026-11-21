"""Data path resolution and dataset discovery utilities for Project Alpha."""

from __future__ import annotations

from pathlib import Path
from typing import Union

CATEGORIES = ("crypto", "metals", "macro", "futures")

# Legacy ETF alias fallbacks and futures symbol equivalencies
ALIASES = {
    "GLD_daily.parquet": "XAUUSD_1d.parquet",
    "SLV_daily.parquet": "XAGUSD_1d.parquet",
    "SPY_daily.parquet": "ES_1d.parquet",
    "QQQ_daily.parquet": "NQ_1d.parquet",
    "TLT_daily.parquet": "US30Y_1d.parquet",
    "BTC_daily.parquet": "BTCUSDT_1d.parquet",
    "ETH_daily.parquet": "ETHUSDT_1d.parquet",
}

PREFIX_REPLACEMENTS = {
    "MES_": "ES_",
    "SPX500_": "ES_",
    "USA500_": "ES_",
    "MNQ_": "NQ_",
    "NAS100_": "NQ_",
    "USATECH_": "NQ_",
    "USB10Y_": "US10Y_",
    "USB30Y_": "US30Y_",
    "ZN_": "US10Y_",
    "ZB_": "US30Y_",
}


def resolve_data_path(data_dir: Union[Path, str], filename: str) -> Path:
    """Resolve a dataset path by checking root and categorized subfolders.

    Checks:
    1. data_dir / filename
    2. data_dir / sub / filename for sub in ('crypto', 'metals', 'macro', 'futures')
    3. ALIASES mapping (e.g. GLD_daily -> XAUUSD_1d)
    4. PREFIX_REPLACEMENTS (e.g. MES_1h -> ES_1h, MNQ_15m -> NQ_15m)
    Returns direct path if not found.
    """
    base = Path(data_dir)
    direct = base / filename
    if direct.exists():
        return direct

    for sub in CATEGORIES:
        candidate = base / sub / filename
        if candidate.exists():
            return candidate

    if filename in ALIASES:
        return resolve_data_path(base, ALIASES[filename])

    for prefix, target in PREFIX_REPLACEMENTS.items():
        if filename.startswith(prefix):
            mapped_name = target + filename[len(prefix):]
            return resolve_data_path(base, mapped_name)

    return direct
