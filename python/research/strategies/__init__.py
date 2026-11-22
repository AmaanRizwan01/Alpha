"""Plug-and-Play Strategy Registry for Project Alpha Research Laboratory.

Discovers and instantiates quantitative strategies across evaluation tiers:
- passed: Tier 1 active production-ready strategies (local & gitignored)
- suboptimal: Tier 2 strategies (VOLATILITY_BREAKOUT)
- failed: Tier 3 archived candidates (SESSION_IMBALANCES, CROSS_SECTIONAL_MOMENTUM, etc.)
- all: Every discovered strategy across the entire laboratory.

Any strategy file can be added, deleted, or moved between tiers without breaking other
strategies or the framework.
"""

from __future__ import annotations

import importlib
import inspect
import pkgutil
from pathlib import Path
from typing import Dict, List, Optional, Type

from python.research.strategies.base import BaseStrategy, StrategyPerformance


def _discover_all_strategies() -> Dict[str, Dict[str, Type[BaseStrategy]]]:
    """Scan passed/, suboptimal/, failed/, and root strategies directory."""
    package_dir = Path(__file__).resolve().parent
    tiers: Dict[str, Dict[str, Type[BaseStrategy]]] = {
        "passed": {},
        "suboptimal": {},
        "failed": {},
    }

    # 1. Scan subpackages (passed, suboptimal, failed)
    for tier in ("passed", "suboptimal", "failed"):
        tier_dir = package_dir / tier
        if not tier_dir.exists():
            continue
        for _, module_name, is_pkg in pkgutil.iter_modules([str(tier_dir)]):
            if module_name in ("base", "__init__"):
                continue
            try:
                full_module_name = f"python.research.strategies.{tier}.{module_name}"
                mod = importlib.import_module(full_module_name)
                for _, obj in inspect.getmembers(mod, inspect.isclass):
                    if (
                        issubclass(obj, BaseStrategy)
                        and obj is not BaseStrategy
                        and getattr(obj, "__module__", "").startswith(full_module_name)
                    ):
                        try:
                            instance = obj()
                            tiers[tier][instance.strategy_id] = obj
                        except Exception:
                            tiers[tier][obj.__name__] = obj
            except Exception:
                continue

    # 2. Also scan root strategies/ for any loose strategy files
    for _, module_name, is_pkg in pkgutil.iter_modules([str(package_dir)]):
        if is_pkg or module_name in ("base", "__init__", "passed", "suboptimal", "failed"):
            continue
        try:
            full_module_name = f"python.research.strategies.{module_name}"
            mod = importlib.import_module(full_module_name)
            for _, obj in inspect.getmembers(mod, inspect.isclass):
                if (
                    issubclass(obj, BaseStrategy)
                    and obj is not BaseStrategy
                    and getattr(obj, "__module__", None) == mod.__name__
                ):
                    try:
                        instance = obj()
                        tiers["passed"][instance.strategy_id] = obj
                    except Exception:
                        tiers["passed"][obj.__name__] = obj
        except Exception:
            continue

    return tiers


def get_available_strategies(tier: Optional[str] = None) -> Dict[str, Type[BaseStrategy]]:
    """Return dictionary of dynamically discovered strategy classes by tier.

    tier:
      - None: returns active production + benchmark strategies ('passed' + 'suboptimal')
      - 'passed': returns Tier 1 production strategies
      - 'suboptimal': returns Tier 2 benchmark strategies
      - 'failed': returns Tier 3 killed candidates
      - 'all': returns all strategies across all tiers
    """
    discovered = _discover_all_strategies()
    if tier == "passed":
        return discovered["passed"]
    elif tier == "suboptimal":
        return discovered["suboptimal"]
    elif tier == "failed":
        return discovered["failed"]
    elif tier == "all":
        merged = {}
        for t in ("passed", "suboptimal", "failed"):
            merged.update(discovered[t])
        return merged
    else:
        # Default (None): passed + suboptimal (main active library)
        merged = {}
        merged.update(discovered["passed"])
        merged.update(discovered["suboptimal"])
        return merged


def load_all_strategies(tier: Optional[str] = None) -> List[BaseStrategy]:
    """Instantiate and return all available plug-and-play strategies for the given tier."""
    classes = get_available_strategies(tier=tier)
    instances: List[BaseStrategy] = []
    for sid, cls in classes.items():
        try:
            instances.append(cls())
        except Exception as exc:
            print(f"[!] Warning: Failed to instantiate strategy {sid}: {exc}")
    return instances


__all__ = [
    "BaseStrategy",
    "StrategyPerformance",
    "get_available_strategies",
    "load_all_strategies",
]
