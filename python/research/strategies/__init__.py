"""Plug-and-Play Strategy Registry for Project Alpha Research.

Dynamically scans, registers, and instantiates all quantitative strategies present
in the strategies directory. Any strategy file can be deleted or plugged back in
without breaking the registry or other strategies.
"""

from __future__ import annotations

import importlib
import inspect
import pkgutil
from pathlib import Path
from typing import Dict, List, Type

from python.research.strategies.base import BaseStrategy, StrategyPerformance

STRATEGY_CLASSES: Dict[str, Type[BaseStrategy]] = {}


def _discover_strategies() -> Dict[str, Type[BaseStrategy]]:
    """Scan current directory for BaseStrategy subclasses."""
    package_dir = Path(__file__).resolve().parent
    found: Dict[str, Type[BaseStrategy]] = {}

    for _, module_name, is_pkg in pkgutil.iter_modules([str(package_dir)]):
        if is_pkg or module_name in ("base", "__init__"):
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
                    # Instantiate temporary object to read strategy_id
                    try:
                        instance = obj()
                        found[instance.strategy_id] = obj
                    except Exception:
                        # If init requires args, try without
                        found[obj.__name__] = obj
        except Exception as e:
            # If a strategy fails to import (e.g. broken or in-progress), skip it gracefully
            continue

    return found


def get_available_strategies() -> Dict[str, Type[BaseStrategy]]:
    """Return dictionary of all dynamically discovered strategy classes."""
    return _discover_strategies()


def load_all_strategies() -> List[BaseStrategy]:
    """Instantiate and return all available plug-and-play strategies."""
    classes = get_available_strategies()
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
