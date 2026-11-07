# Project Alpha v3 (Peer-Reviewed Edition) — High-Patience Asymmetric Trading System & Antigravity Gemini 3.8 Flash Blueprint

**Version:** 3.1.0 (Post-Review Production Specification)  
**Date:** November 7, 2026  
**Location Context:** Ajman / UAE (Federal SCA/CMA & Dubai VARA Regulatory Awareness)  
**Target Coding Agent:** Antigravity Gemini 3.8 Flash (with Mandatory Human Audit Gates on Money Paths)  
**Initial Capital:** $\text{USD } 100.00$  
**Monthly Infrastructure & Data Budget:** $\text{USD } 0.00 / \text{month}$  
**Core Philosophy:** **Extreme Sniper Patience.** The default state of the bot is $100\%$ flat in yield-bearing cash. If our validated statistical edge is not present, the bot does nothing—even if it has to wait for days or weeks.

---

# 0. Synthesis of `PLAN.md` (v1), `PLAN_V2.md` (v2), and Peer Review of `PLAN_V3.md`

This specification merges the disciplined scientific skeleton of `PLAN_V2.md` with the structural alpha upgrades of `PLAN_V3.md`, while stripping out every unverified assumption, circular math claim, and architectural bloat identified in peer review.

## 0.1 What We Keep, What We Fix, and What We Cut

| Audit Area | Flaw Identified in Prior Drafts | Definitive Fix in `PLAN_V3.md` (v3.1) |
|---|---|---|
| **Patience & Trade Frequency** | Bots that force daily trades bleed out in choppy regimes. | **Zero-FOMO Patience Doctrine:** No minimum trade quota. A Regime Efficiency & Chop Filter locks the bot in cash for days or weeks until a high-conviction tail event occurs. |
| **Expectancy Math** | v3 Draft 1 plugged assumed win/scratch/loss rates ($48/37/15\%$) into an equation and called $+0.72R$ a "mathematical proof." | **Removed circular math.** All win/scratch/loss rates are treated as unknown empirical outputs. Realistic net edges in production are $+0.05R\text{ to }+0.25R$ per trade. If out-of-sample testing does not prove $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > 0$, the system stays flat. |
| **The "Scratch Exit" & "0% Loss" Fallacy** | Placing a passive maker limit at original `Entry Price` when a trade is losing after 15m will not fill unless price rebounds. Also, stops after a breakeven ratchet can still gap and incur taker fees + slippage. | **Fixed Mechanics & Pre-Registered Ablation:** A stagnation time-stop exits at the **current touch** (cutting a dead trade at $-0.10R\text{ to }-0.25R$ instead of $-1.0R$). Furthermore, both the Time-Stop Scratch and Breakeven Ratchet are tested as **falsifiable ablation hypotheses** against a standard trailing baseline so they are only enabled if they empirically improve net Sortino/expectancy without choking winners. |
| **Sample Size vs. Over-Filtering** | Requiring 4 rare pillars to align simultaneously yields $< 10$ trades/year (impossible to validate statistically) and relies on historical liquidation ticks that are hard to obtain for free. | **Decoupled Hierarchical Hypotheses ($H_1 \dots H_4$):** Each hypothesis is tested independently on $4+$ years of free historical data (`data.binance.vision` & OKX/Coinbase archives) and **must produce $\ge 100$ independent trades** to pass statistical validation. |
| **Unverified Fee & Yield Claims** | Assuming zero-fee `BTC/FDUSD` or $5\%\text{--}8.5\%$ APY exists permanently without checking your specific UAE entity. | **Phase 0 Dynamic Discovery:** Zero-fee maker pairs and Simple Earn / Auto-Earn APY are queried dynamically via live API in Phase 0. The cost model always uses the **actual live fee tier** returned by your authenticated account. |
| **Free Hosting Traps** | 1) Oracle Cloud reclaims Always Free VMs if CPU/network stays $< 20\%$. 2) Cloudflare Workers with 24/7 WebSockets can hit free-tier duration limits. | **Cron-Poll Serverless + Anti-Reclaim Cloud:** Because decisions run on 15m/1h bar closes and stops live on the exchange matching engine, Cloudflare Workers can run via **stateless Cron Triggers** ($< 2\%$ of free limit, zero 24/7 WebSocket billing). For OCI Always Free, we document the exact Pay-As-You-Go $\$0.00$ lock and CPU keep-alive rules, plus your local **Proxmox LXC** container. |
| **Architecture Bloat & Spot Key Myth** | 1) Binance SBE binary decoding + ONNX runtime in Rust is overkill for 15m–4h bars. 2) Spot exchanges do not offer "reduce-only" API keys. | **De-Bloated Stack:** Standard JSON REST/WS + pure-Rust linear/tree weight evaluation (zero SBE/ONNX bloat). Replaced the "reduce-only spot key" myth with audited **compile-time command restriction** in `alpha-watchdog` + exchange-hosted OCO stops. |

---

# 1. The "Zero-FOMO Sniper Patience" Doctrine (Waiting Days is Success)

Retail bots lose money because they try to extract profit from markets that are currently offering $0\text{ bps}$ of edge. Crypto markets spend roughly $70\%\text{ to }80\%$ of the time in noisy, mean-reverting, fee-bleeding chop, and $20\%\text{ to }30\%$ of the time in high-momentum directional expansions or violent liquidation flushes.

## 1.1 The 3-Gate Abstention Filter (Why the Bot Happily Waits for Days)

Before any strategy hypothesis is even allowed to evaluate an entry, the market state must pass three hard **Regime Abstention Gates**:

1. **Gate 1 — Kaufman Efficiency Ratio ($\text{ER}_t$) / Anti-Chop Filter:**
   Over rolling window $N = 24$ bars, measure directional displacement divided by total path length:
   $$\text{ER}_{t, N} = \frac{|P_t - P_{t-N}|}{\sum_{i=0}^{N-1} |P_{t-i} - P_{t-i-1}| + \epsilon} \in [0, 1]$$
   * **Rule:** For trend/breakout strategies ($H_1, H_2$), if $\text{ER}_{t, 24} < \theta_{\text{chop}}$ (where $\theta_{\text{chop}} \approx 0.30$ is calibrated in walk-forward), the market is in random-walk chop. **Action:** `ABSTAIN` (stay $100\%$ flat in cash).

2. **Gate 2 — Net Expected Edge vs. $3\times$ All-In Friction Gate:**
   Let total round-trip friction be:
   $$C_{\text{rt, bps}} = \text{Fee}_{\text{entry, bps}} + \text{Fee}_{\text{exit, bps}} + \text{Spread}_{\text{bps}} + 2 \times \text{Slippage}_{\text{bps}}$$
   * **Rule:** A trade is prohibited unless the model's conservative predicted gross move $\widehat{M}_{\text{bps}}$ satisfies:
     $$\widehat{M}_{\text{bps}} \ge 3.0 \times C_{\text{rt, bps}}$$
     If we are trading on a standard $16\text{--}20\text{ bps}$ round-trip pair, the expected move must exceed $48\text{--}60\text{ bps}$. If Phase 0 confirms an active $0\text{ bps}$ maker pair, $C_{\text{rt, bps}}$ drops significantly, but the $3\times$ friction multiplier remains mandatory.

3. **Gate 3 — Macro Event & Spread Anomaly Blackout:**
   * If current bid-ask spread $> 2.5 \times \text{MedianSpread}_{24\text{h}}$, or the current UTC timestamp is within $\pm 30\text{ minutes}$ of a scheduled high-impact macro release (FOMC, US CPI, NFP), or exchange data has any sequence gap: **Action:** `ABSTAIN`.

**Operational Consequence:** It is completely normal and expected for `alpha-node` to log `STATE: FLAT_WAITING_FOR_EDGE` for **3, 7, or 14 consecutive days**. During this entire waiting period, $95\%$ of your quote balance sits inside the exchange's **Auto-Earn / Simple Earn Flexible** facility collecting passive yield with zero market risk.

---

# 2. Venue & UAE Regulatory Reality Check (Phase 0 First, Code Second)

You are operating from **Ajman, UAE**. In the UAE, regulatory jurisdiction and available exchange features depend on the specific entity your account is onboarded under:
* **OKX Middle East Fintech FZE (VARA Regulated):** Full UAE spot and derivatives license (retail perpetuals capped at $5\times$ after suitability assessment). Spot fees are typically $0.08\%$ maker / $0.10\%$ taker on USDT pairs.
* **Binance (Binance FZE / Global / SCA Coverage):** Depending on whether a UAE resident outside Dubai is onboarded under Binance FZE or international entities, available promotional zero-fee pairs (`BTC/FDUSD`, `BTC/USDC`, `ETH/FDUSD`) and `Simple Earn` eligibility vary.

### The Phase 0 Rule
We **never** hardcode assumptions about zero-fee pairs or yield rates. In **Phase 0 (Day 1)**, before writing any strategy code, the agent runs `python/scripts/phase0_feasibility.py` using your read-only API keys on OKX and/or Binance to query:
1. Exact `makerCommission` and `takerCommission` for every `BTC` and `ETH` spot pair (`USDT`, `USDC`, `FDUSD`, `AED`).
2. Exact `minNotional` / `minSz`, `stepSize` / `lotSz`, and `tickSize`.
3. Whether `Auto-Earn` / `Simple Earn Flexible` API endpoints are enabled for your account entity and what the current real-time APY is.
4. Whether exchange-native conditional OCO stops (`STOP_LOSS_LIMIT` / `oco`) and `cancel-all-after` timers are supported on spot for your entity.

**Instrument Decision at $\text{USD } 100$:**
As proven in `PLAN_V2.md`, BTC and ETH perpetual futures contracts often have minimum contract sizes (`ctVal` $\times$ `minSz`) and higher taker fees ($0.25\%$ per side in UAE) that make them unsuitable for a $\text{USD } 100$ conservative account. Therefore, **Live v3 is strictly Spot Long-or-Flat on `BTC` and `ETH`**.

---

# 3. Falsifiable Strategy & Exit Hypotheses (Tested Under Strict v2 Gates)

To avoid the sample-size collapse criticized in peer review, we do **not** chain 12 conditions into one untestable mega-rule. Instead, we test four independent, falsifiable hypotheses ($H_1 \dots H_4$) in sequential order using $4+$ years of free historical data (`data.binance.vision` + OKX public archives + Coinbase public candles).

Every hypothesis **must generate $\ge 100$ independent trades** across the historical test sample or it fails automatically for insufficient statistical power.

## 3.1 Entry Hypotheses (Tested Sequentially)

| ID | Hypothesis Name | Exact Mathematical Logic | Free Historical Data Used | Pre-Registered Kill Rule |
|---|---|---|---|---|
| **$H_1$** | **Regime-Gated Session Volatility Breakout (Baseline Core)** | Enter long on `1h` (or `4h`) bar close when: 1) $\text{ER}_{t, 24} \ge \theta_{\text{chop}}$ (non-choppy regime), 2) Rolling 20-bar volatility is expanding out of a squeeze ($P_{\text{close}} > \text{EMA}_{20} + k \cdot \text{ATR}_{14}$), and 3) Current UTC hour falls inside institutional liquidity windows (Asia/EU/US opens, tested vs. 24h baseline). | 4+ years of 15m/1h/4h Spot Candles (`BTC`, `ETH`) | Fails if walk-forward net Sharpe $95\%$ CI includes $0$, or $< 100$ trades, or fails to beat random entries with identical trade count. |
| **$H_2$** | **Cross-Venue Coinbase Institutional Lead-Lag Filter (Ablation on $H_1$)** | Compute Coinbase USD Spot vs. Execution Stablecoin Spot Premium $Z$-score ($Z_{\text{CPI}, t}$) and 15m Volume-Weighted Taker Flow Divergence ($\text{CVTFD}_t$). Test whether filtering or weighting $H_1$ entries by $Z_{\text{CPI}, t} > \theta_{\text{cpi}}$ improves out-of-sample net expectancy and reduces false breakouts. | Coinbase Public `BTC-USD` & `ETH-USD` 1m/15m candles + Execution Venue candles (4+ years free via public REST/archives) | If adding $Z_{\text{CPI}, t}$ does not increase out-of-sample net expectancy and Deflated Sharpe over $H_1$ alone by a statistically significant margin ($p < 0.05$) while keeping $\ge 100$ trades, **delete the Coinbase feed**. |
| **$H_3$** | **Derivatives Crowding & Capitulation Flush Reversal** | Mean-reversion long entry after extreme derivatives deleveraging: 1) 8h/1h Funding Rate $Z$-score $Z_{F, 7\text{d}} < -1.8$ (crowded shorts paying longs), 2) 15m/1h Open Interest drop $\Delta \text{OI} \le -2.0\%$ (leveraged flush), and 3) Capitulation volume climax ($V_{t} > 3.0 \times \text{SMA}(V, 24)$) with bullish close recovery. *(Note: Uses free historical OI + Funding + Volume climax so we have 4+ years of history; live recorder additionally logs real-time liquidation streams for future refinement).* | Free Binance `data.binance.vision` `fundingRate` + `metrics` (Open Interest) + 15m/1h candles | Fails if $< 100$ historical events across `BTC` + `ETH` or if lower $95\%$ bootstrap bound of net expectancy $\le 0$ after taker/maker costs. |
| **$H_4$** | **Regularized Linear Meta-Labeler** | L2-regularized Logistic Regression (Platt-calibrated) trained on Purged CV to predict $\mathbb{P}(\text{Net Trade Return} > 0 \mid \text{Signal } H_1 \text{ or } H_3)$. Trades only when $p_t \cdot \mathbb{E}[W] - (1-p_t)\cdot \mathbb{E}[L] - C_{\text{rt}} > 0$. | Features stamped strictly at $t \le t_{\text{decision}}$ | Fails if it does not improve net out-of-sample Calmar/Sortino ratio over the raw unfiltered $H_1 / H_3$ signals on the untouched 6-month holdout. |

## 3.2 Exit & Loss-Minimization Hypotheses (Solving the "Scratch Hole" Honestly)

Our goal is to minimize losing trades and make losses negligible **when doing so actually increases net profitability**. Peer review correctly noted two dangers with naive breakeven scratches:
1. A passive limit order placed at `Entry Price` when price is currently below entry will not fill in a losing trade.
2. Pulling a stop-loss to breakeven too early (`+0.4 ATR`) can choke off winning trend trades during normal pullbacks.

Therefore, we define **physically executable exit mechanics** and test them via strict ablation against a standard baseline exit:

* **Baseline Exit ($E_0$ — Pure ATR Hard Stop + Chandelier Trail):**
  * Initial Exchange-Hosted Hard Stop: $P_{\text{stop}} = P_{\text{entry}} - k_{\text{stop}} \cdot \text{ATR}_{14}$ (where $k_{\text{stop}} \in [1.5, 2.5]$).
  * Trailing Exit: Chandelier Exit at $\max_{s \in [t_0, t]}(P_{\text{high}, s}) - k_{\text{trail}} \cdot \text{ATR}_{14}$.

* **Ablation Exit 1 ($E_1$ — "Active Touch Time-Stop Scratch" for Stagnant Trades):**
  * **Why it works physically:** Momentum and capitulation-bounce trades have strong time decay. If price has not moved at least $+0.25 \cdot \text{ATR}_{14}$ in our favor after $M$ bars (e.g., $M \in \{1, 2, 3\}$ bars), the catalyst has failed.
  * **How it executes without the passive-limit hole:** Instead of placing an unfillable limit order back at `Entry Price`, the engine exits **at the current market touch** (a short 10-second `Post-Only` limit at the *current* best ask, immediately falling back to a collared marketable limit at the current best bid).
  * **Result:** Cuts a dead trade at a tiny fraction of a stop ($-0.10R\text{ to }-0.25R$, or $+0.05R$ if slightly green) instead of sitting around waiting to take a full $-1.00R$ hard stop.
  * **Kill Rule:** $E_1$ is adopted **only if** walk-forward testing proves that enabling $E_1$ increases net out-of-sample expectancy and Sortino ratio compared to $E_0$.

* **Ablation Exit 2 ($E_2$ — "Delayed Breakeven Ratchet + Partial Harvest"):**
  * Once unrealized profit reaches $+k_{\text{ratchet}} \cdot \text{ATR}_{14}$ (tested across $k_{\text{ratchet}} \in \{0.8, 1.0, 1.2, 1.5\}$ so trades have room to breathe), amend the exchange-hosted stop-loss to:
    $$P_{\text{stop, new}} = P_{\text{entry}} \times \left(1 + f_{\text{entry}} + f_{\text{exit, taker}} + s_{\text{slippage\_buffer}}\right)$$
  * **Honest Risk Accounting:** We never claim a ratcheted stop has "$0\%$ loss probability." In backtests and live risk math, a ratcheted stop is modeled with worst-case taker fees ($f_{\text{exit, taker}}$) plus a gapping/slippage penalty ($s_{\text{slippage\_buffer}} \ge 5\text{ bps}$, and $15\text{ bps}$ in high-volatility stress tests).
  * **Kill Rule:** $E_2$ is adopted **only if** it beats $E_0$ on net out-of-sample profit and drawdown-adjusted return.

---

# 4. De-Bloated Architecture & Honest Security Model

For a strategy operating on 15m/1h/4h bars and waiting patiently for days between trades, microsecond SBE binary parsers, ONNX C-bindings, and 6 concurrent WebSockets add failure modes without adding edge. We strip the architecture down to a **rock-solid, minimal Rust core**.

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                     PUBLIC & PRIVATE EXCHANGE ENDPOINTS                      │
│   Primary Venue (OKX or Binance REST/WS)  +  Optional Coinbase USD Feed      │
└───────────────────┬──────────────────────────────────────┬───────────────────┘
                    │                                      │
                    ▼                                      ▼
┌─────────────────────────────────────────┐  ┌─────────────────────────────────┐
│     PROCESS 1: `alpha-node` (Rust)      │  │ PROCESS 2: `alpha-watchdog`     │
│                                         │  │ (150-Line Minimal Rust Binary)  │
│  1. Bar & Book Ingest (JSON + Decimal)  │  │                                 │
│  2. `alpha-core` Features & Regime Gate │  │ - Checks `alpha-node` heartbeat │
│     (Waits in Cash if ER_t < threshold) │  │ - Refreshes Exchange Dead-Man   │
│  3. Independent Risk Gate               │  │   `cancel-all-after` timer      │
│  4. Order FSM + Exchange-Hosted Stops   │  │ - Compile-time restricted to    │
│  5. Idle Cash Auto-Earn Manager         │  │   Cancel & Emergency Flatten    │
│  6. Async Parquet Recorder              │  │ - Pings Healthchecks.io + Tg    │
└───────────────────┬─────────────────────┘  └─────────────────▲───────────────┘
                    │                                          │
                    ▼                                          │
┌──────────────────────────────────────────────────────────────┴───────────────┐
│              SQLite (WAL Mode) State DB  +  Parquet Data Archive             │
└──────────────────────────────────────────────────────────────────────────────┘
```

## 4.1 Why This Stack Is Safer and Better Suited for Antigravity Gemini 3.8 Flash

1. **One Shared Strategy Library (`alpha-core` via `PyO3`):**
   * Written in pure, safe Rust (`#![forbid(unsafe_code)]`) using `rust_decimal::Decimal` for all price/quantity/fee math.
   * Compiled into a Python module via `maturin` so offline Python walk-forward backtests execute the **exact same compiled Rust code** that runs in production.
2. **Zero ONNX / Binary Protocol Bloat:**
   * If $H_4$ (Logistic Meta-Labeler) passes validation, its model is just $K$ floating-point weights and a Platt-scaling sigmoid $(a, b)$ saved in a SHA-256 signed `model_weights.json` file. Evaluating $\sigma(a \cdot (\mathbf{w}^\top \mathbf{x} + b_0) + b)$ takes 15 lines of auditable Rust code with zero external C/WASM ML libraries.
3. **Honest API Key Security on Spot Exchanges:**
   * Spot exchanges (Binance Spot, OKX Spot) do **not** offer a "reduce-only" API key permission on spot wallets (any key with `Spot Trading` permission can theoretically buy or sell).
   * Instead of pretending the exchange enforces a reduce-only spot key, we enforce isolation at two real levels:
     1. **Exchange-Hosted Protection First:** Every entry immediately attaches an exchange-hosted OCO / conditional stop-loss order and refreshes the exchange's native `cancel-all-after` dead-man timer. Even if your server explodes, the exchange matching engine holds your stop.
     2. **Compile-Time Restricted Watchdog (`alpha-watchdog`):** The watchdog binary does not import `alpha-venues::submit_entry_order`. Its REST client struct literally only compiles two methods: `cancel_all_open_orders()` and `marketable_limit_sell_entire_base_balance()`.

---

# 5. Priority #1 Hosting: Realistic $\$0.00/\text{Month}$ Online Deployment (Without Buying a VPS)

Per your requirements, we want the bot online 24/7 for $\$0.00/\text{month}$, ideally without needing your local Proxmox server powered on—while avoiding the **Oracle Cloud idle reclamation** and **Cloudflare WebSocket billing** traps highlighted in peer review.

Here are the three verified execution modes, ranked by priority:

## 5.1 Mode 1 (True Zero-VM Serverless): Cloudflare Workers Cron + Turso SQLite (`$0.00/mo`)

* **How we avoid Cloudflare's WebSocket limits completely:**
  Because our validated hypotheses ($H_1, H_2, H_3$) evaluate on **15-minute / 1-hour bar closes** and our protective stop-losses are **hosted directly on the exchange's matching engine**, the bot does **not** need a 24/7 persistent outbound WebSocket in production!
* **Execution Flow:**
  1. A **Cloudflare Worker Cron Trigger** (`*/5 * * * *` or `*/15 * * * *`) wakes up every 5 or 15 minutes (and every 1 minute only while an active trade is in its early scratch window).
  2. It fetches the latest candles, order-book top, funding/OI, and private order state via fast HTTPS REST calls (< 400 ms total wall time), reads state from **Turso Free Tier SQLite**, runs `alpha-core`, and updates orders if needed.
* **Free Tier Math:** Running once every 5 minutes is only **$288\text{ invocations/day}$**—which is **$0.28\%$ of Cloudflare's $100{,}000\text{ requests/day}$ Free Tier** and $0\%$ WebSocket duration charge. It cannot be reclaimed for idleness and never depends on your home internet.

## 5.2 Mode 2 (24/7 Persistent Stream + Recorder): Oracle Cloud Always Free ARM (`me-dubai-1` / `eu-frankfurt-1`) with Anti-Reclaim Lock

If you want 24/7 tick/book recording alongside live trading on a dedicated Linux box without using your home Proxmox server:
* **The OCI Idle Reclamation Trap & Exact Fix:** Oracle reclaims "Always Free" compute instances on Free Tier accounts if CPU, memory, and network stay below $20\%$ over a 7-day window.
  * **Fix A (Account Level):** Upgrade the OCI account type to *Pay-As-You-Go (PAYG)* while provisioning **strictly within the Always Free resource limits** (up to 4 Ampere A1 cores, 24 GB RAM, 200 GB disk) and set a hard OCI Budget Alert at $\text{USD } 0.01$. Oracle officially does **not** reclaim idle Always Free instances on PAYG-verified accounts, and your monthly bill remains $\text{USD } 0.00$.
  * **Fix B (Workload Level):** Schedule the nightly Python walk-forward verification and Parquet compression job via `systemd` timer on the instance so rolling 95th-percentile CPU utilization naturally stays $> 20\%$.

## 5.3 Mode 3 (100% Self-Hosted Fallback): Local Proxmox Debian 12 LXC Container

Since you already own a Proxmox server, `infra/proxmox-lxc/` provides a 1-command script to spin up an unprivileged Debian 12 LXC container ($256\text{ MB}$ RAM) with `chrony` time sync and `Healthchecks.io` dead-man monitoring. You can run Phase 1–3 data recording and heavy backtests on Proxmox, and deploy the lightweight live engine to Cloudflare Workers or OCI.

---

# 6. Risk Engine, Position Sizing & Idle Yield Specification

## 6.1 Conservative Risk Limits

| Control | Specification |
|---|---|
| **Instrument & Leverage** | Spot Long-or-Flat Only (`1.0x`, zero margin borrowing, zero shorting) |
| **Default State** | `FLAT_EARNING` (100% in quote currency inside Auto-Earn / Simple Earn when no edge is active) |
| **Risk Budget Per Full Stop ($r_{\text{max}}$)** | **$0.60\%$ of current equity** ($\text{USD } 0.60$ at $\text{USD } 100$) |
| **Max Open Positions** | $1$ across the entire account |
| **Max Entries Per Day** | $2$ (prevents overtrading and fee bleed) |
| **Max Entries Per Week** | $6$ (enforces high-selectivity sniper patience) |
| **Daily Loss Stop** | $-1.20\%$ of start-of-day equity $\rightarrow$ Flatten & lock out for $24\text{ hours}$ |
| **Weekly Loss Stop** | $-2.40\%$ of start-of-week equity $\rightarrow$ Flatten & lock out for $72\text{ hours}$ |
| **Hard High-Water-Mark Drawdown Halt** | **$-6.00\%$ from equity peak** $\rightarrow$ Permanent `HALT` requiring manual human review & signed re-arm |

## 6.2 Exact Cost-Aware Position Sizing Formula

For account equity $E$, risk fraction $r = 0.006$, intended entry price $P_e$, hard stop price $P_s$, actual account maker/taker fee rates $f_e, f_x$, and conservative stop-exit slippage $s_x$ (minimum $5\text{ bps}$):

$$L_{\text{unit}} = |P_e - P_s| + P_e f_e + P_s (f_x + s_x)$$

$$q_{\text{raw}} = \min\left(\frac{E \cdot r}{L_{\text{unit}}}, \ \frac{0.98 \cdot E}{P_e}\right)$$

$$q_{\text{final}} = \left\lfloor \frac{q_{\text{raw}}}{\text{stepSize}} \right\rfloor \times \text{stepSize}$$

**Pre-Submission Rejections:**
1. If $q_{\text{final}} < \text{minQty}$ or $q_{\text{final}} \cdot P_e < \text{minNotional}$, **SKIP TRADE** (never raise risk $r$ to meet `minNotional`).
2. If stop distance $\frac{|P_e - P_s|}{P_e} < 3 \times C_{\text{rt}}$, **SKIP TRADE**.

## 6.3 Idle Cash Yield ("Paid to Wait for Days")

When `Position == 0` for more than $15\text{ minutes}$ and Phase 0 confirms the endpoint is available for your entity:
* **OKX:** Enable `set-auto-earn` on the quote balance so funds earn passive exchange yield automatically while remaining available as trading collateral.
* **Binance:** Subscribe idle quote balance (leaving a $\text{USD } 5$ buffer) to `Simple Earn Flexible` (`POST /sapi/v1/simple-earn/flexible/subscribe`), and call instant redemption (`POST /sapi/v1/simple-earn/flexible/redeem`) prior to placing an entry order.
* *Honest expectation:* On $\text{USD } 100$, passive yield adds roughly $\text{USD } 3\text{ to USD } 8\text{ per year}$ depending on prevailing rates—not a fortune, but it offsets exchange fees while the bot waits days for A+ setups.

---

# 7. Validation Protocol & Statistical Gates (Restoring Full v2 Rigor)

No hypothesis moves from Research to Paper, or from Paper to Micro-Live, without passing every gate below.

## 7.1 Mandatory Statistical Requirements

1. **Minimum Sample Size Gate ($\ge 100$ Independent Trades):**
   * Any hypothesis producing fewer than $100$ independent trades across the historical backtest period is **statistically unproven** and cannot be promoted.
2. **Chronological Walk-Forward + Untouched 6-Month Holdout:**
   * Purged and embargoed walk-forward splits (minimum $24\text{h}$ purge / $48\text{h}$ embargo).
   * The most recent $6\text{ months}$ of data are locked in a vault and evaluated **once** per candidate strategy.
3. **Conservative Cost & Queue Modeling:**
   * First-pass backtests must remain net-positive even when assuming **taker fees on entries** and $5\text{ bps}$ slippage per side.
   * For `Post-Only` maker simulation, an order is only counted as filled if historical prices trade *through* our limit price by at least $1\text{ tick}$ (or top-of-book volume crosses $2\times$ our order size).
4. **Deflated Sharpe Ratio ($\text{DSR}$) & Block-Bootstrap Lower Bound:**
   * Log every trial in `experiments.sqlite`. Report the Bailey–Lopez de Prado Deflated Sharpe Ratio ($\text{DSR} > 0.95$) and 10,000-sample Politis–Romano stationary block-bootstrap $95\%$ confidence interval for net expectancy per trade.
   * **Gate:** The **lower $95\%$ confidence bound** of net trade expectancy after all costs must be strictly $> 0$.
5. **Parameter Perturbation & Regime Survival:**
   * Varying every strategy parameter ($\theta_{\text{chop}}$, $\text{ATR}$ multiplier, lookback window) by $\pm 20\%$ must not flip net expectancy negative.
   * Strategy must show positive net expectancy across at least 3 distinct market regimes (bull expansion, bear flush, sideways range).

## 7.2 Promotion Gates & Realistic Timeline

| Gate Transition | Strict Requirement |
|---|---|
| **Phase 0 $\rightarrow$ Phase 1** | `docs/gates/PHASE_00_FEASIBILITY.md` generated from live exchange API confirming your UAE entity, exact maker/taker fees, any zero-fee pairs, and `minNotional` compatibility at $\text{USD } 100$. |
| **Research $\rightarrow$ Paper (`Phase 3 -> 5`)** | At least one hypothesis ($H_1 \dots H_4$) passes $\ge 100$ trades, $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > 0$ on walk-forward AND untouched 6-month holdout, $\text{DSR} > 0.95$, $\pm 20\%$ parameter perturbation survival, and zero look-ahead leakage in CI. **If no hypothesis passes, the project stops here and no live trading engine is deployed.** |
| **Paper $\rightarrow$ Micro-Live (`Phase 5 -> 6`)** | **Minimum 8 weeks of continuous paper operation** (and at least $50\text{--}100$ paper trades, extending weeks if trade frequency is lower due to patience rules); paper fill rates and slippage reconcile with live order book; all 4 kill-switch layers drilled; **human code audit completed on `sizing.rs`, `risk.rs`, and `execution.rs`**. |
| **Micro-Live $\rightarrow$ Capital Scaling** | **Minimum 8 weeks live at $\text{USD } 100$**; realized net expectancy within the paper confidence interval; manual operator sign-off in steps of at most $2\times$. |

---

# 8. Complete Guide & Guardrails for **Antigravity Gemini 3.8 Flash**

Because **Antigravity Gemini 3.8 Flash** is optimized for high speed, it must follow strict structural guardrails so speed never compromises financial safety.

## 8.1 Mandatory Coding Rules for Antigravity Gemini 3.8 Flash

1. **Small, Auditable Files ($< 300\text{ Lines}$ per File):**
   * Never write monolithic 1,500-line files. Keep every Rust module under $300\text{ lines}$ with a single clear responsibility so both the model and human reviewer can verify every branch.
2. **Compiler-Enforced Safety (`Cargo.toml` Lints):**
   * Every crate in `rust/crates/` must include at the top of `lib.rs` / `main.rs`:
     ```rust
     #![forbid(unsafe_code)]
     #![deny(
         clippy::unwrap_used,
         clippy::expect_used,
         clippy::panic,
         clippy::indexing_slicing,
         clippy::float_arithmetic
     )]
     ```
   * *(Note: Allow `clippy::float_arithmetic` only inside isolated statistical indicator functions in `features.rs`, never in `sizing.rs`, `risk.rs`, or `execution.rs` where `rust_decimal::Decimal` is mandatory).*
3. **Mandatory Human Audit Gate on 3 Files:**
   * Antigravity Gemini 3.8 Flash must place a header comment `// HUMAN_AUDIT_REQUIRED: DO NOT DEPLOY WITHOUT OPERATOR REVIEW` at the top of:
     1. `rust/crates/alpha-core/src/sizing.rs`
     2. `rust/crates/alpha-core/src/risk.rs`
     3. `rust/crates/alpha-node/src/execution.rs`
4. **Property-Based Testing Before Any Implementation Is Marked Complete:**
   * Every function in `sizing.rs` and `risk.rs` must have a `proptest` suite running $10{,}000$ randomized inputs proving that:
     * Output order notional never exceeds `equity * max_leverage`.
     * Output risk never exceeds `equity * max_risk_per_trade_fraction` ($0.60\%$).
     * Rounding always rounds **down** (`RoundingStrategy::ToZero`) to `stepSize`.
     * When `ER_t < chop_threshold` or `expected_move < 3 * round_trip_cost`, the strategy always returns `Action::AbstainWait`.
5. **Never Guess API Schemas or Fabricate Results:**
   * Always capture real JSON responses from the exchange into `tests/fixtures/` first, write deserialization tests against those exact files, and never invent backtest metrics.

## 8.2 Target Repository Layout

```text
project-alpha-v3/
├── Cargo.toml                        # Rust workspace
├── pyproject.toml                    # Python research dependencies (polars, duckdb, lightgbm, scipy, maturin)
├── .env.example                      # Env var names only
├── PLAN_V3.md                        # This peer-reviewed specification
├── docs/
│   ├── decisions/                    # 0001-venue-entity.md, 0002-hosting-mode.md
│   ├── gates/                        # PHASE_00_FEASIBILITY.md .. PHASE_06_LIVE.md
│   ├── research/                     # Pre-registered hypotheses H1..H4 & ablation reports
│   └── runbooks/                     # kill-switch-drill.md, recovery.md
├── configs/
│   ├── paper.toml                    # Default safe config (allow_live_orders = false)
│   ├── production.toml               # Requires signed LIVE_APPROVED file
│   └── calendar_blackouts.json       # Scheduled FOMC, CPI, NFP UTC timestamps
├── migrations/
│   └── 0001_state.sql                # SQLite WAL schema (orders, fills, positions, halt_flag, trials)
├── rust/
│   └── crates/
│       ├── alpha-types/              # Decimal wrappers, Bar, OrderBookTop, TradeProposal, AlphaError
│       ├── alpha-core/               # ER_t Chop Gate, Features, H1-H4 Rules, Sizing, Risk Gate, PyO3
│       ├── alpha-venues/             # OKX v5 & Binance Spot REST/WS adapters + Coinbase public feed
│       ├── alpha-node/               # Recorder, Cron/Stream Evaluator, Execution FSM, Auto-Earn
│       └── alpha-watchdog/           # Compile-time restricted Cancel/Flatten supervisor + Telegram
├── python/
│   ├── scripts/
│   │   └── phase0_feasibility.py     # Queries live fees, minNotional, Auto-Earn APY, and entity rules
│   └── research/
│       ├── download_history.py       # Downloads 4+ years of free Binance/OKX/Coinbase data
│       ├── backtest.py               # Calls compiled `alpha_core` Rust module via PyO3
│       ├── ablation.py               # Tests H2 (Coinbase CPI) and E1/E2 (Scratch/Ratchet) vs baselines
│       └── validator.py              # Enforces >=100 trades, DSR, Bootstrap LCB, Leakage & Perturbation
└── tests/
    ├── fixtures/                     # Captured real API responses from Phase 0
    ├── proptest_money/               # 10,000-case property tests for sizing, risk, and patience gates
    ├── leakage/                      # Timestamp truncation & shuffled-label CI tests
    └── fault_injection/              # Disconnects, stop-ack timeouts, partial fills, duplicate events
```

## 8.3 Phase-by-Phase Execution Checklist for Antigravity Gemini 3.8 Flash

1. **Phase 0 (Week 1) — Live Feasibility & Fee Discovery:**
   * Implement and run `python/scripts/phase0_feasibility.py`.
   * Record real maker/taker fees, zero-fee pair availability (`FDUSD`/`USDC`/`USDT`), `minNotional`, and Auto-Earn eligibility in `docs/gates/PHASE_00_FEASIBILITY.md`.
2. **Phase 1 (Weeks 1–2) — Recorder & Deterministic Replayer:**
   * Build `alpha-types`, `alpha-venues`, and the Parquet/SQLite recorder in `alpha-node`.
   * Pass a 7-day soak test with forced reconnects and sequence-gap recovery (`cargo test --test replay_determinism`).
3. **Phase 2 (Weeks 2–4) — `alpha-core` Rust Engine + PyO3 Research Kit + Leakage CI:**
   * Implement the Patience/Chop Gate ($\text{ER}_t$), $3\times$ Cost Gate, Sizing, and Risk Engine in Rust.
   * Expose to Python via `maturin develop --release` and pass all `proptest_money` and `leakage` tests.
4. **Phase 3 (Weeks 4–8) — Pre-Registered Hypotheses ($H_1 \dots H_4$) & Exit Ablation ($E_0, E_1, E_2$):**
   * Download 4+ years of free historical candles, funding rates, and open interest.
   * Test $H_1$ (Session Volatility Breakout), $H_2$ (Coinbase Lead-Lag ablation), $H_3$ (Funding/OI Capitulation Bounce), and $E_1/E_2$ (Active Touch Time-Stop Scratch & Delayed Ratchet ablation).
   * **Hard Stop Gate:** Only promote components that empirically pass $\ge 100$ trades, $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > 0$ on both walk-forward and the untouched 6-month holdout, and $\text{DSR} > 0.95$. If nothing passes, write the no-edge report and stop.
5. **Phase 4 (Weeks 8–11) — Execution FSM, Exchange-Hosted Stops & Watchdog Drills:**
   * Implement order lifecycle, exchange-hosted OCO/conditional stops, `cancel-all-after` dead-man timer, Auto-Earn subscription/redemption, and `alpha-watchdog`.
   * Perform mandatory **Human Code Review** on `sizing.rs`, `risk.rs`, and `execution.rs`, and pass all fault-injection tests on Demo/Testnet.
6. **Phase 5 (8+ Weeks) — Continuous Paper Trading Gate:**
   * Deploy to your zero-cost host (Cloudflare Cron Worker, OCI Always Free with PAYG lock, or Proxmox LXC).
   * Run for at least 8 weeks and $\ge 50\text{--}100$ paper trades, reconciling simulated fills against live order-book depth and verifying that the bot sits patiently in cash during choppy weeks.
7. **Phase 6 (8+ Weeks) — Micro-Live at $\text{USD } 100$:**
   * Enable live trading via signed `LIVE_APPROVED` file at $0.60\%$ risk per trade ($\text{USD } 0.60$ max stop loss). Monitor daily Telegram digests before any future capital scaling.