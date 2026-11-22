# Systematic Strategy Improvement & Optimization Playbook (`IMPROVE_STRAT.md`)

## 1. Philosophy: Hypothesis-Driven Refinement vs. Curve-Fitting

Improving a trading strategy is the most dangerous phase in quantitative research. Every parameter tweak, filter addition, and exit adjustment consumes **degrees of freedom** and inflates the probability of **backtest overfitting** (fitting historical noise rather than persistent market structure).

### The 5 Non-Negotiable Rules of Improvement
1. **Economic Hypothesis First:** Never change a rule just because an optimizer says it raised Sharpe. State *why* the change should work structurally (e.g., *"Filtering out low-liquidity overnight hours reduces adverse spread slippage"* or *"Scaling stops by ATR adapts to volatility regimes"*).
2. **One Change at a Time (Ablation Testing):** Modify one component per experiment and measure marginal impact on the **Validation Set**, keeping the **Out-of-Sample (OOS) Holdout** strictly untouched.
3. **Log Every Trial:** Increment a persistent **Trial Counter ($M$)** for every variation tested. Every extra trial raises the bar required for the Deflated Sharpe Ratio ($\text{DSR}$).
4. **Occam's Razor (Parsimony Penalty):** Adding a new rule or parameter must produce a **material, broad-based improvement** across metrics—not a tiny bump that makes the model fragile.
5. **Improve the Worst Part First:** Focus first on eliminating structural drag (excess turnover, bad regimes, execution leakage, poor position sizing) before adding complex secondary indicators.

---

## 2. Diagnostic Attribution: Pinpointing Where Alpha Leaks

Before changing anything, dissect the baseline strategy to locate the exact bottleneck.

### 2.1 Multi-Dimensional PnL Slicing
Break down Net EV, Sharpe, Win Rate, and Basis PnL across slices:
- **Directional Split:** Long trades vs. Short trades. (Do shorts bleed consistently in upward-drifting equity indices, or are longs failing in bear regimes?)
- **Temporal Seasonality:** Hour of day (open auction volatility vs. midday chop vs. close), day of week, month/quarter-end rebalancing, funding-settlement windows (e.g., 8-hour crypto funding intervals).
- **Volatility Regime Buckets:** Split history into Low ($\text{Vol} < P_{33}$), Medium ($P_{33} - P_{66}$), and High ($\text{Vol} > P_{66}$) realized volatility regimes.
- **Market Trend & Basis Regimes:** Trending vs. Mean-Reverting environments; Contango ($\text{Basis} > 0$) vs. Backwardation ($\text{Basis} < 0$).

### 2.2 MAE & MFE Trade Path Analysis
Plot **Maximum Adverse Excursion (MAE)** and **Maximum Favorable Excursion (MFE)** normalized by entry volatility (ATR or $\sigma$) for every trade:
- **Winning trades with tiny MAE vs. Losing trades with huge MAE:** Indicates a well-placed volatility stop-loss can cut left-tail losses without prematurely stopping out winners.
- **Losing trades that first showed high MFE:** Indicates weak exit timing—trades move into profit and then fully mean-revert. Consider trailing stops, partial profit scaling, or time-based exits.
- **Winning trades with immediate large MAE:** Indicates entries are consistently too early (catching falling knives); test confirmation triggers or passive limit entry ladders.

### 2.3 Signal Alpha Decay Curve
Measure the forward Information Coefficient ($\text{IC}_h$) and cumulative forward return at horizons $h \in \{1, 2, 5, 10, 20, 50, \dots\}$ bars after a signal fires:
$$\text{IC}_h = \text{Corr}\left( \text{Signal}_t, \; r_{t \to t+h} \right)$$
- **Fast decay (peaks at $h = 1$ to $3$ bars):** Holding longer only adds uncompensated market noise; shorten holding period or tighten time stops.
- **Slow build / persistent IC:** Exiting too early leaves EV on the table; widen holding horizon and reduce turnover.
- **Post-peak reversal (negative marginal IC after $h^*$):** Implement a mandatory time-exit at $h^*$ bars.

---

## 3. Parameter Engineering & Anti-Overfitting Discipline

### 3.1 Degrees of Freedom & Parameter Normalization
- **Hard Parameter Cap:** Aim for **$2$ to $5$ core free parameters** per independent signal module. Strategies with $> 8$ tightly tuned parameters almost always collapse out-of-sample.
- **Replace Magic Numbers with Dimensionless Ratios:**
  - *Bad:* Fixed `$500` stop-loss, fixed `2%` profit target, fixed `$0.10` threshold.
  - *Good:* Stop-loss at $k \times \text{ATR}_n$, signal threshold at $z$-score $\ge 1.5\sigma$, basis trigger scaled by rolling basis standard deviation.
- **Use Logarithmic / Coarse Grids:** Test lookbacks like $\{10, 20, 40, 80\}$ instead of $\{17, 18, 19, 20, 21\}$.

### 3.2 Parameter Stability Surfaces (Plateaus vs. Spikes)
Generate 2D heatmaps of Sharpe Ratio and Net EV across pairs of parameters $(\theta_1, \theta_2)$:
- **Golden Rule:** Never pick the absolute highest-Sharpe grid cell if it sits on a narrow spike surrounded by poor performance.
- **Center-of-Mass Selection:** Select parameters located at the geometric center of a **broad, flat plateau** where shifting any parameter by $\pm 20\%$ to $\pm 30\%$ retains at least $75\%$ to $80\%$ of peak Sharpe and keeps EV strictly positive.
- **Ensemble Averaging Across Parameters:** Instead of picking one lookback (e.g., `EMA_20`), blend signals across a clean geometric range (e.g., equal-weighted signal across $\{10, 20, 40\}$). This lowers parameter variance and boosts OOS Sharpe.

### 3.3 Walk-Forward Optimization (WFO) & Purged Cross-Validation
If parameters must adapt over time, validate the optimization *process* itself via Walk-Forward Optimization:
1. Optimize parameters on window $[t_0, t_1]$ (IS).
2. Apply chosen parameters strictly forward on $[t_1, t_2]$ (OOS).
3. Roll forward (or expand anchored window) and stitch all OOS segments together.
4. Measure **Walk-Forward Efficiency (WFE):**
   $$\text{WFE} = \frac{\text{Annualized Return}_{\text{stitched OOS}}}{\text{Annualized Return}_{\text{IS}}}$$
   - **Target:** $\text{WFE} \ge 0.50$ to $0.60$, and chosen parameters should not violently whipsaw from one window to the next.

---

## 4. Signal & Alpha Improvements

### 4.1 Signal Processing & Outlier Hygiene
- **Continuous Signals Over Binary Triggers:** Convert raw binary rules (`0` or `1`) into continuous conviction scores $s_t \in [-1, +1]$ (e.g., via $\tanh(z_t / c)$ or rank percentiles) so stronger edges receive proportionally larger size.
- **Cross-Sectional Neutralization (Multi-Asset Strategies):** Demean signals cross-sectionally (or sector-neutralize / beta-neutralize) to remove unintended market directional drift and isolate pure relative-value alpha.
- **Winsorization:** Cap extreme feature outliers (e.g., at $\pm 3\sigma$ or $1\text{st}/99\text{th}$ percentiles) so single bad ticks or flash crashes do not distort rolling signals.

### 4.2 Regime Filtering & Meta-Labeling
Rather than adding arbitrary indicator filters, structure improvements cleanly:
- **Volatility & Liquidity Filters:** Pause trading only when economic logic justifies it (e.g., avoid mean-reversion entries during extreme volatility expansion regimes, or avoid trading when spread exceeds $k \times \text{EV}_{\text{bps}}$).
- **Meta-Labeling (Two-Stage Architecture):**
  - *Primary Model:* Predicts trade side (Long/Short) with high recall.
  - *Secondary (Meta) Model:* Predicts the probability $p \in [0, 1]$ that the primary trade will be profitable (using regime features like volatility, basis slope, volume surge, and drawdown state), and maps $p$ directly to position size rather than hard binary blocking.

### 4.3 Basis, Term-Structure & Microstructure Conditioning
- **Carry Alignment:** In futures/perpetuals, tilt directional sizing with favorable basis/funding carry or require higher directional conviction when trading *against* steep negative carry.
- **Cross-Asset Confirmation:** Validate single-asset breakout/reversion signals against sector ETFs, volatility indices (e.g., VIX term structure), or spot-vs-perp lead-lag dynamics.
- **Check Multicollinearity:** Before adding a second indicator/feature, compute its correlation with existing features. Adding two features with $\rho > 0.75$ adds overfitting risk with near-zero incremental information.

---

## 5. Execution, Turnover & Cost Optimization

Often the highest-ROI improvement in quant trading comes from **execution engineering**, not new alpha signals.

### 5.1 Turnover Damping (No-Trade Zones / Hysteresis)
If continuous signals rebalance slightly every bar, transaction costs destroy EV.
- **Rebalance Buffer (Deadband):** Only trade when target position $w_t^*$ differs from current position $w_{t-1}$ by more than a cost-aware threshold $\delta$:
  $$\text{Trade only if } |w_t^* - w_{t-1}| > \delta$$
- **Hysteresis Entry/Exit Thresholds:** Enter when signal $|z_t| > 2.0$, but hold until signal decays past $|z_t| < 0.5$ instead of flipping around a single line.
- **Cost-Aware Objective:** Optimize directly for **Net Sharpe** and **Net $\text{EV}_{\text{bps}}$**, or explicitly penalize turnover in the optimization objective:
  $$\max_{w} \; \mathbb{E}[r_{\text{net}}] - \frac{\lambda}{2}\sigma^2 - \kappa \cdot \text{Turnover}$$

### 5.2 Order Execution & Slippage Reduction
- **Passive (Maker) vs. Aggressive (Taker) Split:** For mean-reversion strategies (which provide liquidity), test passive limit orders placed at the touch or slightly inside spread to capture spread/maker rebates—**but explicitly model adverse selection** (limit orders get filled $100\%$ of the time when price blows through them, and $< 100\%$ of the time on winning bounces).
- **Execution Slicing:** For larger notional orders, split execution via TWAP, VWAP, or Percentage-of-Volume ($\text{POV} \le 5\%$) to control market impact, which scales roughly via the **Square-Root Law**:
  $$\text{Impact} \approx Y \cdot \sigma_{\text{daily}} \cdot \sqrt{\frac{Q}{V_{\text{daily}}}}$$
- **Avoid Auction / Illiquid Transition Windows:** Skip entries during the first/last minutes of a session (unless explicitly trading auction alpha) or around scheduled macro announcements if spreads widen uncontrollably.

---

## 6. Position Sizing, Risk Overlays & Portfolio Scaling

Upgrading risk management frequently elevates Sharpe, Calmar, and CAGR without altering the underlying entry signal.

### 6.1 Volatility Targeting (Dynamic Risk Normalization)
Scale position size inversely to short-term realized volatility (e.g., EWMA or Garman-Klass volatility $\hat{\sigma}_t$) so every trade/day carries uniform risk:
$$w_t = \text{clip}\left( \frac{\sigma_{\text{target}}}{\hat{\sigma}_t}, \; 0, \; L_{\max} \right) \times s_t$$
where $L_{\max}$ is a strict maximum leverage cap.
- *Benefit:* Automatically de-risks during market panics, stabilizes return variance, cuts Max Drawdown, and reduces left-tail kurtosis.

### 6.2 Fractional Kelly Sizing
Theoretical optimal geometric growth rate (Kelly fraction):
$$f^* = \frac{\mu - r_f}{\sigma^2} \quad \text{(continuous)} \qquad \text{or} \qquad f^* = \frac{p \cdot b - q}{b} \quad \text{(binary)}$$
- **Strict Quant Rule:** **Never trade Full Kelly ($1.0 f^*$).** Estimation error in $\mu$ and $\sigma$ turns Full Kelly into catastrophic drawdowns. Use **Quarter-Kelly ($0.25 f^*$) to Half-Kelly ($0.50 f^*$)**, which captures $75\%$ of optimal growth rate at a fraction of the variance.

### 6.3 Exit Architecture Refinement
Compare exit mechanisms systematically:
- **Signal-Driven Exit:** Exit when the alpha signal neutralizes or reverses (most natural; avoids arbitrary price levels).
- **Volatility-Scaled Stop-Loss / Trailing Stop:** Protects against tail-risk blowups without suffocating normal noise.
- **Time Stop:** Forces exit after $h_{\max}$ bars if the thesis fails to materialize (frees capital and eliminates dead-money drift).
- **Portfolio Risk Overlays:** Apply hard caps on single-asset concentration, net directional beta exposure, gross leverage, and sector/factor tilts.

---

## 7. Improvement Acceptance Gate (Keep vs. Revert Checklist)

Before accepting any modification into the master strategy version, it must pass **all** of the following checks on the Validation dataset:

1. **Clear OOS/Validation Uplift:** Net Sharpe increases meaningfully (e.g., $\Delta \text{SR} \ge +0.15$ to $+0.25$) OR Max Drawdown drops by $\ge 20\%$ with flat/higher Sharpe OR Net $\text{EV}_{\text{bps}}$ increases while turnover drops.
2. **Complexity Penalty Cleared:** Does the improvement survive after penalizing for extra parameters (higher DSR / lower AIC/BIC)? If a complex rule adds $+0.05$ Sharpe, **revert it**.
3. **Broad Consistency:** Improvement holds across at least $70\%$ of sub-periods (years/regimes) and across multiple assets in the universe—not solely from fixing one historical crash.
4. **Parameter Plateau Preserved:** The modified strategy still sits on a smooth parameter plateau with no cliff edges.
5. **Version Control:** Tag the new baseline (`v1.0` -> `v1.1`), record the exact hypothesis, metric deltas, and cumulative trial count $M$, then proceed to `STRESS_TEST_STRAT.md`.