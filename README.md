# Alpha - Trade with Math, Not Hype

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)
[![Rust: 1.80+](https://img.shields.io/badge/Rust-1.80+-orange.svg)](https://www.rust-lang.org)
[![Python: 3.11+](https://img.shields.io/badge/Python-3.11+-yellow.svg)](https://www.python.org)
[![Zero Panic](https://img.shields.io/badge/Safety-%23!%5Bforbid(unsafe__code)%5D-success.svg)]()

> Most trading strategies look great on paper until you factor in exchange fees, slippage, and chop.
> Alpha lets you test ideas against 10 years of historical data in seconds, extract rules from plain English or video transcripts, and run what survives on a deterministic Rust engine.

<p align="center">
  <img src="assets/launch-video.gif" alt="Project Alpha Demo" width="100%" />
</p>

### Why I Built This

I watched too many friends blow up accounts following hype strategies that look great on social media but have zero real statistical edge. Most people don't have the tooling to backtest against 10 years of tick data, taker fees, and execution slippage. 

I built Alpha to do that math in seconds: test any idea or video setup before risking a single dollar, and run what actually survives through a clean Rust engine.

---

## 1. What Is Alpha?

Alpha is a dual-layer quantitative trading platform designed around two goals:

1. **The Strategy Lab:** Most discretionary trading setups fail when tested across multiple market regimes. Alpha extracts entry and exit rules directly from video transcripts or text prompts, compiles them into vectorized code, and runs 10-year purged walk-forward backtests accounting for exchange taker fees, spreads, and slippage.
2. **The Core Rust Engine:** If a strategy survives the data, execution needs to be safe and deterministic. Built in pure Rust (`alpha-core` and `alpha-node`), Alpha handles scale-invariant position sizing (`$100` to `$10,000+`), exchange-hosted atomic stop orders (`OTOCO`), and instant SQLite crash recovery.

---

## 2. The 4-Step Strategy Lab Workflow

Whether you use the terminal CLI or an AI coding agent (Antigravity, Claude Code, Codex), the entire lifecycle takes seconds:

```
Trading Setup or Video URL
          │
          ▼
   1. [/extract]    ──► Pulls transcript & parses entry/exit rules into clean JSON
          │
          ▼
   2. [/draft]      ──► Generates vectorized strategy code
          │
          ▼
   3. [/test]       ──► Runs 10-year backtest with taker fees & slippage
          │
      ┌───┴───┐
      ▼       ▼
   [FAIL]   [PASS]
      │       │
      │       └─► Saved to python/research/strategies/passed/<name>/
      │           Backtest doc saved to docs/backtests/passed/<name>.md
      │           (Private and gitignored by default)
      │
      └─────────► Moved to python/research/strategies/failed/<name>/
                  Autopsy logged to docs/backtests/KILLED_STRATEGIES.md
                  (Public on GitHub for shared learning)
```

---

## 3. Quick Start (Get Running in 3 Minutes)

You don't need to be a quant or software engineer to use Alpha. Follow these 3 steps:

### Step 1: Install the Basics (If you don't already have them)
* **Python (3.11+)**: Download from [python.org](https://www.python.org/downloads/) *(check "Add Python to PATH" during installation)*.
* **Rust Toolchain**: Install from [rustup.rs](https://rustup.rs/) *(takes 30 seconds)*.
* **Git**: Install from [git-scm.com](https://git-scm.com/).

### Step 2: Clone & One-Time Environment Setup
Open your terminal (PowerShell, Command Prompt, or Terminal on macOS/Linux) and paste:

```bash
git clone https://github.com/AmaanRizwan01/Alpha.git
cd Alpha

# Create and activate virtual environment
python -m venv .venv

# On Windows:
.venv\Scripts\activate
# On Mac/Linux:
source .venv/bin/activate

# Install the engine and research tools
pip install -U pip maturin
pip install -e .
pip install huggingface_hub yfinance youtube-transcript-api
```

### Step 3: Run the One-Command Setup
Download 10 years of historical tick data across 5 asset classes (Crypto, Metals, Index Futures, Treasuries, Macro), resample into 9 timeframes, and run system verification:

* **In Your AI Agent (Antigravity, Claude Code, Cursor, Codex):**
  Just type:
  ```text
  /setup
  ```
* **Or in the Terminal:**
  ```bash
  python -m python.research.lab setup
  ```

---

## 4. How to Test Any Strategy or YouTube Video

### Option A: In Your AI Agent (Antigravity, Claude Code, Cursor, Codex)
Alpha includes built-in agent skills so you can talk to the lab in plain English:
* `/setup` — One-command onboarding: downloads 10 years of data and validates test suites.
* `/extract <url>` — Grabs video transcript and parses setup, trigger, and stop rules.
* `/draft <name>` — Drafts the strategy using our multi-timeframe Polars primitives.
* `/test <name>` — Runs the 10-year fee reality check and shows the verdict card.
* `/cemetery` — Displays the graveyard of failed retail strategies.

### Option B: In the Terminal
```bash
# 1. Extract strategy rules from any YouTube trading video
python -m python.research.lab extract "https://youtube.com/watch?v=..." my_strategy

# 2. Test your strategy on 10 years of tick data with fees
python -m python.research.lab test my_strategy

# 3. View the Strategy Cemetery (failed retail strategies)
python -m python.research.lab cemetery
```

---

## 5. Automatic Privacy by Design (Keep Live Strategies Local)

We believe in open-source infrastructure, but trading models shouldn't be publicly leaked by default.

Alpha enforces an automatic privacy separation:
* **Failed Strategies are Public:** If a strategy fails the 10-year quantitative gates (negative EV, fee drag, or severe drawdown), it is placed in `python/research/strategies/failed/` and logged to `docs/backtests/KILLED_STRATEGIES.md`. This is tracked in Git to build a shared public cemetery of debunked setups.
* **Passed Strategies are Private:** When a strategy passes all statistical gates, it is placed in `python/research/strategies/passed/<name>/` and its backtest report is saved in `docs/backtests/passed/<name>.md`. Both folders are protected by `.gitignore` by default. Winning models stay local to your environment.

---

## 6. Common Strategy Failure Modes (The Strategy Cemetery)

Here is what happens when common retail trading setups are subjected to 10 years of mathematical friction:

| Strategy | Common Claim | The Real Math (10-Year Backtest) | Why It Fails |
| :--- | :--- | :--- | :--- |
| **Naive 5m RSI Scalper** | *"90% winrate on 5-minute charts!"* | Net Return: **-78.4%** • Trades: 4,821 | Destroyed by taker fees ($34,000 paid to exchanges). |
| **Standard MACD Cross** | *"The easiest trend indicator ever"* | Net Return: **-41.2%** • Profit Factor: 0.84 | Whipsawed repeatedly during sideways consolidation. |
| **Martingale Grid Recovery** | *"Never takes a loss!"* | Net Return: **-100.0%** • Max Drawdown: **-100%** | Account wiped out during a single liquidation cascade. |
| **Unbounded 1m Breakouts**| *"Catch explosive moves early"* | Net Return: **-62.1%** • Slippage: -38% | Severe slippage on market orders during illiquid wicks. |

*(Read the full post-mortems in [`docs/backtests/KILLED_STRATEGIES.md`](docs/backtests/KILLED_STRATEGIES.md)).*

---

## 7. The Pure Rust Production Engine

The live execution daemon is completely decoupled from the research lab to keep production lean, fast, and crash-resilient:

```text
       ┌─────────────────────────────────────────────────────────────────────────────────────┐
       │                                     alpha-node                                      │
       │                   Async 24/7 Production Daemon (Tokio • SQLite WAL)                 │
       └──────────────────────────┬───────────────────────────────────────┬──────────────────┘
                                  │                                       │
                                  ▼                                       ▼
       ┌──────────────────────────────────────┐       ┌──────────────────────────────────────┐
       │             alpha-venues             │       │              alpha-core              │
       │  Binance & OKX Protocol Deserializer │       │   Pure Quantitative Math & Sizing    │
       │  Atomic OTOCO / attachAlgoOrds Stops │       │   #![forbid(unsafe_code)] • Decimal  │
       └──────────────────────────────────────┘       └──────────────────────────────────────┘
```

### Engine Safety Invariants:
1. **Zero Unsafe Code:** `alpha-core`, `alpha-node`, and `alpha-venues` strictly enforce `#![forbid(unsafe_code)]` and `#![deny(clippy::unwrap_used)]`.
2. **Exact Decimal Accounting:** Every price, order quantity, fee, and balance is calculated using `rust_decimal::Decimal`. Never floating-point `f64`.
3. **Atomic Exchange-Hosted Stops:** Entries submit parent and stop orders in a single API call (`attachAlgoOrds` on OKX, `OTOCO` on Binance). If server power or internet cuts out, the stop loss is already live on the exchange order book.
4. **Instant State Recovery:** SQLite Write-Ahead Logging (WAL) records every order and position synchronously. Cold-boot reconciliation reconstructs state in under 10 milliseconds after power loss.
5. **Scale-Invariant Sizing:** Risk is computed as a percentage of real-time equity (`r = 0.01` to `0.02`), automatically adapting from `$100` to `$10,000+` with strict exchange lot size and minimum notional checks.

---

## 8. License

This project is licensed under the **GNU Affero General Public License v3.0 (AGPL-3.0)**. 

You are free to inspect, modify, and run this system for personal and educational use. However, if you modify or run a modified version on a server or network service, you must make the complete source code of your modified version available to all users under the same AGPL-3.0 license. Closed-source commercial exploitation, proprietary SaaS wrapping, or reselling is strictly prohibited.

---

## 9. Legal Disclaimer

> [!CAUTION]
> **This software is for educational, research, and technical evaluation purposes only.**
> Nothing in this repository constitutes financial, investment, legal, or tax advice. Algorithmic trading and cryptocurrency markets involve substantial risk of financial loss. Past backtested performance is strictly hypothetical and does not guarantee future results. The authors and contributors assume zero liability for financial losses, software bugs, execution failures, or exchange errors incurred while using this software.
