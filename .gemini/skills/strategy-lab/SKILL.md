---
name: strategy-lab
description: Autonomous Quant Strategy Lab for extracting YouTube trading videos into clean code, backtesting against 10 years of market friction, and managing the public Cemetery vs private passed vault.
---

# Alpha: Strategy Lab Agent Skill

This skill allows AI coding agents (Antigravity, Claude Code, Codex) to autonomously extract, draft, backtest, and route trading strategies without human friction.

## Core Commands

### 1. `/extract <youtube_url> [name]`
Fetches the video transcript using `youtube_transcript_api` and extracts deterministic setup, trigger, and stop rules into structured JSON in `python/research/strategies/drafts/<name>/`.

### 2. `/draft <name>`
Translates the extracted JSON specification into a standardized `BaseStrategy` class conforming to `python/research/strategies/base.py`, leveraging our multi-timeframe Polars primitives.

### 3. `/test <name>`
Runs a full 10-year purged walk-forward backtest on historical data with:
- Exchange taker fees (0.05% - 0.10%)
- Slippage & spread friction
- Scale-invariant position sizing

### 4. Automatic Routing Rule
- **If the strategy FAILS (< 30 trades, negative EV, fee drag, or > 40% drawdown):**
  - Moved to `python/research/strategies/failed/<name>/`
  - Autopsy logged to `docs/backtests/KILLED_STRATEGIES.md`
  - Tracked in Git and published to GitHub as public research.
- **If the strategy PASSES:**
  - Moved to `python/research/strategies/passed/<name>/`
  - Backtest report saved to `docs/backtests/passed/<name>.md`
  - **Protected by `.gitignore` by default (never pushed to GitHub).**

### 5. `/cemetery`
Prints the list of debunked retail strategies and their causes of death.
