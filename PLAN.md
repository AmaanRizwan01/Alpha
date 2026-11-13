# Project Alpha v3 (Final Production & Scale-Up Edition) — Scale-Invariant Asymmetric Trading System

**Version:** 3.2.0 (Final Master Specification)  
**Date:** November 7, 2026  
**Location Context:** Ajman, UAE (UAE SCA/CMA & Dubai VARA Regulatory Compliance)  
**Primary Infrastructure:** Local Proxmox VE Server (Debian 12 LXC) + Exchange-Side Atomic Protection  
**Capital Roadmap:**  
- **Stage 1 (Months 1–6 Incubation):** $\text{USD } 100.00$ (Evaluated strictly in $\%$ return, $\text{bps}$, and $R$-multiples)  
- **Stage 2 (Post-6-Month Gate):** $\text{USD } 5{,}000.00 \rightarrow \text{USD } 10{,}000.00+$ (Zero code changes required)  
**Core Operating Law:** **Extreme Sniper Patience.** If our validated statistical edge is not present, the bot does nothing and remains $100\%$ flat in yield-bearing cash—even if it has to wait for days or weeks.

---

# 0. Executive Summary & Synthesis of All Audits

This document is the final, unified engineering and quantitative blueprint synthesizing `PLAN.md` (v1), `PLAN_V2.md` (v2), and two rounds of adversarial technical review.

## 0.1 Why We Judge Everything in Percentages ($\%$), Basis Points ($\text{bps}$), and $R$-Multiples

On $\text{USD } 100$, dollar figures look small ($+30\%$ is $\text{USD } 30$), whereas on $\text{USD } 10{,}000$, that exact same $+30\%$ is $\text{USD } 3{,}000$. Because the sole goal of the $\text{USD } 100$ phase is to **prove durable percentage profitability over 6 months before deploying $\text{USD } 5{,}000\text{--}10{,}000$**, the entire system is engineered to be **Scale-Invariant**:

1. **No Dollar Constants in Code:** Every position size, stop distance, fee budget, slippage allowance, and drawdown limit is expressed strictly as a fraction of current equity ($E_t$), basis points of notional ($\text{bps}$), or multiples of initial trade risk ($1R$).
2. **Liquidity-Scalable Universe From Day 1:** We trade only **BTC and ETH Spot** (`BTC/USDT`, `ETH/USDT`, or Phase-0 verified zero-fee `FDUSD`/`USDC` equivalents). A strategy that works on illiquid micro-caps at $\text{USD } 100$ will collapse under its own market impact at $\text{USD } 10{,}000$. On `BTC` and `ETH`, a $\text{USD } 5{,}000\text{--}10{,}000$ order represents $< 0.2\%$ of top-5-level order-book depth, meaning execution slippage at $\text{USD } 10{,}000$ is virtually identical to $\text{USD } 100$.
3. **Quantization Improvement at Scale:** At $\text{USD } 100$, exchange lot-size rounding (`stepSize = 0.00001 BTC` $\approx \text{USD } 0.75$) introduces up to $\sim 1.5\%$ sizing quantization error. At $\text{USD } 5{,}000\text{--}10{,}000$, quantization error drops below $0.03\%$, meaning live execution tracks theoretical percentage math even more closely as you scale.

## 0.2 Summary of All Architectural & Quantitative Fixes in Final v3.2

| Component | Flaw in Earlier Versions | Final Production Solution in v3.2 |
|---|---|---|
| **Hosting Infrastructure** | Oracle Cloud rejects many signups; GCP Free Tier is US-only (geo-blocked by OKX/Binance); Cloudflare Cron breaks 20s maker loops, lacks static IPs for key allowlisting, and can't record order books. | **100% Proxmox LXC + Atomic Exchange-Side Orders:** Runs on your local Proxmox server in Ajman (native UAE IP, 24/7 WebSockets, $0/mo) paired with **Atomic Entry+Stop (`attachAlgoOrds` / `OTOCO`)** and **`cancel-all-after`** so a home power or ISP outage can never leave an unprotected position. |
| **Fee-to-Stop Ratio ($C_{\text{rt}} / D_{\text{stop}}$)** | A tight $1\%$ stop with $20\text{ bps}$ ($0.20\%$) round-trip fees loses $0.20R$ to fees on every trade before slippage. | **Structural Swing Stops ($\ge 2.0\%\text{--}3.5\%$) + Maker Priority:** By operating on `1h`–`4h` swing horizons (or Phase-0 verified $0\%$ maker pairs), round-trip maker fees ($16\text{ bps}$ or $0\text{ bps}$) consume $\le 0.05R\text{--}0.08R$ per trade. |
| **Patience & Anti-Chop** | Bots that trade every day get ground to dust in sideways markets. | **3-Gate Abstention Engine:** Uses Kaufman Efficiency Ratio ($\text{ER}_{t,24}$), a $3\times$ All-In Friction Gate, and Macro/Spread blackouts to keep the bot $100\%$ flat in yield-bearing cash for days or weeks until an expansion regime occurs. |
| **Exit Mechanics ("Scratch & Ratchet")** | Draft 1 used a passive limit at entry price (which won't fill in a losing trade) and assumed $0\%$ loss after ratcheting. | **Executable Active-Touch Time Stop ($E_1$) & Delayed Ratchet ($E_2$):** Exits stagnant trades at the *current* bid/ask touch (cutting dead trades at $\approx -0.15R$ instead of $-1.0R$). Tested as pre-registered **ablations** against a standard Chandelier trail ($E_0$). |
| **Statistical Gates (Walk-Forward vs. Holdout)** | Requiring a 95% lower confidence bound ($\text{LCB}_{95\%} > 0$) on a 6-month holdout ($N \approx 35$ trades) rejects real $+0.15R$ edges due to small-$N$ standard error ($\approx 0.16R$). | **Calibrated Two-Stage Gate:** Require $N \ge 100$ trades and $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > 0$ across the **full out-of-sample walk-forward dataset** ($4+$ years), and use the untouched 6-month holdout as a **regime consistency check** ($\text{EV}_{\text{net}} > 0$, drawdown within limit, mean inside walk-forward $95\%$ CI). |
| **Rust `unsafe` vs. `PyO3` Conflict** | Putting `#![forbid(unsafe_code)]` inside `alpha-core` alongside `PyO3` procedural macros fails compilation because `#[pyfunction]` expands to `unsafe` FFI code. | **Clean Crate Split:** `alpha-core` is 100% pure Rust with `#![forbid(unsafe_code)]` and zero FFI dependencies. A separate thin wrapper crate, `alpha-py`, imports `alpha-core` and exposes the `PyO3` bindings to Python. |

---

# 1. Thorough Hosting Research: Why Free Cloud Fails for Crypto Bots & How We Make Proxmox 100% Bulletproof

You noted that Oracle Cloud will not let you sign up, and asked whether any other $0.00/month cloud option exists before resorting to your Proxmox server. Here is the exhaustive 2026 technical audit of every free cloud provider for a 24/7 algorithmic crypto system:

## 1.1 Audit of Non-Oracle Free Cloud Providers

| Provider | Free Tier Offering | Fatal Flaw for an OKX / Binance Trading System | Verdict |
|---|---|---|---|
| **Google Cloud (GCP)** | `e2-micro` Always Free VM | Free only in US regions (`us-east1`, `us-central1`, `us-west1`). Both **Binance (`HTTP 451`)** and **OKX (`HTTP 403`)** strictly geo-block US IP addresses from market/trade APIs. | **Unusable** |
| **Cloudflare Workers / Render / Fly.io / Koyeb** | Serverless / Micro PaaS | 1) **No static egress IP** on free tiers—meaning you cannot IP-allowlist your API key (an unacceptable security risk when scaling to $\text{USD } 10{,}000$). 2) Containers sleep or sever 24/7 WebSockets, breaking order-book recording and dead-man timers. | **Unsafe & Incompatible** |
| **AWS / Azure** | 6–12 Month Trial Credits | Expires before or right as your 6-month incubation finishes, forcing a migration right when you scale to $\text{USD } 5{,}000\text{--}10{,}000$. | **Rejected** |
| **Your Local Proxmox Server (Ajman, UAE)** | Debian 12 LXC ($512\text{ MB}$ RAM, 1 vCore) | Only risk is a local home power or Etisalat/du ISP drop. **100% solved** via Atomic Exchange-Side Orders (`attachAlgoOrds` / `OTOCO`) + Exchange Dead-Man Timer (`cancel-all-after`) + Dynamic IP/Subaccount controls. | **Selected (#1 Best Option)** |

## 1.2 How We Make Your Local Proxmox Server Safer Than a Cloud VPS

Running on your Proxmox server in Ajman gives you a **native UAE residential IP** (passing OKX UAE VARA and Binance UAE regional checks natively with ~15–60 ms latency to Middle East/Europe gateways) and unlimited local NVMe/SSD storage for Parquet order-book archives.

To ensure that a **home power outage, router reboot, or ISP disconnect can never hurt your account**—even when trading $\text{USD } 10{,}000$—we enforce **Three Exchange-Side Hardware-Independence Guarantees**:

1. **Guarantee 1 — Atomic Entry + Stop-Loss Submission (`attachAlgoOrds` / `OTOCO`):**
   * We **never** send an entry order first and wait for a fill event to send a stop-loss order.
   * **On OKX Spot (`POST /api/v5/trade/order`):** Every entry order includes the `attachAlgoOrds` array containing the stop-loss trigger price (`slTriggerPx`) and collared limit/market execution (`slOrdPx: "-1"` or limit collar).
   * **On Binance Spot (`POST /api/v3/orderList/otoco`):** Every entry uses an **OTOCO (One-Triggers-One-Cancels-the-Other)** order list where the working entry order automatically activates the pending exchange-hosted stop-loss (`STOP_LOSS_LIMIT`) and take-profit legs the microsecond the entry fills.
   * **Result:** If your Proxmox server loses power 1 millisecond after sending an entry order, the exchange matching engine *already* holds the linked stop-loss order.
2. **Guarantee 2 — Exchange-Side Dead-Man Timer (`POST /api/v5/trade/cancel-all-after`):**
   * While `alpha-node` has any working unfilled entry order, it calls OKX's `POST /api/v5/trade/cancel-all-after` with `{"timeOut": "60"}` every $20\text{ seconds}$.
   * If your Proxmox server or home internet goes offline, the exchange automatically cancels all working unfilled entry orders after $60\text{ seconds}$, while leaving active `attachAlgoOrds` / OCO protective stops intact on existing positions.
3. **Guarantee 3 — External Zero-Cost Heartbeat Alert (`Healthchecks.io` + Exchange App):**
   * The Proxmox container pings a free external `Healthchecks.io` URL every 60 seconds. If your home power or internet drops for $> 2\text{ minutes}$, `Healthchecks.io` (running on external cloud infrastructure) sends an instant push notification via Telegram/Email to your phone so you can monitor the exchange-hosted stop from your mobile app on 5G.

---

# 2. The Scale-Invariant Economics: Solving Fee Drag & Quantization ($100 $\rightarrow$ $10,000)

## 2.1 The Fee-to-Stop Ratio Law ($C_{\text{rt}} / D_{\text{stop}}$)

Peer review highlighted a critical mathematical truth:
Suppose round-trip trading fees + spread/slippage are $C_{\text{rt}} = 0.20\%$ ($20\text{ bps}$), and your strategy uses a tight $D_{\text{stop}} = 1.00\%$ ($100\text{ bps}$) stop-loss distance.
Then on every single trade, friction consumes:

$$
\text{Friction in } R = \frac{C_{\text{rt}}}{D_{\text{stop}}} = \frac{0.20\%}{1.00\%} = 0.20R \quad (20\% \text{ of your risk budget!})
$$

If friction eats $0.20R$ per trade, even a strong strategy will struggle to compound.

### How We Drive Friction Below $0.06R$

We attack both the numerator ($C_{\text{rt}}$) and the denominator ($D_{\text{stop}}$):

1. **Numerator Attack (Phase 0 Fee Discovery & Maker Execution):**
   * In Phase 0, `phase0_feasibility.py` queries `GET /api/v3/account/commission` (Binance) and `GET /api/v5/account/trade-fee` (OKX) across all quote currencies (`USDT`, `USDC`, `FDUSD`).
   * If your UAE entity offers a **$0.00\%$ maker-fee pair** (such as `BTC/FDUSD` or `BTC/USDC` promotions), maker entry fee is $0\text{ bps}$.
   * Even on standard fee pairs (`OKX BTC-USDT` at $8\text{ bps}$ maker / $10\text{ bps}$ taker), entering via a `Post-Only` (`ordType: "post_only"` / `LIMIT_MAKER`) order at the best bid cuts entry fees down to $8\text{ bps}$ ($0.08\%$).
2. **Denominator Attack (Minimum Structural Stop Distance $D_{\text{stop}} \ge 2.20\%$):**
   * We operate on **`1h` and `4h` swing horizons** and enforce a hard floor on stop distance:
     
     $$
     D_{\text{stop}} = \frac{|P_e - P_s|}{P_e} \ge \max\left(2.20\%, \ 12.5 \times C_{\text{rt, maker}}\right)
     $$
   * With $D_{\text{stop}} = 2.50\%$ ($250\text{ bps}$) and maker round-trip fee of $16\text{ bps}$ (or $0\text{--}10\text{ bps}$ on promotional pairs), fee drag drops to:
     
     $$
     \text{Friction in } R = \frac{16\text{ bps}}{250\text{ bps}} = 0.064R \quad (\text{or } \le 0.04R \text{ on zero-maker pairs})
     $$

## 2.2 How the Exact Same Math Scales from $\text{USD } 100$ to $\text{USD } 10{,}000$

Let:
- $E_t$: Active trading equity in quote currency.
- $r$: Risk fraction per trade = **$0.75\%$ of equity** ($r = 0.0075$) when $D_{\text{stop}} \ge 2.20\%$, or **$1.00\%$** ($r = 0.0100$) when $D_{\text{stop}} \ge 2.50\%$ so that the resulting position notional always clears exchange `minNotional` ($\text{USD } 5\text{--}10$) cleanly at $E_0 = \text{USD } 100$.

Let's verify the exact numbers across your capital roadmap for a trade with $D_{\text{stop}} = 2.50\%$, maker entry fee $f_e = 0.08\%$, taker stop fee $f_x = 0.10\%$, and stop slippage allowance $s_x = 0.05\%$ (total per-unit loss fraction $l_{\text{unit}} = 2.50\% + 0.08\% + 0.10\% + 0.05\% = 2.73\%$):

| Metric | Stage 1: Incubation ($\text{USD } 100$) | Stage 2A: Scale-Up ($\text{USD } 5{,}000$) | Stage 2B: Full Target ($\text{USD } 10{,}000$) |
|---|---|---|---|
| **Risk Budget ($1R = 1.00\% \text{ of } E_t$)** | $\text{USD } 1.00$ ($1.00\%$) | $\text{USD } 50.00$ ($1.00\%$) | $\text{USD } 100.00$ ($1.00\%$) |
| **Position Notional ($N = \frac{E_t \cdot r}{l_{\text{unit}}}$)** | $\text{USD } 36.63$ ($36.63\%$ of $E_t$) | $\text{USD } 1{,}831.50$ ($36.63\%$ of $E_t$) | $\text{USD } 3{,}663.00$ ($36.63\%$ of $E_t$) |
| **Clears Exchange `minNotional` ($\sim\text{USD } 5\text{--}10$)?** | **Yes** ($3.6\times\text{--}7.3\times$ above min) | **Yes** ($183\times$ above min) | **Yes** ($366\times$ above min) |
| **Lot-Size Quantization Error (`0.00001 BTC` step)** | $\sim 1.0\%$ of position size | $\sim 0.02\%$ of position size | $\sim 0.01\%$ of position size |
| **Order Book Impact on `BTC/USDT` or `ETH/USDT`** | $0.00\text{ bps}$ (Top level) | $0.00\text{ bps}$ (Top level holds $>\text{USD } 50\text{k}$) | $< 0.20\text{ bps}$ (Top 2 levels) |

Notice why setting $r = 1.00\%$ with $D_{\text{stop}} \ge 2.20\%\text{--}3.00\%$ is the sweet spot:
1. At $\text{USD } 100$, the position notional is $\text{USD } 30\text{--}\text{USD } 42$, which is well above every spot exchange's $\text{USD } 5\text{--}10$ minimum order size, while keeping maximum single-trade loss strictly capped at $1.00\%$ of equity.
2. Because position notional is $\sim 35\%\text{--}40\%$ of equity per trade, you don't even need $100\%$ of your cash in a single trade, or you can run up to $2$ independent positions (`BTC` and `ETH` at $0.75\%$ risk each) while keeping $> 20\%$ cash buffer.

---

# 3. The "Zero-FOMO Sniper Patience" Engine & Sequential Hypotheses

## 3.1 The 3-Gate Abstention Filter (Why the Bot Waits Days for A+ Setups)

The bot evaluates on `1h` (and `4h`) bar closes. Before any entry hypothesis is checked, all three abstention gates must pass:

1. **Gate 1 — Kaufman Efficiency Ratio ($\text{ER}_{t, 24}$) Anti-Chop Filter:**
   
   $$
   \text{ER}_{t, 24} = \frac{|P_t - P_{t-24}|}{\sum_{i=0}^{23} |P_{t-i} - P_{t-i-1}| + \epsilon} \in [0, 1]
   $$
   * If $\text{ER}_{t, 24} < \theta_{\text{er}}$ (pre-registered range $\theta_{\text{er}} \in [0.25, 0.35]$ for trend breakouts), the market is in noisy sideways chop. **Action:** `ABSTAIN_WAIT_IN_CASH`.
2. **Gate 2 — Volatility Compression-to-Expansion Filter (Minimum ATR Gate):**
   * Compute $\text{ATR\_pct}_t = \frac{\text{ATR}_{14, t}}{P_t}$.
   * If $\text{ATR\_pct}_t < 0.90\%$ on `1h` bars (or if expected $2.5 \times \text{ATR}_{14}$ target move is $< 3.0 \times C_{\text{rt}}$), the market lacks sufficient range to overcome trading costs cleanly. **Action:** `ABSTAIN_WAIT_IN_CASH`.
3. **Gate 3 — Macro Calendar & Data Integrity Blackout:**
   * Block entries within $\pm 30\text{ minutes}$ of scheduled FOMC, US CPI, and NFP releases (`configs/calendar_blackouts.json`), whenever bid-ask spread $> 2.5 \times \text{MedianSpread}_{24\text{h}}$, or if any WebSocket sequence gap is unresolved.

When any gate abstains, the bot stays flat in `Auto-Earn` / `Simple Earn Flexible`—even if that takes **5, 10, or 14 days**.

## 3.2 Entry Hypotheses (Tested One by One, Never Bundled Blindly)

Per peer review, we test **$H_1$ alone first** across $4+$ years of free historical data (`2021-01-01` to `2026-11-01` from `data.binance.vision` and OKX archives). $H_2$ and $H_3$ are tested only after $H_1$ is evaluated, and only if they meet the $\ge 100$-trade sample requirement.

| Priority | Hypothesis | Mathematical Specification | Sample Target | Kill Rule |
|---|---|---|---|---|
| **Priority 1 ($H_1$)** | **Regime-Gated Volatility Breakout (Primary Live Candidate)** | On `1h` or `4h` `BTC` and `ETH` bars: 1) $\text{ER}_{t,24} \ge \theta_{\text{er}}$, 2) Close breaks above N-bar Donchian/Keltner upper band ($P_t > \max_{i \in [1..N]} P_{\text{high}, t-i}$ with $N \in \{20, 36, 48\}$), 3) Volume confirmation $V_t > 1.25 \times \text{SMA}(V, 20)$, and 4) Taker Buy Ratio $\frac{V_{\text{taker\_buy}, t}}{V_t} > 0.53$. *(Note: Free Binance klines include `taker_buy_base_asset_volume` in column 9 going back to 2017!)* | $150\text{--}350$ trades across `BTC` + `ETH` | Killed if full walk-forward $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) \le 0$, or $< 100$ trades, or fails to beat random entries with identical trade count. |
| **Priority 2 ($H_2$)** | **Cross-Venue Coinbase Institutional Spot Premium (Ablation on $H_1$)** | Compute 4h z-score of Coinbase `BTC-USD` vs. Execution Venue `BTC-USDT` basis ($Z_{\text{CPI}, t}$). Test whether requiring $Z_{\text{CPI}, t} > 0.0$ (US spot trading at a premium to offshore stablecoin spot) on $H_1$ signals increases out-of-sample net expectancy and Deflated Sharpe. | $\ge 100$ trades | If it does not significantly improve out-of-sample Sortino/Calmar over $H_1$ alone, **drop the Coinbase feed completely**. |
| **Priority 3 ($H_3$)** | **Deleveraging Flush Mean-Reversion (Secondary Uncorrelated Strategy)** | Long entry on `1h` bars after a derivatives flush: 1) 24h return $r_{24\text{h}} \le -4.5\%$, 2) Perpetual Funding Rate in bottom 15th percentile over 30 days ($Z_{F, 30\text{d}} < -1.2$), and 3) Bullish reversal bar ($P_{\text{close}} > P_{\text{open}}$ with lower wick $> 40\%$ of bar range and volume $> 2.0 \times \text{SMA}(V, 20)$). *(Thresholds calibrated to ensure $\ge 100$ historical events across `BTC` + `ETH`).* | $100\text{--}180$ trades across `BTC` + `ETH` | Killed if $< 100$ trades or walk-forward $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) \le 0$. |

## 3.3 Exit Ablation Hypotheses ($E_0$ vs. $E_1$ vs. $E_2$)

To maximize our win/loss asymmetry without falling for the "passive scratch hole," we test three executable exit policies in Phase 3:

1. **$E_0$ (Baseline Chandelier ATR Trail):**
   * Initial atomic hard stop at $P_s = P_e - k_{\text{stop}} \cdot \text{ATR}_{14}$ ($k_{\text{stop}} \in \{2.0, 2.5\}$, minimum $2.20\%$ distance).
   * Trailing stop ratchets upward on every bar close to $\max_{s \in [t_0, t]}(P_{\text{high}, s}) - k_{\text{trail}} \cdot \text{ATR}_{14}$ ($k_{\text{trail}} \in \{2.5, 3.0\}$).
2. **$E_1$ (Ablation: Active-Touch Stagnation Time Stop):**
   * If after $M$ bars ($M \in \{3, 6\}$ hours) the trade's maximum favorable excursion (MFE) has not reached $+0.40R$ and current unrealized PnL is between $-0.35R$ and $+0.15R$, the breakout has stalled.
   * Exit immediately at the current market touch (15-second `Post-Only` at best ask, followed by collared marketable limit at best bid), cutting the stagnant trade at $\approx -0.15R$ instead of waiting for a $-1.00R$ stop.
   * **Adoption Rule:** Enabled only if $E_0 + E_1$ beats $E_0$ alone on out-of-sample walk-forward net expectancy and Sortino ratio.
3. **$E_2$ (Ablation: Delayed Breakeven Ratchet at $+1.25R$):**
   * Once unrealized profit reaches $+1.25R$ (giving the trade ample room to breathe first), amend the exchange-hosted stop-loss to `Entry Price + All-In Taker Fee + 10 bps Buffer`.
   * Modeled in backtests with full taker fees and a $10\text{ bps}$ adverse gap/slippage penalty.
   * **Adoption Rule:** Enabled only if $E_0 + E_2$ improves out-of-sample Calmar/Sortino over $E_0$.

---

# 4. Calibrated Statistical Gates (Solving the Small-Sample Holdout Paradox)

Peer review pointed out a crucial statistical calculation:
If a strategy generates ~80 trades per year (~1.5 trades/week across `BTC` and `ETH`), a 6-month holdout contains $N \approx 40$ trades. With per-trade return standard deviation $\sigma \approx 1.0R$, the standard error on $N = 40$ is:

$$
\text{SE}_{40} = \frac{1.0R}{\sqrt{40}} \approx 0.158R \implies 1.645 \times \text{SE}_{40} \approx 0.26R
$$

Requiring $\hat{\mu} - 1.645 \times \text{SE} > 0$ on a 40-trade holdout would reject any true edge below $+0.26R$/trade! Meanwhile, across a **4.5-year walk-forward test ($N \approx 300$ trades)**, the standard error is $\frac{1.0R}{\sqrt{300}} \approx 0.058R$ ($1.645 \times \text{SE}_{300} \approx 0.095R$), which has the statistical power to detect realistic $+0.12R\text{ to }+0.25R$ edges.

Therefore, we use a **Mathematically Calibrated Two-Stage Research Gate**:

| Validation Stage | Dataset Window | Exact Mathematical Requirement to Pass |
|---|---|---|
| **Stage A: Primary Statistical Proof (Purged Walk-Forward)** | $4.5\text{ Years}$ (`2021-01-01` to `2026-05-01`, rolling 12m train / 3m out-of-sample test with 48h purge/embargo) | 1. **Sample Size:** $N_{\text{oos}} \ge 120$ independent out-of-sample trades.<br>2. **Lower Confidence Bound:** 10,000-sample Politis–Romano block-bootstrap 95% lower bound $\text{LCB}_{95\%}(\text{EV}_{\text{net}}) > +0.02R$ after conservative fees & slippage.<br>3. **Deflated Sharpe Ratio:** $\text{DSR} > 0.95$ across all logged trials.<br>4. **Regime & Perturbation:** Positive net return in Bull, Bear, and Sideways years; survives $\pm 20\%$ parameter shifts. |
| **Stage B: Untouched Holdout Sanity Check** | Final $6\text{ Months}$ (`2026-05-01` to `2026-11-01`, evaluated **once**) | 1. **Positive Net Return:** $\text{EV}_{\text{net, holdout}} > 0$ after all costs.<br>2. **Consistency Check:** Holdout mean trade return falls inside the $95\%$ bootstrap confidence interval of Stage A.<br>3. **Drawdown Compliance:** Holdout max drawdown $\le 8.0\%$ at $1\%$ risk per trade. |

---

# 5. Clean Rust + PyO3 Architecture (Resolving the `#![forbid(unsafe_code)]` Crate Boundary)

To guarantee that `alpha-core` is 100% memory-safe (`#![forbid(unsafe_code)]`) while still compiling into a native Python module via `PyO3` (whose procedural macros generate `unsafe` FFI glue), we separate `alpha-core` from `alpha-py`:

```text
project-alpha-v3/
├── Cargo.toml                        # Workspace: alpha-types, alpha-core, alpha-py, alpha-venues, alpha-node
├── pyproject.toml                    # Builds `alpha-py` via maturin; installs polars, duckdb, scipy
├── .env.example                      # Names only (OKX_API_KEY, BINANCE_API_KEY, HEALTHCHECKS_URL, TG_TOKEN)
├── PLAN_V3.md                        # This master specification
├── docs/
│   ├── decisions/                    # 0001-venue-fees.md, 0002-proxmox-atomic-safety.md
│   ├── gates/                        # PHASE_00_FEASIBILITY.md .. PHASE_06_SCALE_UP.md
│   └── research/                     # Pre-registered H1..H3 & E0..E2 ablation reports
├── configs/
│   ├── paper.toml                    # Default mode (allow_live_orders = false)
│   ├── live_100.toml                 # Stage 1 ($100 incubation, 1.0% risk per trade)
│   ├── live_5000.toml                # Stage 2 ($5,000–$10,000 scale-up, 0.75%–1.0% risk)
│   └── calendar_blackouts.json       # FOMC, CPI, NFP UTC blackout windows
├── migrations/
│   └── 0001_state.sql                # SQLite WAL schema (orders, fills, positions, halt_state, equity_hwm)
├── rust/
│   └── crates/
│       ├── alpha-types/              # #![forbid(unsafe_code)] Decimal types, Bar, BookTop, OrderIntent
│       ├── alpha-core/               # #![forbid(unsafe_code)] Pure Rust: ER_t Chop Gate, H1-H3, E0-E2, Sizing, Risk Gate
│       ├── alpha-py/                 # Thin PyO3 wrapper around `alpha-core` for Python backtests
│       ├── alpha-venues/             # #![forbid(unsafe_code)] OKX v5 (port 443, attachAlgoOrds, cancel-all-after) & Binance Spot (OTOCO)
│       └── alpha-node/               # #![forbid(unsafe_code)] Single binary with subcommands: `record`, `trade`, `flatten`, `status`
├── python/
│   ├── scripts/
│   │   └── phase0_feasibility.py     # Queries live UAE account fees, zero-fee pairs, minNotional, Auto-Earn
│   └── research/
│       ├── download_history.py       # Downloads 2021-2026 Binance/OKX/Coinbase candles, funding, OI
│       ├── run_walkforward.py        # Executes `alpha_py` across Purged Walk-Forward + Holdout
│       └── validate_gates.py         # Computes DSR, Block-Bootstrap LCB, Perturbation & Leakage checks
└── tests/
    ├── fixtures/                     # Real JSON responses captured in Phase 0
    ├── proptest_money/               # 10,000-case property tests for scale-invariant sizing & risk
    ├── leakage/                      # Timestamp truncation & shuffled-label CI tests
    └── fault_injection/              # Atomic OTOCO/attachAlgoOrds verification, disconnect & restart drills
```

### Critical 2026 Venue Implementation Details (`alpha-venues`)

1. **OKX v5 WebSocket & REST Specifics (October/November 2026 Standard):**
   * **Port `443` Mandatory:** OKX discontinued legacy WebSocket port `8443` on `2026-10-31`. Use `wss://ws.okx.com/ws/v5/public`, `wss://ws.okx.com/ws/v5/private`, and `wss://ws.okx.com/ws/v5/business`.
   * **Order Book Sequence Check:** Validate `prevSeqId == last_seq_id` on every incremental `books` update (`prevSeqId == -1` indicates a fresh snapshot or maintenance reset). On any sequence gap, mark the book `INVALID`, halt entries, and resubscribe.
   * **Atomic Stop Attachment:** On `POST /api/v5/trade/order` (`tdMode: "cash"`), always populate the `attachAlgoOrds` array (do not use deprecated inline `slTriggerPx` fields which return `sCode=54070`).
   * **Dead-Man Switch:** Call `POST /api/v5/trade/cancel-all-after` with `{"timeOut": "60"}` every $20\text{ seconds}$ while an entry order is working.
2. **Binance Spot API Specifics:**
   * **Commission Check:** Query `GET /api/v3/account/commission?symbol=BTCFDUSD` (and `BTCUSDC`, `BTCUSDT`, `ETHUSDT`) to get exact `standardCommission.maker` and `taker` rates.
   * **Atomic Entry + Stop (`OTOCO`):** Submit entries via `POST /api/v3/orderList/otoco` with `workingType=LIMIT_MAKER` (or `LIMIT`) and `pendingBelowType=STOP_LOSS_LIMIT` so the protective stop is atomically bound to the entry on Binance's matching engine.

### 5.1 OKX Multi-Sub-Account Production Architecture & API Key Management

To guarantee strict risk isolation across strategies—preventing margin cross-contamination, conflicting position directions, and order sequence clashes—Project Alpha structures production execution across **three isolated OKX sub-accounts**:

| Sub-Account Name | Strategy ID | Instrument Universe | Timeframe | Execution Model & Venue |
|---|---|---|---|---|
| **`alphabreakout`** | `VOLATILITY_BREAKOUT` | Spot `BTC/USDT`, `ETH/USDT` | 4h bars | Post-Only Limit Maker + `attachAlgoOrds` atomic stops |
| **`alphacarry`** | `FUNDING_CARRY` | Spot + Perp (`BTC-USDT-SWAP`, `ETH-USDT-SWAP`) | 8h funding cycles | Delta-Neutral Basis Arbitrage + Fee-Hurdle Lock + ADL Watchdog |
| **`alphamacro`** | `MACRO_TREND` | Tokenized Macro (`XSPY-USDT`, `XQQQ-USDT`, `PAXG-USDT`, `BTC-USDT`) | 1D bars | Real-Yield Momentum + ATR Chandelier Trailing Stop |

#### The OKX $100 Minimum Balance Rule for Trade API Keys
* **Platform Security Enforcement:** OKX strictly requires a sub-account to maintain a **balance of $\ge \$100$** before granting API trading permissions (`Trade`).
* When sub-accounts are first created with $\$0$ balance, the API Key creation interface will grey out or reject the "Trade" permission checkbox.
* Therefore, API key generation and live key swapping must strictly follow the operational runbook below.

#### Step-by-Step Deposit & API Key Swap Procedure (Operational Runbook)

1. **Step 1: Deposit / Transfer Capital to Sub-Accounts:**
   - In OKX Dashboard -> navigate to `Assets` -> `Sub-Account Transfer`.
   - Transfer $\ge \$100$ (or target initial capital) from the Main Account (Funding or Unified Trading balance) into each of the 3 sub-accounts:
     - Transfer $\$100+$ to `alphabreakout`
     - Transfer $\$100+$ to `alphacarry`
     - Transfer $\$100+$ to `alphamacro`
2. **Step 2: Generate Dedicated Sub-Account API Credentials:**
   - Log into or switch account context to `alphabreakout` (or use OKX Sub-Account Management API generator).
   - Navigate to `User Profile` -> `API Keys` -> `Create V5 API Key`.
   - Set **Permissions:** Check `Read` and `Trade`. **Strictly leave `Withdraw` UNCHECKED.**
   - Set **Passphrase:** Enter a dedicated strong passphrase for the sub-account.
   - Set **IP Allowlist:** Bind the key to the Proxmox VE Debian 12 LXC static IP / outbound gateway (e.g., `192.168.1.230` or your home router's static public IP).
   - Securely record `API Key`, `Secret Key`, and `Passphrase`.
   - Repeat for `alphacarry` and `alphamacro`.
3. **Step 3: Update Environment Files on Workstation and Proxmox:**
   - Update `.env` locally and `/opt/alpha/.env` on the Proxmox LXC node (`chmod 600 /opt/alpha/.env`):
     ```bash
     # Sub-Account 1: Breakout
     OKX_API_KEY_BREAKOUT="<alphabreakout_api_key>"
     OKX_API_SECRET_BREAKOUT="<alphabreakout_secret_key>"
     OKX_PASSPHRASE_BREAKOUT="<alphabreakout_passphrase>"

     # Sub-Account 2: Funding Carry
     OKX_API_KEY_CARRY="<alphacarry_api_key>"
     OKX_API_SECRET_CARRY="<alphacarry_secret_key>"
     OKX_PASSPHRASE_CARRY="<alphacarry_passphrase>"

     # Sub-Account 3: Macro Trend
     OKX_API_KEY_MACRO="<alphamacro_api_key>"
     OKX_API_SECRET_MACRO="<alphamacro_secret_key>"
     OKX_PASSPHRASE_MACRO="<alphamacro_passphrase>"
     ```
4. **Step 4: Pre-Flight Verification & Live Cutover:**
   - Run `alpha-node status /opt/alpha/configs/paper.toml` to verify database connectivity, sub-account balance synchronization, and zero circuit-breaker halts.
   - When transitioning from Phase 5 (Paper Burn-In) to Phase 6 (Micro-Live Incubation), set `allow_live_orders = true` in `configs/live_100.toml`, verify systemd service unit points to the live config, and reload `systemctl daemon-reload && systemctl restart alpha-node.service`.

---

# 6. The 6-Month Incubation & Scale-Up Roadmap ($\text{USD } 100 \rightarrow \text{USD } 5{,}000\text{--}10{,}000$)

To hit your goal of proving the bot over 6 months on $\text{USD } 100$ and then scaling to $\text{USD } 5{,}000\text{--}10{,}000$, we structure the timeline into **Build & Validate (Weeks 1–6)** followed by the **6-Month Live Percentage Proof**:

| Phase | Timeline | Objective & Deliverable | Exit Gate |
|---|---|---|---|
| **Phase 0: Live Account & Fee Discovery** | **Days 1–2** | Run `python/scripts/phase0_feasibility.py` on your Proxmox box or workstation using read-only keys for OKX and Binance. | `docs/gates/PHASE_00_FEASIBILITY.md` records your exact maker/taker bps, any $0\%$ maker pairs, `minNotional`, and Auto-Earn APY. |
| **Phase 1: Historical Data & `alpha-core` Engine** | **Weeks 1–2** | Build `alpha-types`, `alpha-core`, and `alpha-py`. Download 2021–2026 `1h`/`4h` candles (with taker buy volume), funding, and OI into Parquet. Start the 24/7 Proxmox live order-book recorder in parallel. | `cargo test --workspace` passes 10,000 `proptest` cases for scale-invariant sizing ($100 to $50,000), rounding, and zero look-ahead leakage. |
| **Phase 2: Hypothesis Testing ($H_1 \rightarrow H_2 \rightarrow H_3$) & Exit Ablation ($E_0..E_2$)** | **Weeks 2–4** | Test $H_1$ alone first. Run Purged Walk-Forward (2021–May 2026) and evaluate the winning configuration once on the untouched 6-month holdout (May–Nov 2026). Test $H_2$, $H_3$, $E_1$, $E_2$ strictly as ablations. | Passes Stage A ($\ge 120$ trades, $\text{LCB}_{95\%} > +0.02R$, $\text{DSR} > 0.95$) and Stage B (positive holdout inside Stage A CI). **If nothing passes, stop before writing execution code.** |
| **Phase 3: Execution FSM, Atomic Stops & Demo Drills** | **Weeks 4–6** | Implement `alpha-venues` and `alpha-node` with atomic `attachAlgoOrds` / `OTOCO`, `cancel-all-after` dead-man switch, Auto-Earn idle cash yield, and Telegram/Healthchecks alerts. | Human audit of `sizing.rs`, `risk.rs`, and `execution.rs` completed. Passes pulled-ethernet-cable drill on OKX Demo / Binance Testnet proving exchange-side stop survives server death. |
| **Phase 4: Paper Burn-In & 6-Month $\text{USD } 100$ Incubation** | **Months 2–7** | 4 weeks of live paper verification reconciling maker fill rates against recorded order books, followed by **Micro-Live deployment on $\text{USD } 100$** tracking net $\%$ return, $R$-expectancy, slippage in $\text{bps}$, and max drawdown $\%$. | **The $\text{USD } 5{,}000\text{--}10{,}000$ Scale-Up Gate:**<br>1. Net positive $\%$ return after all fees over the incubation window.<br>2. Realized slippage within $\pm 3\text{ bps}$ of backtest assumptions.<br>3. Max drawdown $< 6.0\%$.<br>4. Zero unhandled state mismatches or missed stops. |
| **Phase 5: Two-Tranche Capital Scale-Up** | **Month 7+** | Scale capital in two controlled tranches without changing a single line of strategy code:<br>• **Tranche 1:** Deposit $\text{USD } 2{,}500\text{--}5{,}000$, run for 4 weeks.<br>• **Tranche 2:** Scale to $\text{USD } 10{,}000$ once Tranche 1 fill rates and slippage match the $\text{USD } 100$ baseline. | Continuous autonomous operation with exchange-side atomic stops and $-6.0\%$ high-water-mark drawdown circuit breaker. |

---

# 7. Strict Guardrails for the Coding Agent

1. **Enforce Workspace Lints in `Cargo.toml`:**
   Every pure-Rust crate (`alpha-types`, `alpha-core`, `alpha-venues`, `alpha-node`) must declare:
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
   *(Only `alpha-py` omits `#![forbid(unsafe_code)]` to allow `PyO3` macro expansion, and only `alpha-core::indicators` allows `clippy::float_arithmetic` for statistical $Z$-score/ATR calculations).*
2. **Scale-Invariance Property Test (`tests/proptest_money/scale_invariance.rs`):**
   Write an automated property test proving that for any valid market state, calling `calculate_position_size()` with $E = \text{USD } 100$, $E = \text{USD } 5{,}000$, and $E = \text{USD } 10{,}000$ produces identical percentage risk ($\frac{\text{Risk}_{\text{USD}}}{E} \le r_{\max}$) and identical percentage notional allocation within lot-step quantization bounds.
3. **Human Audit Gate on Money Files:**
   `sizing.rs`, `risk.rs`, and `execution.rs` must remain under $250\text{ lines}$ each, include exhaustive unit and property tests, and be reviewed line-by-line by the operator before `LIVE_APPROVED` is signed.
```

### Summary of What Was Perfected in `PLAN_V3.md` (v3.2 Final Edition)

1. **Scale-Invariant Percentage & $R$-Multiple Architecture ($\text{USD } 100 \rightarrow \text{USD } 5{,}000\text{--}10{,}000$):** Section 2 proves the exact math across $\text{USD } 100$, $\text{USD } 5{,}000$, and $\text{USD } 10{,}000$, showing how lot quantization noise drops from $\sim 1.0\%$ at $\text{USD } 100$ to $0.01\%$ at $\text{USD } 10{,}000$ while `BTC` and `ETH` order-book impact stays under $0.2\text{ bps}$.
2. **Definitive Hosting Solution (Section 1):** Documents why GCP Free Tier (US geo-blocked by Binance/OKX), Cloudflare/Render/Fly (no static IP for API key allowlisting + dropped WebSockets), and Oracle Cloud (signup blocks) fail, and pairs your **Ajman Proxmox Server** with **Atomic Exchange-Side Stop Attachment (`attachAlgoOrds` on OKX / `OTOCO` on Binance Spot)** and **`cancel-all-after` Dead-Man Timers** so a home power or internet cut can never leave a position unprotected.
3. **Solved the Fee-to-Stop Ratio & Holdout Paradox (Sections 2, 3, 4):** Enforces a minimum structural stop distance ($\ge 2.20\%$ on `1h`/`4h` swing bars) so round-trip maker fees consume $\le 0.064R$, tests **$H_1$ alone first** (using free Binance `taker_buy_base_asset_volume` history going back to 2017), and separates `alpha-core` (`#![forbid(unsafe_code)]`) from `alpha-py` (`PyO3`).

Would you like me to generate the standalone **`python/scripts/phase0_feasibility.py`** script next so you can run Phase 0 on your Proxmox server or PC and immediately inspect your live OKX/Binance fee tiers, zero-fee pairs, and `minNotional` filters?