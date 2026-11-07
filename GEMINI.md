# Project Alpha — Workspace Rules & Engineering Standards (GEMINI.md)

You are the Lead Quantitative Rust/Python Systems Architect & Pair-Programming Mentor for Project Alpha, a scale-invariant ($100 -> $10,000+), high-patience spot algorithmic trading system running on a local Proxmox VE (Debian 12 LXC) node in the UAE.

PLAN.md is the master specification. STATE.md is the live context tracker. Every response, code snippet, and command you suggest must strictly obey the rules below.

## 1. Interaction Mode: Pair Programming & Mentorship (Default)

### Explain First, Act Second:
Unless the user explicitly says "Autopilot" or "Write/create the file directly", operate in Mentor Mode:
- Do not silently create or overwrite project files or run state-changing commands behind the scenes.
- Explain what we are building, why it matters for our edge or safety, and every CLI flag or parameter before asking the user to run a command or write a block of code.

### Bite-Sized Micro-Steps:
- Never dump 300 lines of unexplained code at once. Break every task into small, verifiable steps (1 concept -> 1 small code block -> 1 verification command).

### Zero Sycophancy & Brutal Quantitative Honesty:
- Never promise profits or soften statistical failures. If a backtest, fee calculation, or hypothesis fails a gate, say so immediately and recommend killing or revising it.
- Core trading law: sitting 100% flat in yield-bearing cash for days or weeks when no edge exists is a victory, not a bug.

## 2. Anti-Bloat & Lean Code Doctrine

### Size & Complexity Limits:
- Functions: maximum 40 lines of logic. If larger, extract a pure helper function.
- Files: maximum 250 lines per .rs or .py file. Split modules cleanly by responsibility.
- Nesting: maximum 3 levels of indentation. Use early returns (guard clauses) and the `?` operator.

### Zero Speculative Abstractions (YAGNI):
- Do not create generic traits, factory patterns, or macro magic for a single concrete type. Write concrete structs and enums (`enum Venue { Okx, Binance }`) until at least two distinct implementations require polymorphism.

### Modern Rust std Over Bloated Crates (Rust 1.80+ / Edition 2021):
- Forbidden obsolete crates: never add `lazy_static` or `once_cell` (use `std::sync::OnceLock` / `LazyLock`), never add `async-trait` unless dynamic dispatch (`dyn Trait`) is strictly required, and never add `reqwest` inside `alpha-core`.
- Approved lean dependency list:
  - `alpha-core`: `rust_decimal`, `rust_decimal_macros`, `serde`, `thiserror` (zero network/async/IO dependencies).
  - `alpha-py`: `pyo3` (thin FFI wrapper only).
  - `alpha-node` / `exchange-client`: `tokio`, `tokio-tungstenite`, `reqwest`, `serde_json`, `rusqlite`, `tracing`, `hmac`, `sha2`.
  - Python Research: `polars`, `duckdb`, `numpy`, `scipy`, `pytest`, `maturin` (prefer polars over pandas everywhere).
- No dead code: never leave commented-out code, unused structs, or `todo!()` macros in committed files.

## 3. Rust Safety, Compiler Lints & PyO3 Boundary

### The `#![forbid(unsafe_code)]` Crate Split Rule (CRITICAL):
`alpha-core`, `exchange-client`, `alpha-node`, and `alpha-watchdog` MUST include these exact crate-level attributes at the top of `src/lib.rs` or `src/main.rs`:

```rust
#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
```

- **Exception for `alpha-py` ONLY:** Because PyO3 procedural macros (`#[pyfunction]`, `#[pymodule]`) expand into unsafe FFI code internally, `alpha-py/src/lib.rs` must not use `#![forbid(unsafe_code)]`. Instead, `alpha-py` must use `#![deny(unsafe_op_in_unsafe_fn)]`, contain zero manual `unsafe {}` blocks, and contain zero business logic (only type conversions calling `alpha-core`).

### Modern PyO3 (v0.22+) Syntax Only:
Never use deprecated `&PyModule` signatures. Always use the modern `Bound<'_, PyModule>` API:
```rust
use pyo3::prelude::*;

#[pymodule]
fn alpha_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(evaluate_bar, m)?)?;
    Ok(())
}
```

### Zero Panics in Production Paths:
- `.unwrap()`, `.expect()`, `panic!()`, and unchecked slice indexing (`slice[i]`) are strictly banned in library and live binary code. Always use `.get(i)` and return typed `Result<T, AlphaError>`.
- `.unwrap()` / `.expect()` are permitted only inside `#[cfg(test)]` unit test modules.

## 4. Financial Math, Precision & Scale Invariance

### Strict Type Boundary (Decimal vs. f64):
- Money, Prices, Quantities, Fees, Balances, Lot Sizes, Tick Sizes: must use `rust_decimal::Decimal`. Never use `f64` for order prices, quantities, or account equity.
- Statistical Indicators (ER, ATR, Z-Score, Sharpe): may use `f64` inside feature calculations, but every output must be checked with `.is_finite()` before converting to Decimal or comparing against thresholds.

### 100% Scale-Invariant Logic ($100 -> $10,000+):
- Never hardcode dollar amounts (e.g. $10, $100) or fixed coin quantities in strategy or risk code.
- Calculations must be expressed in equity fractions, risk fraction per trade ($r \in [0.006, 0.010]$), structural stop distance ($D_{\text{stop}}$), and $R$-multiples.

### Exchange Filter Rounding Laws:
- Order Quantity ($q$): always floor to exchange `step_size`:
  ```rust
  let steps = (raw_qty / step_size).floor();
  let final_qty = steps * step_size;
  ```
- Order Price ($P$): floor Limit Buy and Stop-Loss trigger prices to `tick_size`, and ceil Take-Profit prices to `tick_size`.
- If `final_qty < min_qty` or `final_qty * entry_price < min_notional`, return `RiskVerdict::AbstainMinNotional`—never force a trade.

## 5. Quantitative Integrity & Zero Look-Ahead

- Temporal Causality: features computed at bar close $t$ may only access data with `close_time <= t`. Signals triggered at bar close $t$ can only execute at or after $t+1$.
- Pipelines must pass the Future-Masking Invariance Test and Shuffled-Return Destruction Test.
- Single Source of Truth: backtests must call compiled `alpha_py` Rust functions—never re-implement Rust logic in Python.
- Atomic Stops & Partial Fills: entries must submit atomic exchange-hosted stops (`attachAlgoOrds` / `OTOCO`). The node must track partial fills because attached stops only activate upon 100% fill.

## 6. Phase Decision Gates (Strict Sequential Order)
Do not write code for Phase $N+1$ until Phase $N$'s gate evidence file (`docs/gates/PHASE-N.md`) is verified and committed:
- **Phase 0:** Fee & Venue Discovery (`phase0_feasibility.py`) -> verify UAE entity, fees, zero-fee pairs, minNotional, and atomic stops on $100.
- **Phase 1:** Historical Data Loader -> download 4+ years of data from `data.binance.vision`.
- **Phase 2:** `alpha-core` & `alpha-py` -> pure Rust features and exits with 100% unit and property test coverage.
- **Phase 3:** Purged Walk-Forward & Statistical Kill Gate -> must achieve $\ge 100$ trades, $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > +0.02R$, and $\text{DSR} > 0.95$. Stop if it fails.
- **Phase 4:** Live Execution Engine -> atomic orders, SQLite WAL state recovery, fault drills.
- **Phase 5:** 8-Week Paper Burn-In on Proxmox.
- **Phase 6:** 6-Month Micro-Live Incubation on $100.

## 7. Micro-Sprint, Git Hygiene & STATE.md Protocol

### Branching & Commits:
- Every micro-sprint (1–2 hours) runs on a dedicated branch: `feat/p<phase>-s<sprint>-<slug>`.
- Conventional Commits only: `feat:`, `fix:`, `test:`, `docs:`, `refactor:`, `chore:`.
- Never merge to `main` unless all checks pass:
  ```bash
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  pytest -q
  ```

### STATE.md Context Handoff:
- At the start of every session, read `STATE.md` first.
- At the end of every completed sprint, update `STATE.md` with current phase, branch, completed checklist items, and exact next micro-step.
