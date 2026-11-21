---
name: setup
description: One-command repository onboarding and automated setup. Downloads 10-year historical market data for all assets (crypto, metals, index futures, treasuries, macro), resamples 1m data across all 9 timeframes (1m to 1mo), validates dataset integrity, and runs the full Python and Rust test suites to ensure the node is ready.
---

# Alpha: Automated Repository Setup (`/setup`)

This skill allows AI coding agents (Antigravity, Claude Code, Cursor, Codex) to autonomously onboard and set up the entire Project Alpha quantitative trading environment with a single command.

## Triggering

Run `/setup` in the agent chat interface, or invoke when a user asks to:
- Download the historical market data
- Set up the repository after cloning
- Verify all multi-timeframe datasets and test suites

## Workflow Instructions for the Agent

When the user asks to run `/setup`:

1. **Execute the Setup Pipeline:**
   Run the master setup script from the repository root:
   ```bash
   python -m python.research.setup
   ```
   *(If running in a local virtual environment: `.venv\Scripts\python.exe -m python.research.setup` on Windows or `.venv/bin/python -m python.research.setup` on Linux).*

2. **Under the Hood Execution:**
   The script automatically:
   - Verifies if datasets already exist in `data/raw/` to avoid redundant downloads.
   - Downloads missing assets across 5 asset classes (Crypto, Metals, Index Futures, Treasuries, Macro).
   - Resamples 1m data into all 9 timeframes: `1m`, `5m`, `15m`, `30m`, `1h`, `4h`, `1d`, `1w`, `1mo`.
   - Validates all 83 parquet files for monotonicity and geometric integrity.
   - Runs `pytest -q` (Python models) and `cargo test --workspace` (Rust execution engine).

3. **Output & Verification:**
   Confirm that all 83 dataset files are marked `[OK]` and both test suites pass 100%. Report the summary table back to the user.
