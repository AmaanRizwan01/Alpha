"""Strategy Lab Automated Backtester & Cemetery Router.

Tests strategies on historical data with realistic taker fees and slippage.
Automatically routes:
- FAILED strategies -> python/research/strategies/failed/<name>/ & KILLED_STRATEGIES.md (Public)
- PASSED strategies -> python/research/strategies/passed/<name>/ & docs/backtests/passed/<name>.md (Private, gitignored)
"""

import sys
import shutil
from pathlib import Path
from datetime import datetime
from typing import Dict, Any, Optional

ROOT_DIR = Path(__file__).resolve().parents[3]
if str(ROOT_DIR) not in sys.path:
    sys.path.insert(0, str(ROOT_DIR))

from python.research.strategies.base import BaseStrategy, StrategyPerformance


def evaluate_and_route_strategy(
    strategy: BaseStrategy,
    data_dir: Optional[Path] = None,
    source_dir: Optional[Path] = None,
) -> Dict[str, Any]:
    """Run full 10-year backtest and automatically route to passed/ or failed/."""
    d_dir = data_dir or (ROOT_DIR / "data" / "raw")
    perf = strategy.run_backtest(d_dir)
    
    # Statistical kill gates
    is_passed = (
        perf.trade_count >= 30
        and perf.profit_factor >= 1.25
        and perf.net_return_pct > 0.0
        and perf.max_drawdown_pct < 0.40
    )
    
    slug = strategy.strategy_id.lower()
    
    if is_passed:
        target_dir = ROOT_DIR / "python" / "research" / "strategies" / "passed" / slug
        target_dir.mkdir(parents=True, exist_ok=True)
        
        # Move or save strategy files into private passed/
        if source_dir and source_dir.exists():
            for item in source_dir.iterdir():
                shutil.copy2(item, target_dir / item.name)
        
        # Save private backtest document
        doc_dir = ROOT_DIR / "docs" / "backtests" / "passed"
        doc_dir.mkdir(parents=True, exist_ok=True)
        doc_path = doc_dir / f"{strategy.strategy_id}.md"
        doc_path.write_text(
            f"# {strategy.strategy_name} — Backtest Report\n\n"
            f"- **Date Generated:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
            f"- **Trade Count:** {perf.trade_count}\n"
            f"- **Win Rate:** {perf.win_rate_pct:.1f}%\n"
            f"- **Profit Factor:** {perf.profit_factor:.2f}\n"
            f"- **Net Return:** +{perf.net_return_pct:.1f}%\n"
            f"- **Max Drawdown:** -{perf.max_drawdown_pct:.1f}%\n\n"
            f"> [!NOTE]\n"
            f"> This strategy passed all statistical gates and is saved in your private vault (gitignored).\n",
            encoding="utf-8"
        )
        verdict = "PASSED (Saved to Private Vault)"
    else:
        target_dir = ROOT_DIR / "python" / "research" / "strategies" / "failed" / slug
        target_dir.mkdir(parents=True, exist_ok=True)
        
        if source_dir and source_dir.exists():
            for item in source_dir.iterdir():
                shutil.copy2(item, target_dir / item.name)
        
        # Append autopsy to public KILLED_STRATEGIES.md
        killed_doc = ROOT_DIR / "docs" / "backtests" / "KILLED_STRATEGIES.md"
        autopsy_entry = (
            f"\n\n### {strategy.strategy_name} (`{strategy.strategy_id}`)\n"
            f"- **Tested On:** {datetime.now().strftime('%Y-%m-%d')}\n"
            f"- **Trade Count:** {perf.trade_count}\n"
            f"- **Win Rate:** {perf.win_rate_pct:.1f}%\n"
            f"- **Profit Factor:** {perf.profit_factor:.2f}\n"
            f"- **Net Return:** {perf.net_return_pct:+.1f}%\n"
            f"- **Max Drawdown:** -{perf.max_drawdown_pct:.1f}%\n"
            f"- **Autopsy Cause:** {'Negative EV or fee erosion' if perf.net_return_pct <= 0 else 'High drawdown or low sample size'}\n"
        )
        with open(killed_doc, "a", encoding="utf-8") as f:
            f.write(autopsy_entry)
            
        verdict = "FAILED (Logged to Public Cemetery)"

    return {
        "strategy_id": strategy.strategy_id,
        "is_passed": is_passed,
        "verdict": verdict,
        "trade_count": perf.trade_count,
        "win_rate_pct": perf.win_rate_pct,
        "profit_factor": perf.profit_factor,
        "net_return_pct": perf.net_return_pct,
        "max_drawdown_pct": perf.max_drawdown_pct,
    }
