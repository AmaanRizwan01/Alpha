# Project Alpha — Technical Specification & Production Plan

**Version:** 1.0  
**Prepared:** 2026-11-07  
**Status:** Research and implementation blueprint — not a guarantee of profitability  
**Initial capital:** USD 100  
**Operating model:** Fully autonomous execution after explicit deployment approval; conservative risk; retail exchange APIs  
**Strategy scope:** Directional microstructure strategy + arbitrage research/strategy engine  
**Primary intelligence feed:** Official X API, plus verified news, macro, derivatives and on-chain data

---

# 0. Executive summary

Project Alpha is a small-account, event-driven quantitative trading system. It continuously ingests exchange market data and external information, maintains a local view of market state, evaluates independent strategies, applies a separate risk gate, and submits/reconciles orders through exchange APIs.

The system is designed to run 24/7. It is **not** designed to promise profits, a 70% win rate, negligible losses, or microsecond end-to-end execution. No trading system can guarantee those outcomes. Retail internet/API infrastructure cannot reliably compete with colocated institutional HFT firms on end-to-end microsecond latency.

The engineering objective is:

1. Establish positive, statistically defensible net expectancy after fees, spread, slippage, funding, and failed/partial fills.
2. Keep drawdown and operational risk tightly bounded.
3. Use low-latency local processing where it has measurable value.
4. Detect high-impact public information, including posts from verified public figures, governments, regulators, central banks, companies and protocols.
5. Never allow an LLM, news event, model score or strategy to bypass the risk engine.
6. Start with paper trading; enable live trading only after objective acceptance gates pass.

**Important capital constraint:** USD 100 is enough to build and test the software, but may be too small for many live strategies once exchange minimum notional, fee schedules, minimum quantity/step size, spread and risk limits are considered. Cross-exchange arbitrage needs pre-funded balances on more than one venue and is initially paper-only. Leveraged perpetuals, margin borrowing, martingale and recovery sizing are out of scope for the initial live deployment.

## 0.1 Initial operating decisions

| Decision | Specification |
|---|---|
| Autonomy | Fully autonomous only after explicit operator approval of a signed production release |
| Markets | Crypto first; spot live initially |
| Directional engine | BTC/USDT (or the most liquid eligible spot equivalent) |
| Arbitrage engine | Cross-venue spot and spot/perpetual basis research; paper-only at first |
| Risk posture | Conservative, no leverage, no averaging down |
| Execution | Retail public REST/WebSocket APIs |
| Local processing | Microsecond-scale feature calculations are a target to measure, not an end-to-end execution promise |
| X monitoring | Official X API only; streaming where account plan permits, recent-search polling as a fallback |
| AI | Asynchronous classification and summarization; never on the critical order-submission path |
| Initial account risk | At most 0.5% of equity per trade as a *target*, subject to minimum order sizes and cost-aware sizing; no trade if constraints cannot be met |
| Daily stop | 1% of starting-of-day equity; configurable downward, never silently increased |
| Emergency stop | Immediate halt of new orders; cancel working orders; manage/flatten positions according to pre-approved safe policy |

---

# 1. Non-negotiable principles

1. **No guaranteed-profit claims.** All signals are hypotheses until validated on untouched data and then observed live.
2. **No trade is a valid action.** The bot must be comfortable remaining flat for hours or days.
3. **Risk engine is independent.** Strategies cannot alter risk limits, exchange permissions or kill-switch state.
4. **No leverage in the initial live phase.** Use spot only. Short strategies and derivatives arbitrage remain in simulation until separately approved.
5. **No untrusted X scraping.** Use the official X API or a licensed provider whose contract permits the intended use. Do not scrape logged-in pages, evade access controls, or depend on unofficial endpoints.
6. **No single-source headline trading.** A social post is an alert, not proof. Verify account identity, original post, context, corroborating sources and market response.
7. **No LLM in the hot path.** LLM calls are too variable in latency and reliability. Use deterministic rules and precomputed models for execution.
8. **Fail closed.** If market data is stale, sequence continuity is lost, clock drift is excessive, exchange state is uncertain, or risk state cannot be read, do not open new positions.
9. **Every decision is auditable.** Persist raw event references, feature values, model versions, risk decisions, order intents, exchange responses and fills.
10. **Exchange, legal and tax eligibility must be checked for the user's jurisdiction before live use.** Availability and terms can change.

---

# 2. What "microsecond trading" means for this project

The system can make local calculations in microseconds when implemented efficiently in Rust and when the data is already in memory. That does not mean a retail order is filled in microseconds.

Measure these separately:

- `t_exchange_event`: exchange event timestamp, if provided.
- `t_socket_receive`: local monotonic timestamp when bytes arrive.
- `t_decode_done`: parsed event ready.
- `t_book_update_done`: local book updated and validated.
- `t_feature_done`: features computed.
- `t_decision_done`: strategy decision made.
- `t_order_send`: request handed to network stack.
- `t_ack_receive`: exchange acknowledgement received.
- `t_fill_receive`: fill event received.
- `t_fill_exchange`: exchange fill timestamp, if provided.

Report p50, p90, p95, p99 and max for each stage. Keep exchange timestamps and local monotonic timestamps separate. Synchronize host time with chrony/NTP; use monotonic clocks for durations.

Target internal budgets (initial engineering targets, not exchange guarantees):

| Stage | Initial target |
|---|---:|
| Decode + validate one event | < 100 µs p99 on benchmark host |
| Update local book | < 100 µs p99 |
| Compute deterministic feature vector | < 100 µs p99 |
| Strategy + risk decision | < 250 µs p99 |
| Local event-to-order-intent | < 1 ms p99 |
| End-to-end exchange acknowledgement | Measure only; no promised target |
| Actual fill latency | Measure only; depends on venue, queue, liquidity and network |

Do not optimize prematurely. First prove a profitable signal at realistic latency. Only then benchmark Rust, CPU pinning, allocator behavior, kernel/network tuning or a closer VPS region.

---

# 3. Market and venue selection

## 3.1 Initial market

Start with one highly liquid crypto spot pair, normally BTC/USDT or an eligible local equivalent. Do not begin with a broad universe of illiquid altcoins.

Venue selection is a scored due-diligence decision, not a brand preference. Before selecting a venue, verify:

- The exchange accepts the user's country/residency and account type.
- Spot trading and API access are available to that account.
- The exact fee tier and maker/taker fees.
- Minimum notional, quantity step, price tick and order types for the chosen symbol.
- WebSocket market-data quality and documented snapshot/delta semantics.
- Private user-data stream for orders, fills and balances.
- API request limits and WebSocket limits.
- API key permissions, IP allowlisting and withdrawal-disable controls.
- Historical data availability and terms of use.
- Operational status, incident history and support.
- Whether the exchange's local legal status and terms permit the intended use.

Potential adapters to evaluate:

- **Binance Spot API:** broad spot market coverage, documented REST, WebSocket streams, exchange information and symbol filters. Check regional eligibility, fees and current API documentation.
- **Bybit V5:** spot/derivatives API and public order-book stream; documented order-book update frequencies and rate limits. Check account and regional eligibility.
- **Coinbase Advanced Trade:** WebSocket market data including Level 2 and authenticated user channel. Evaluate the relevant products and fees.

Do not connect all venues in v1. Select one venue for live spot execution after due diligence; keep other adapters for read-only comparison and later arbitrage simulation.

## 3.2 Venue adapter interface

Each venue adapter must implement:

```text
VenueAdapter
  connect_public_market_data()
  connect_private_user_stream()
  fetch_exchange_info()
  fetch_balances()
  fetch_open_orders()
  fetch_order(order_id)
  submit_order(order_intent)
  amend_order(order_id, amendment)
  cancel_order(order_id)
  cancel_all(symbol?)
  normalize_market_event(raw_event)
  normalize_order_event(raw_event)
  normalize_fill(raw_event)
  normalize_error(raw_error)
  get_fee_schedule()
  get_server_time()
```

Every adapter must expose capability flags. Never assume that all venues support the same order types, time-in-force, post-only behavior, client order IDs, amend semantics or cancel-on-disconnect features.

## 3.3 Market-data requirements

Subscribe to, where supported:

- Best bid/ask.
- Level 2 depth snapshot and incremental updates.
- Trades/aggregated trades with aggressor side where available.
- Candles for context only.
- Exchange status/instrument status.
- Symbol metadata and trading filters.

Rebuild the local book using the exchange's exact documented snapshot/delta procedure. Validate sequence IDs/checksums when supplied. If a gap, checksum mismatch, unexpected snapshot or reconnect occurs, mark the book invalid, stop new entries, fetch a fresh snapshot and replay deltas according to venue documentation.

---

# 4. Strategy architecture

Strategies are separate plugins. They cannot directly place orders. They emit `TradeProposal` objects into the risk engine.

```text
TradeProposal {
  proposal_id
  strategy_id
  strategy_version
  venue
  symbol
  side
  intended_entry_type
  reference_price
  stop_price
  target_price_or_exit_rule
  max_holding_time
  expected_gross_edge_bps
  expected_fee_bps
  expected_spread_cost_bps
  expected_slippage_bps
  expected_funding_bps
  confidence_or_probability
  feature_snapshot_id
  valid_until_monotonic_ns
  reason_codes[]
}
```

## 4.1 Strategy A — directional order-flow / microstructure

**Purpose:** Estimate short-horizon price direction and avoid entering when expected adverse selection or execution cost exceeds the predicted edge.

### Required features

Compute rolling windows at 100 ms, 250 ms, 1 s, 5 s, 30 s and 1 min, subject to data quality and event rate:

- Best bid, best ask, mid-price, spread.
- Bid/ask depth at top 1, 5, 10 and 20 levels.
- Order-flow imbalance (OFI).
- Order-book imbalance (OBI).
- Microprice displacement from mid.
- Aggressive buyer/seller executed volume.
- Trade count and average trade size.
- Short-term realized volatility.
- Price impact per unit of aggressive volume.
- Book replenishment and cancellation rates.
- Volatility regime and spread regime.
- Time since last book reset/reconnect.
- X/news event-risk flag.
- Cross-market reference return, if a reliable independent feed exists.

### Formula: mid-price

\[
m_t = \frac{a_t+b_t}{2}
\]

where \(a_t\) is best ask and \(b_t\) is best bid.

### Formula: spread in basis points

\[
spread_{bps} = 10{,}000 \times \frac{a_t-b_t}{m_t}
\]

### Formula: order-book imbalance

For selected depth levels \(L\):

\[
OBI_t = \frac{\sum_{i=1}^{L}q^{bid}_{i,t}-\sum_{i=1}^{L}q^{ask}_{i,t}}
{\sum_{i=1}^{L}q^{bid}_{i,t}+\sum_{i=1}^{L}q^{ask}_{i,t}+\epsilon}
\]

### Formula: microprice

For best bid/ask quantities \(q_b,q_a\):

\[
microprice_t = \frac{a_t q_b + b_t q_a}{q_b+q_a+\epsilon}
\]

### Formula: microprice displacement

\[
microprice\_edge_{bps}=10{,}000 \times \frac{microprice_t-m_t}{m_t}
\]

### Formula: OFI

Use a documented best-level OFI definition. For consecutive book states, one common form is:

\[
e_n =
\mathbf{1}_{P^B_n \ge P^B_{n-1}}q^B_n
-\mathbf{1}_{P^B_n \le P^B_{n-1}}q^B_{n-1}
-\mathbf{1}_{P^A_n \le P^A_{n-1}}q^A_n
+\mathbf{1}_{P^A_n \ge P^A_{n-1}}q^A_{n-1}
\]

\[
OFI_{t,W} = \sum_{n \in W}e_n
\]

Normalize by recent depth/volume and winsorize outliers. Validate the exact sign convention against the implementation and test fixtures.

### Formula: aggressive trade-flow imbalance

\[
TFI_{t,W} =
\frac{V^{buy}_{t,W}-V^{sell}_{t,W}}
{V^{buy}_{t,W}+V^{sell}_{t,W}+\epsilon}
\]

Aggressor-side classification must follow the venue's documented field. If aggressor side is absent, infer cautiously from quotes and label the feature as inferred.

### Formula: realized volatility

For log returns \(r_i=\ln(m_i/m_{i-1})\):

\[
RV_{t,W}=\sqrt{\sum_{i \in W}r_i^2}
\]

Annualization is not required for very short-horizon execution gating; use consistent units.

### Signal model

Start with a transparent baseline, not a neural network:

\[
\hat r_{t,h} = \beta_0 + \sum_j\beta_j z_{j,t}
\]

where \(z_j\) are standardized features and \(\hat r_{t,h}\) is the predicted return over horizon \(h\).

Compare:
- Logistic regression for direction probability.
- Ridge/Elastic Net regression for expected return.
- LightGBM for nonlinear feature interactions.
- A simple rule-based baseline.

Do not deploy a model just because its classification accuracy is high. Evaluate calibrated probability, net expected return and stability across time.

### Trade condition

For a long proposal:

\[
E_{net,bps} =
\hat r_{t,h,bps}
- fee_{entry,bps}
- fee_{exit,bps}
- spreadCost_{bps}
- slippage_{bps}
- safetyBuffer_{bps}
\]

Only propose a trade if:

- Data quality is valid.
- Spread and depth pass limits.
- Model is in a regime in which it has been validated.
- Expected net edge exceeds a pre-registered threshold.
- Cost-aware risk sizing yields a valid order.
- No news/event cooldown or kill-switch is active.

The directional strategy should initially be **spot long-or-flat**. Do not borrow or short in the first live version. Short-side and perpetual strategies can be simulated independently.

## 4.2 Strategy B — arbitrage engine

The arbitrage engine runs independently and initially has **no live execution permission**.

### B1. Cross-exchange spot arbitrage

For buying on venue A and selling on venue B:

\[
GrossEdge = Bid_B - Ask_A
\]

\[
NetEdge_{bps} =
10{,}000 \times \frac{Bid_B-Ask_A}{ReferencePrice}
- Fee_A - Fee_B - Slippage_A - Slippage_B
- InventoryCost - Transfer/SettlementRiskBuffer
\]

Use executable order-book depth, not last-traded prices. Compute a size-dependent VWAP for both legs. The smaller executable leg constrains the arbitrage quantity.

Requirements:
- Pre-funded inventory on both venues.
- Both balances and order books current.
- Both legs executable within limits.
- A hedge/unwind plan if one leg fills and the other fails.
- Inventory limits per venue and asset.
- No dependence on a blockchain transfer to complete an immediate arbitrage.
- Include withdrawal/deposit suspension and counterparty risk in venue risk.

With USD 100, capital fragmentation and minimum notionals can make this impractical. Keep it paper-only until capital, fees, venue access and execution tests justify otherwise.

### B2. Spot/perpetual basis and funding research

For spot price \(S\) and perpetual price \(F\):

\[
Basis_{bps}=10{,}000\times\frac{F-S}{S}
\]

Expected net carry over holding period \(T\):

\[
Carry_{net}=ExpectedFunding
+ExpectedBasisConvergence
-EntryFees-ExitFees
-Spread-Slippage-Borrow/CapitalCost
\]

Funding payments are variable, can reverse, and do not guarantee convergence. Include mark-price, index-price, liquidation, collateral, venue and basis-gap risk. This strategy remains simulation-only initially because derivatives introduce leverage/margin/liquidation risk and the account is small.

### B3. Triangular arbitrage

Only investigate after robust order-book simulation. For a three-leg path A→B→C→A, multiply executable conversion rates and subtract all fees, rounding, slippage and leg-failure risk. A displayed price loop is not an executable arbitrage. Simulate partial fills and the risk of being left with residual inventory.

---

# 5. X (Twitter) intelligence subsystem

## 5.1 Objective

Monitor public posts from selected accounts and topics, identify potentially market-moving statements, verify provenance and context, and publish a structured event to the risk/strategy layer within seconds where the API and network permit.

**Do not claim guaranteed delivery within seconds.** X API access tier, platform delivery, rate limits, network path, reconnects and provider outages affect latency. Measure observed post-created-to-received and received-to-classified latency.

## 5.2 Account watchlist categories

Create a configurable, reviewed watchlist. Candidate categories:

### Public figures and business leaders
- Donald Trump (official account identity must be resolved and verified).
- Elon Musk (official account identity must be resolved and verified).
- CEOs/founders of major crypto, technology and financial companies.
- Publicly identified executives whose statements have historically been relevant to tracked assets.

### Governments and public institutions
- The White House and relevant official executive-branch accounts.
- U.S. Treasury.
- Federal Reserve and relevant regional Fed accounts.
- U.S. SEC and CFTC.
- ECB and Bank of England.
- Official El Salvador presidency and finance/economic ministry accounts.
- Other finance ministries, central banks and regulators selected by the operator.

### Companies and protocols
- Major exchanges and their official status/announcement accounts.
- Bitcoin and Ethereum ecosystem organizations.
- Issuers of major stablecoins.
- ETF issuers and custodians.
- Major listed companies with material crypto/technology exposure.
- Protocol security/status accounts.

Do not assume a display name or blue verification badge proves authenticity. Resolve each account through the official API, store its stable numeric user ID, username, profile URL, verification/affiliation evidence, verification timestamp and source. Revalidate on username changes. Require a human to approve additions to the high-impact watchlist.

## 5.3 X API integration

Preferred path:
- Official X API v2.
- Filtered Stream endpoint when the account plan and project permissions permit.
- Manage stream rules through the documented rules endpoint.
- Use `from:username` or account-specific query operators as supported by current API docs.
- Request required post fields such as ID, text, author ID, created time, referenced posts, entities, attachments and public metrics where permitted.
- Store the raw API response and request/receive timestamps.
- Deduplicate by post ID and edit-history IDs.
- Handle disconnects, reconnect backoff, rate-limit headers and missed-event recovery.
- Use recent search (last seven days) for catch-up/reconciliation where available; do not treat polling as equivalent to a persistent stream.

Official docs currently describe recent search, full-archive search access, query operators such as `from:`, and filtered-stream examples. Access levels and pricing must be checked in the X developer console at procurement time. Do not assume the filtered stream is included in every plan.

Do not use browser scraping or unofficial endpoints as a production dependency. Third-party X data vendors may be evaluated only after legal/contractual review of coverage, latency, provenance, retention, redistribution and trading use rights.

## 5.4 Event pipeline

```text
X Filtered Stream / approved provider
  -> durable raw-event queue
  -> dedupe + account identity check
  -> language / entity / asset extraction
  -> event-type classifier
  -> source reliability + novelty + relevance scoring
  -> corroboration / official-source verification
  -> market-impact watch window
  -> risk gate (usually reduce/disable trading first)
  -> strategy event bus
  -> audit log + dashboard alert
```

Use a durable queue so a classifier restart does not lose events. Preserve original post ID, URL, author ID, created timestamp, received timestamp, edit history, quoted/replied-to post references and source.

## 5.5 Event taxonomy

Classify posts into controlled labels:

- `REGULATION`
- `MONETARY_POLICY`
- `FISCAL_POLICY`
- `SANCTIONS`
- `ETF_OR_MARKET_PRODUCT`
- `EXCHANGE_LISTING_OR_DELISTING`
- `EXCHANGE_OUTAGE_OR_HACK`
- `PROTOCOL_EXPLOIT_OR_SECURITY`
- `STABLECOIN_RESERVE_OR_DEPEG`
- `CORPORATE_CRYPTO_TREASURY`
- `PAYMENT_OR_ADOPTION`
- `TOKEN_OR_ASSET_MENTION`
- `POLITICAL_OR_GEOPOLITICAL`
- `RUMOR_OR_UNVERIFIED_CLAIM`
- `NON_MARKET_RELEVANT`

Each event must include:
- source account ID and resolved identity,
- post ID and canonical URL,
- post creation and local receive timestamps,
- detected assets/markets,
- event type,
- extracted factual claims,
- whether it is original, reply, quote or repost,
- confidence and evidence,
- corroborating source URLs,
- novelty score,
- possible direction only if supported by a validated event model,
- uncertainty/ambiguity notes,
- expiration/cooldown time.

## 5.6 X event scoring

Do not let an LLM directly return a buy/sell command. The classifier returns features. A deterministic policy decides what those features are allowed to do.

Example event relevance score:

\[
Score = w_sS + w_nN + w_aA + w_cC + w_vV
\]

Where:
- \(S\): source reliability (identity/authority, not popularity),
- \(N\): novelty versus recent posts,
- \(A\): asset relevance,
- \(C\): corroboration,
- \(V\): validated historical event impact for that source/event class.

All components are calibrated from historical outcomes. No arbitrary score should be treated as a probability.

Initial policy:
- Unverified account or unclear post: log and alert only.
- Rumor/claim without primary confirmation: risk-off flag only; no directional order.
- Official exchange outage/hack/security notice: disable affected venue/asset entries, cancel working orders if safe, reconcile balances.
- Major central-bank/regulatory statement: pause new entries around the event until spread/volatility stabilize; do not infer direction from a headline alone.
- Verified material company/protocol announcement: create event record, check corroboration and observe actual market response; only a separately validated event strategy may propose a trade.
- Conflicting sources, deleted post, suspicious account compromise, ambiguous joke/sarcasm or quote without context: no trade.

## 5.7 Seconds-level reaction policy

For a high-impact post:
1. Receive event.
2. Verify the author ID against the approved watchlist.
3. Check if it is a repost, reply, quote or edited post.
4. Extract entities and asset references.
5. Check primary-source links and at least one independent reliable source when feasible.
6. Query current market state and data freshness.
7. Evaluate spread, volatility, order-book depth and current position.
8. Initially, the safest automated action is often **cancel new passive quotes / pause new entries / reduce risk**, not immediately chase the price.
9. A directional trade is allowed only if a separately validated event-response strategy has positive net expectancy for that event class and current execution conditions.
10. Log the full decision and monitor retractions/corrections.

The event subsystem must never treat a public figure's post as guaranteed truth or guaranteed market direction. Accounts can be compromised, posts can be sarcastic, vague, edited, deleted, misinterpreted or quickly contradicted.

---

# 6. Other intelligence feeds

Prioritize sources that add information not already embedded in price.

| Feed | Candidate provider/API | Use | Initial priority |
|---|---|---|---|
| Exchange market data | Selected exchange official WebSocket | Trades, L2, book, fills | P0 |
| X posts | Official X API v2 | Public statements and announcements | P1 |
| Exchange status | Official exchange status/announcements | Outages, maintenance, incidents | P1 |
| Macro data | FRED API | Historical macro series and release data | P2 |
| Economic calendar | Trading Economics API or another licensed calendar | Scheduled event risk | P2 |
| On-chain transfers | Whale Alert API | Entity-labelled large transfer alerts | P2 |
| Entity attribution | Arkham Intel API | Wallet/entity attribution, with confidence | P2 |
| Protocol data | Official protocol APIs, blockchain RPC/indexer | Contract events, token supply, protocol status | P2 |
| General news | Licensed crypto/financial news API | Discovery and corroboration | P2 |
| Broad web monitoring | GDELT DOC API | Broad news discovery, not primary truth | P3 |
| Market reference | CoinGecko or another market-data provider | Cross-checks and metadata | P3 |

Provider selection is conditional on current pricing, coverage, latency, rate limits, redistribution/trading rights and data quality. Do not buy all subscriptions up front. Start with exchange-native feeds and X access; add paid sources only when a measured backtest shows incremental value.

### On-chain interpretation rules

A large transfer does not automatically mean selling. It may be:
- internal exchange wallet movement,
- custody reshuffling,
- collateral movement,
- bridge activity,
- exchange hot/cold wallet rotation,
- a transfer between entities under common control.

Use entity labels with confidence and corroborate with exchange flows, transaction context and market response. Never auto-short solely because a whale sends tokens to an exchange.

---

# 7. Data architecture

## 7.1 Services

Initial deployment can be a modular monolith with separate processes for failure isolation. Avoid premature microservices.

1. `market-ingest`: exchange WebSocket clients.
2. `x-ingest`: official X API stream and reconciliation.
3. `news-ingest`: optional licensed news/calendar providers.
4. `normalizer`: common event schemas.
5. `book-engine`: local order book and integrity checks.
6. `feature-engine`: deterministic features and rolling windows.
7. `event-intelligence`: asynchronous text classification/corroboration.
8. `strategy-runner`: directional and arbitrage strategies.
9. `risk-engine`: independent final approval/rejection.
10. `execution-engine`: orders, cancel/replace, reconciliation.
11. `recorder`: append-only event and decision log.
12. `dashboard-api` and `dashboard-ui`.
13. `supervisor`: health checks, watchdog and kill-switch coordination.

## 7.2 Recommended stack

| Layer | Recommendation | Reason |
|---|---|---|
| Core execution/data plane | Rust | Predictable performance, memory safety, low overhead |
| Research/backtests | Python | Ecosystem for statistics, ML and notebooks |
| Strategy research | Polars, NumPy, SciPy, statsmodels, scikit-learn, LightGBM | Data and model experimentation |
| Trading engine option | NautilusTrader | Event-driven backtesting/live architecture; evaluate venue adapter fit before adopting |
| Exchange protocol | Native official WebSocket/REST adapters | Avoid hidden behavior and keep full control |
| Historical storage | Parquet + DuckDB | Cheap, reproducible research data |
| Operational database | PostgreSQL | Orders, configs, account state, audit metadata |
| Metrics | Prometheus + Grafana | Health, latency, error and risk metrics |
| Logs/traces | OpenTelemetry + structured JSON logs | Correlation and debugging |
| Queue | NATS JetStream or Redis Streams | Durable event delivery; choose one, don't add both initially |
| Dashboard | React + TypeScript | Monitoring and operator controls |
| Deployment | Linux VPS + Docker Compose initially | Simple, reproducible deployment |
| Secrets | Environment injection from a secrets manager; never commit secrets | Key protection |
| CI | GitHub Actions | Test, lint, build, dependency/security checks |

NautilusTrader should be evaluated as the candidate event-driven trading/backtesting framework. If its current adapter, order model or account model does not match the selected venue's requirements, implement a thin native adapter rather than forcing a mismatch.

## 7.3 Storage tables

Minimum logical schema:

- `market_events`: normalized market events with exchange timestamp, receive timestamp and sequence.
- `book_snapshots`: periodic book checkpoints.
- `book_deltas`: raw/normalized depth deltas.
- `trades`: exchange trades and aggressor classification.
- `external_events`: X/news/on-chain events with source and provenance.
- `feature_snapshots`: feature values and version.
- `model_predictions`: model version, horizon, probability, expected return.
- `trade_proposals`: strategy proposals.
- `risk_decisions`: pass/reject plus reason codes and risk snapshot.
- `order_intents`: immutable intended order.
- `orders`: exchange order state.
- `fills`: immutable execution fills.
- `positions`: reconciled holdings and cost basis.
- `balances`: balance snapshots.
- `pnl_snapshots`: realized/unrealized P&L and fees.
- `system_health`: data freshness, lag, disconnects, clock drift.
- `config_versions`: signed versioned configs.
- `model_registry`: model artifact, training data window, metrics and approval.

Use UTC for persisted wall-clock timestamps and monotonic nanoseconds for local durations. Store raw exchange payloads or a lossless archive for forensic replay, subject to data licensing and retention rules.

---

# 8. Risk engine specification

The risk engine is the only component permitted to authorize an order.

## 8.1 Initial conservative limits

These are starting guardrails for testing, not optimized financial advice. They must be validated against minimum order sizes and fees.

| Control | Initial setting |
|---|---:|
| Live leverage | 1x spot only |
| Risk budget per trade | Target <= 0.5% of current equity |
| Daily realized + unrealized loss stop | 1% of start-of-day equity |
| Weekly drawdown review | 2% |
| Hard account drawdown stop | 5% from high-water mark |
| Concurrent directional positions | 1 |
| Averaging down | Prohibited |
| Martingale | Prohibited |
| Max open orders | 2, including protective/exit orders |
| New trades after data gap | Prohibited until book rebuilt |
| New trades during exchange uncertainty | Prohibited |
| API key withdrawals | Disabled |
| API permissions | Minimum required spot trade/read permissions |

If the exchange minimum size makes the risk budget impossible, skip the trade. Never increase risk to satisfy a minimum notional.

## 8.2 Cost-aware position sizing

Let:
- \(E\): current equity in quote currency.
- \(r\): risk fraction, initially 0.005.
- \(P_e\): expected entry price.
- \(P_s\): stop price.
- \(f_e,f_x\): entry and exit fee rates.
- \(s_e,s_x\): conservative entry and exit slippage rates.
- \(q\): base-asset quantity.

Approximate per-unit adverse loss:

\[
L_{unit}=|P_e-P_s|+P_e(f_e+s_e)+P_s(f_x+s_x)
\]

Risk budget:

\[
R=E\times r
\]

Risk-based quantity:

\[
q_{risk}=\frac{R}{L_{unit}}
\]

Capital-constrained quantity for spot:

\[
q_{capital}=\frac{E_{available}-feeBuffer}{P_e}
\]

Final quantity:

\[
q=\min(q_{risk},q_{capital},q_{liquidity},q_{venueMax})
\]

Then round **down** to venue step size and verify min quantity, max quantity, tick size and minimum notional. If the rounded order is below minimum notional or the effective risk exceeds budget, reject it.

Use conservative fee tier and slippage estimates, not best-case maker assumptions. A maker order may not fill; a post-only order may be rejected or remain unfilled.

## 8.3 Expected value gate

For a binary target/stop approximation:

\[
EV = p_w \times W - (1-p_w)\times L - C
\]

where:
- \(p_w\) is calibrated win probability,
- \(W\) is gross average win,
- \(L\) is gross average loss,
- \(C\) is all expected trading costs.

More generally:

\[
EV_{net}=E[GrossPnL]-E[Fees+Spread+Slippage+Funding+OtherCosts]
\]

Trade only when a conservative lower confidence bound for expected net value is above zero plus a safety margin. Use a block bootstrap or other method that respects serial dependence; do not assume trades are independent if they cluster in the same market regime.

## 8.4 Kill switches

Trigger on:
- daily loss limit reached,
- high-water drawdown limit reached,
- stale market feed,
- sequence gap/book invalid,
- private user stream disconnected,
- balance/position mismatch,
- unknown open order,
- repeated order rejects,
- rate-limit ban or API outage,
- abnormal spread/volatility,
- clock drift beyond configured tolerance,
- database/queue unavailable,
- X/news event classified as critical venue/security incident,
- model version/config mismatch,
- unexpected quantity/price or duplicate order intent.

Kill-switch sequence:
1. Set global `HALT_NEW_ORDERS`.
2. Cancel working entry orders.
3. Preserve protective exits where safe; do not blindly cancel all risk-reducing orders.
4. Reconcile balances, positions and open orders through private stream and REST.
5. If a position exists, follow a pre-approved safe policy (hold with verified protective stop or flatten with bounded slippage). Do not improvise during an outage.
6. Alert operator.
7. Require explicit operator review to re-enable.

---

# 9. Execution engine

## 9.1 Order lifecycle

Model order states explicitly:

`CREATED -> VALIDATED -> SUBMITTED -> ACKNOWLEDGED -> PARTIALLY_FILLED -> FILLED`

Other terminal/intermediate states:
`REJECTED`, `CANCELED`, `EXPIRED`, `UNKNOWN`, `PENDING_CANCEL`.

Never infer a fill from an HTTP 200 response. Acknowledge is not a fill. Use private order/execution streams and periodic REST reconciliation.

## 9.2 Execution policies

Initial supported policies:
- Marketable limit with a strict price collar.
- Post-only limit for experiments where queue and non-fill risk are modelled.
- Cancel/replace only when expected benefit exceeds extra fees, queue loss and API risk.
- Time-in-force chosen per venue and strategy.
- No unbounded market orders.
- No blind retry on ambiguous submission timeout.

Every order intent has a unique client ID. On timeout, query by client ID/order ID before retrying. Avoid duplicate submissions.

## 9.3 Slippage estimate

For a buy order of size \(Q\), calculate the order-book VWAP across asks until \(Q\) is filled:

\[
VWAP_{buy}(Q)=\frac{\sum_i p_i \Delta q_i}{Q}
\]

For sells, use bids. Compare VWAP to mid/reference price and include a stress buffer. Reject if depth is insufficient or expected impact exceeds the strategy's edge.

---

# 10. Backtesting and validation

## 10.1 Data requirements

For microstructure research, candles alone are insufficient. Obtain timestamped trades and order-book snapshots/deltas at the granularity available from the chosen venue. Preserve data gaps and quality flags. Do not silently interpolate missing order-book events.

## 10.2 Simulation requirements

A credible simulator must model:
- maker/taker fees and fee tier,
- bid/ask spread,
- order-book depth and VWAP,
- latency from feed to strategy and strategy to exchange,
- queue position or conservative queue assumptions for passive orders,
- partial fills,
- missed fills,
- cancel/replace latency,
- exchange rate limits and rejects,
- minimum notional, tick and step size,
- funding and basis if derivatives are simulated,
- exchange downtime and reconnects,
- event timestamps and X/news arrival delay,
- market impact where relevant.

NautilusTrader's event-driven backtest and latency-model capabilities are candidates for this work. Verify the simulator's assumptions against actual venue behavior.

## 10.3 Research protocol

- Define hypotheses before testing.
- Use chronological train/validation/test splits.
- Use walk-forward testing.
- Keep a final untouched holdout.
- Use purged/embargoed splits where labels overlap in time.
- Track every parameter search and model experiment.
- Compare against simple baselines.
- Report confidence intervals and block-bootstrap uncertainty.
- Measure performance by volatility regime, time-of-day, spread regime and event class.
- Include delisted/failed assets if testing a universe, avoiding survivorship bias.
- Do not optimize solely for win rate.

## 10.4 Required metrics

- Net P&L after all costs.
- Expectancy per trade.
- Profit factor.
- Win rate with confidence interval.
- Average win / average loss.
- Maximum drawdown and duration.
- Sharpe and Sortino with caveats for high-frequency autocorrelation.
- Turnover and total fees.
- Slippage versus arrival price.
- Fill rate and partial-fill rate.
- Adverse selection after fills.
- Exposure time.
- Tail loss / expected shortfall.
- Performance stability across regimes.
- Calibration (Brier score/reliability curve) for probabilistic models.
- Out-of-sample performance and backtest-overfitting diagnostics.

A 70% win rate is not a requirement unless the reward/risk and net expectancy make it economically meaningful. A lower win rate with positive expectancy can be better than a high win rate with rare catastrophic losses.

## 10.5 Promotion gates

A strategy may move from research to paper only if:
- No look-ahead or survivorship leakage is identified.
- All fees, spread, slippage and latency are modelled.
- Net expectancy is positive on untouched out-of-sample data.
- Results are not driven by one short time period or a tiny number of trades.
- Parameter perturbations do not destroy the edge.
- Risk metrics remain within pre-registered limits.

Paper to live requires:
- At least 4 weeks of continuous paper operation (longer if sample size is inadequate).
- At least 300 paper trades for a high-frequency strategy, or a justified statistically sufficient sample for lower-frequency strategies.
- Predicted and observed execution assumptions reconcile.
- No unresolved critical operational defects.
- Daily kill-switch and recovery drills passed.
- Explicit operator sign-off.

Live to increased size requires a separate review after sufficient live observations. No automatic capital scaling.

---

# 11. AI and machine learning policy

Use ML only where it adds measurable value beyond simple baselines.

Potential models:
- Logistic regression: short-horizon up/down probability.
- Ridge/Elastic Net: expected return.
- LightGBM: nonlinear tabular features.
- Regime classifier: volatility/liquidity regime.
- Anomaly detector: abnormal spread, flow or event conditions.
- NLP classifier: X/news event type, entities, novelty and claim extraction.

Model requirements:
- Versioned features and labels.
- No future information in features.
- Time-aware validation.
- Probability calibration.
- Drift monitoring.
- Explicit model approval and rollback.
- Reproducible training data snapshot.
- Model output includes uncertainty and out-of-distribution flag.

LLM usage:
- Summarize long posts/threads and linked official documents.
- Extract claims, entities and event categories.
- Identify contradictions and missing context.
- Draft human-readable incident summaries.

LLM must not:
- Generate arbitrary orders.
- Change risk limits.
- Interpret a vague post as certain direction.
- execute tools with exchange keys.
- run inside the latency-critical order path.

---

# 12. Security and operational controls

- Separate read-only market-data credentials from trade credentials where possible.
- Disable withdrawals on trading keys.
- Restrict API keys to the bot host IP if supported.
- Use a dedicated exchange subaccount if available.
- Store secrets in a secrets manager or protected environment injection; never in Git, Docker image, logs or dashboard.
- Rotate keys and document revocation.
- Use TLS and verify certificates.
- Run containers as non-root with minimal capabilities.
- Pin dependencies and scan for known vulnerabilities.
- Enable MFA on exchange and cloud accounts.
- Back up database and configuration; encrypt backups.
- Keep production and paper credentials separate.
- No public dashboard exposing balances, API state or controls.
- Require authentication and role-based access for operator controls.
- Audit every manual action.
- Use immutable append-only records for order/fill and risk decisions.
- Maintain a tested recovery procedure after VPS reboot, process crash, exchange outage and database outage.

---

# 13. Monitoring dashboard and alerting

Dashboard sections:

1. **Account:** equity, available balance, realized/unrealized P&L, fees, exposure.
2. **Market:** best bid/ask, spread, depth, volatility, data age, book validity.
3. **Strategies:** active strategy/version, signals, rejected proposals and reason codes.
4. **X intelligence:** latest posts, source identity, event class, confidence, corroboration, timestamps.
5. **Orders:** order state, client ID, exchange ID, fills, partial fills, slippage.
6. **Risk:** daily loss, drawdown, remaining risk budget, kill-switch state.
7. **Latency:** p50/p95/p99 per pipeline stage and exchange acknowledgement/fill.
8. **Health:** WebSocket status, queue lag, database, clock drift, API rate limits.
9. **Research:** paper/live comparison, model drift and strategy promotion status.

Alerts:
- feed disconnected or stale,
- book invalid,
- X stream disconnected,
- high-impact event,
- order rejected or unknown,
- partial fill unresolved,
- balance mismatch,
- daily loss threshold,
- drawdown threshold,
- kill-switch activated,
- unusual latency or slippage,
- API rate limit reached.

Use Prometheus + Grafana and an alert channel (e.g. Telegram bot or email). Alerts are informational; Telegram/email must not be the sole control plane.

---

# 14. Repository layout

Suggested monorepo:

```text
project-alpha/
  PLAN.md
  README.md
  LICENSE
  .gitignore
  .env.example
  docker-compose.yml
  rust/
    crates/
      domain/
      exchange-adapters/
      market-ingest/
      orderbook/
      features/
      strategy-api/
      risk-engine/
      execution-engine/
      event-bus/
      recorder/
      supervisor/
  python/
    research/
    backtests/
    models/
    notebooks/
    x_intelligence/
    data_quality/
  dashboard/
    src/
  configs/
    venues/
    symbols/
    risk/
    strategies/
    x_watchlist/
    event_policy/
  migrations/
  schemas/
    market-event.schema.json
    external-event.schema.json
    trade-proposal.schema.json
    order-intent.schema.json
  tests/
    unit/
    integration/
    replay/
    fault-injection/
    contract/
  infra/
    docker/
    systemd/
    monitoring/
    backup/
  docs/
    architecture/
    runbooks/
    research/
    decisions/
```

Keep `.env` and all real secrets out of Git. `.env.example` contains names only, never values.

---

# 15. Implementation roadmap: zero to production

## Phase 0 — discovery and compliance (Week 1)

Tasks:
- Choose candidate exchange after jurisdiction/account eligibility review.
- Create API test account/subaccount if supported.
- Verify spot symbol, fee schedule and filters.
- Create X developer app and check current API access, pricing and filtered-stream entitlement.
- Build the initial X watchlist with stable user IDs and evidence.
- Confirm data-provider terms permit internal algorithmic trading use.
- Define the risk policy and emergency safe-state behavior.
- Record architecture decisions.

Deliverables:
- `docs/decisions/0001-venue.md`
- `docs/decisions/0002-x-api.md`
- `configs/risk/production.yaml`
- `configs/x_watchlist/accounts.yaml`
- cost estimate for required API plans and VPS.

Exit criteria: all required API access and legal/contractual questions resolved; otherwise remain in offline research.

## Phase 1 — repository and foundations (Week 1–2)

Tasks:
- Create monorepo and CI.
- Define typed domain objects and schemas.
- Implement config validation and secret handling.
- Set up Docker Compose for PostgreSQL, NATS JetStream (or chosen queue), Prometheus and Grafana.
- Add structured logs, metrics and health endpoints.
- Implement a deterministic event recorder/replayer.

Exit criteria:
- CI passes.
- Service restarts cleanly.
- Secrets do not appear in logs or images.
- Event replay is deterministic.

## Phase 2 — exchange market-data ingestion (Week 2–3)

Tasks:
- Implement public WebSocket connection.
- Subscribe to trades and L2 order book for one symbol.
- Implement snapshot/delta reconstruction.
- Detect sequence gaps and stale data.
- Implement reconnect with jittered exponential backoff.
- Archive raw and normalized events.
- Add exchange server-time sampling and local receive timestamps.
- Implement symbol metadata and filters.

Exit criteria:
- 72-hour soak test with reconnects.
- No silent book corruption.
- Data-gap detection and snapshot recovery tests pass.
- Measured latency histograms available.

## Phase 3 — X intelligence ingestion (Week 3–4)

Tasks:
- Integrate official X API v2 filtered stream if entitled.
- Add approved account rules and relevant topic rules.
- Add recent-search reconciliation/catch-up where available.
- Persist original post ID, author ID, created time, receive time, edits and references.
- Implement dedupe and account identity verification.
- Add stream disconnect alerts and API usage/cost metrics.
- Build event taxonomy and deterministic keyword/regex baseline.

Exit criteria:
- Test posts from controlled accounts/topics arrive and are deduplicated.
- Stream reconnect/catch-up behavior tested.
- API access, rate limits, pricing and usage policy documented.
- No trade action is triggered by an unverified post.

## Phase 4 — feature engine (Week 4–5)

Tasks:
- Implement mid, spread, OBI, microprice, OFI, TFI, volatility and depth features.
- Add multi-window aggregation.
- Add data-quality flags.
- Unit-test formulas against hand-calculated fixtures.
- Save feature snapshots for every strategy decision.
- Benchmark CPU and p99 computation latency.

Exit criteria:
- Deterministic replay yields identical features.
- Invalid/stale book blocks strategy evaluation.
- Feature calculation meets internal target on selected VPS.

## Phase 5 — research and backtesting (Week 5–8)

Tasks:
- Collect/retrieve historical trades and depth data.
- Build realistic order-book replay.
- Implement latency, fees, slippage and partial-fill models.
- Implement simple baselines and directional microstructure strategy.
- Implement cross-exchange and basis arbitrage simulators.
- Add walk-forward testing and parameter-search tracking.
- Evaluate incremental predictive value of X event features.

Exit criteria:
- At least one strategy meets promotion criteria on untouched data.
- If none does, do not proceed to live trading; continue research or stop.
- Backtest assumptions and data limitations documented.

## Phase 6 — risk and execution (Week 7–9)

Tasks:
- Implement independent risk service/module.
- Implement cost-aware sizing and venue filter rounding.
- Implement order state machine and client IDs.
- Implement private user stream and REST reconciliation.
- Implement safe cancel/replace and ambiguous-timeout handling.
- Implement kill-switch and recovery runbooks.
- Add fault-injection tests.

Exit criteria:
- Testnet/sandbox or exchange demo environment passes order lifecycle tests.
- Duplicate order tests pass.
- Disconnect and restart tests reconcile orders/positions.
- Risk engine rejects every invalid test case.

## Phase 7 — paper trading (Week 9–13+)

Tasks:
- Run continuously against live market data.
- Execute in simulated account.
- Compare simulated fills with real book/arrival prices.
- Monitor X event classifications and missed/false alerts.
- Collect adequate trade sample.
- Review weekly performance and drift.
- Run incident drills.

Exit criteria:
- Minimum four weeks and adequate sample size.
- Positive out-of-sample and paper net expectancy after conservative costs.
- Risk/latency/reconciliation gates pass.
- No unresolved critical incident.

## Phase 8 — limited live deployment (after gates)

Tasks:
- Start with spot-only and tiny size.
- Use one venue and one symbol.
- Keep arbitrage engine paper-only.
- Set conservative daily loss and drawdown stops.
- Manually review the first live fills and daily reports.
- Compare actual fees/slippage/fills to paper assumptions.
- Do not increase size automatically.

Exit criteria:
- Separate operator approval after a defined live observation window.
- No increase in capital until execution and risk behavior are understood.

## Phase 9 — production hardening and expansion

Only after stable live results:
- Add second venue read-only market data.
- Evaluate pre-funded cross-exchange arbitrage.
- Add more symbols only after per-symbol validation.
- Consider derivatives/basis only with a separate margin/liquidation risk review and explicit capital approval.
- Consider a closer VPS region only if measured latency improves net results.
- Add paid news/on-chain sources only if their incremental value is proven.
- Conduct disaster recovery and key-rotation drills quarterly.

---

# 16. API and tool inventory

The following is a shortlist, not a requirement to purchase every service. Confirm current plans, prices, terms and availability at implementation time.

## Required / first wave

### Exchange API
- Binance Spot API documentation: https://github.com/binance/binance-spot-api-docs
- Binance developer docs: https://developers.binance.com/
- Bybit V5 API docs: https://bybit-exchange.github.io/docs/v5/intro
- Coinbase Advanced Trade API: https://docs.cdp.coinbase.com/advanced-trade/docs/welcome

Select one venue for live v1 after due diligence. Do not assume regional eligibility or a fixed fee schedule.

### X
- X API overview and policy: https://help.x.com/en/rules-and-policies/x-api
- X developer documentation: https://docs.x.com/
- X posts search documentation: https://docs.x.com/x-api/posts/search/introduction
- X API filtered-stream examples: https://github.com/xdevplatform/samples

Use the official filtered stream when included in the account's current access plan. Recent search can support catch-up/reconciliation but is not equivalent to continuous streaming. Check current usage-based pricing and developer terms before committing.

### Event-driven trading framework
- NautilusTrader docs: https://nautilustrader.io/docs/
- NautilusTrader repository: https://github.com/nautechsystems/nautilus_trader

Evaluate adapter coverage and simulation assumptions. Do not assume the framework automatically solves venue-specific behavior.

## Optional intelligence providers

- FRED API: https://fred.stlouisfed.org/docs/api/fred/
- Trading Economics API: https://docs.tradingeconomics.com/
- Whale Alert API: https://developer.whale-alert.io/api-account/documentation
- Arkham Intel API: https://arkm.com/api/docs
- CoinGecko API: https://docs.coingecko.com/
- GDELT DOC API: https://docs.gdeltproject.org/
- DefiLlama API docs: https://docs.defillama.com/

Buy only when a concrete use case, data quality review and measured incremental value justify the recurring cost.

## Infrastructure / engineering

- Rust: https://www.rust-lang.org/
- Tokio async runtime: https://tokio.rs/
- Python: https://www.python.org/
- Polars: https://docs.pola.rs/
- DuckDB: https://duckdb.org/docs/
- PostgreSQL: https://www.postgresql.org/docs/
- NATS JetStream: https://docs.nats.io/nats-concepts/jetstream
- Prometheus: https://prometheus.io/docs/
- Grafana: https://grafana.com/docs/
- OpenTelemetry: https://opentelemetry.io/docs/
- Docker Compose: https://docs.docker.com/compose/
- GitHub Actions: https://docs.github.com/actions

## Security

- OWASP ASVS: https://owasp.org/www-project-application-security-verification-standard/
- OWASP Secrets Management Cheat Sheet: https://cheatsheetseries.owasp.org/cheatsheets/Secrets_Management_Cheat_Sheet.html
- Dependabot: https://docs.github.com/code-security/dependabot

---

# 17. Cost planning

Keep fixed monthly costs close to zero during research. The largest early cost should be data acquisition only if free exchange history is insufficient.

Budget categories:
- VPS: one small Linux instance initially; benchmark before upgrading.
- Database/storage: local attached disk or object storage with backups.
- X API: depends on current access tier, streaming entitlement and usage.
- On-chain/news APIs: optional, usage-based or subscription; defer.
- Exchange fees: variable and potentially the largest recurring cost once trading.
- Domain/dashboard: optional.
- Monitoring: self-host initially.

Do not buy institutional data, co-location, FPGA hardware or enterprise feeds with a $100 account. The cost cannot be justified before strategy validation.

Before enabling live trading, calculate expected monthly cost and the break-even net return required merely to cover infrastructure and data subscriptions. If the break-even requirement is unrealistic, keep the system in paper mode.

---

# 18. Configuration examples

`configs/risk/paper.yaml`:

```yaml
mode: paper
allow_live_orders: false
spot_only: true
max_leverage: 1
risk_per_trade_fraction: 0.005
daily_loss_limit_fraction: 0.01
weekly_review_drawdown_fraction: 0.02
hard_drawdown_fraction: 0.05
max_directional_positions: 1
allow_averaging_down: false
allow_martingale: false
require_valid_orderbook: true
require_fresh_private_state: true
require_positive_net_expectancy: true
```

`configs/strategies/directional.yaml`:

```yaml
enabled: true
mode: paper
symbols:
  - BTCUSDT
windows_ms:
  - 100
  - 250
  - 1000
  - 5000
  - 30000
features:
  - obi
  - ofi
  - microprice_displacement
  - trade_flow_imbalance
  - realized_volatility
  - spread_bps
  - depth_vwap_impact
require_out_of_sample_approval: true
max_data_age_ms: 250
```

`configs/strategies/arbitrage.yaml`:

```yaml
enabled: true
mode: paper
strategy_types:
  - cross_exchange_spot
  - spot_perpetual_basis
  - triangular
require_prefunded_inventory: true
include_all_fees: true
include_slippage: true
include_partial_fill_risk: true
include_unwind_cost: true
```

`configs/x_watchlist/accounts.yaml`:

```yaml
version: 1
accounts:
  - label: public_figure_trump
    username_at_last_verification: null
    x_user_id: null
    category: public_figure
    assets: [BTC, ETH, CRYPTO_MARKET]
    enabled: false
    verified_by_operator: false
    verification_evidence_url: null
    last_verified_at: null

  - label: public_figure_musk
    username_at_last_verification: null
    x_user_id: null
    category: public_figure
    assets: [BTC, DOGE, CRYPTO_MARKET]
    enabled: false
    verified_by_operator: false
    verification_evidence_url: null
    last_verified_at: null

  - label: el_salvador_official
    username_at_last_verification: null
    x_user_id: null
    category: government
    assets: [BTC, CRYPTO_MARKET]
    enabled: false
    verified_by_operator: false
    verification_evidence_url: null
    last_verified_at: null
```

Watchlist entries are intentionally disabled until the operator resolves and verifies the official account ID. Do not hard-code a username as proof of identity.

---

# 19. Acceptance checklist before any live deployment

## Data
- [ ] Exchange stream connected and monitored.
- [ ] Book snapshot/delta logic tested.
- [ ] Sequence gaps and stale data fail closed.
- [ ] X API entitlement, pricing and terms confirmed.
- [ ] X account identities verified by stable ID.
- [ ] X reconnect/catch-up and dedupe tested.
- [ ] Timestamps and clock drift monitored.

## Strategy
- [ ] Hypothesis documented before testing.
- [ ] Baselines included.
- [ ] No look-ahead leakage.
- [ ] Realistic fees, spread, slippage and latency included.
- [ ] Untouched out-of-sample period positive.
- [ ] Regime and parameter stability reviewed.
- [ ] Paper trading meets sample and time gates.

## Risk
- [ ] Cost-aware sizing implemented.
- [ ] Exchange filters applied dynamically.
- [ ] Daily and hard drawdown stops tested.
- [ ] Risk engine cannot be bypassed.
- [ ] No leverage / no averaging down.
- [ ] Emergency cancel/reconcile policy tested.

## Execution
- [ ] Unique client order IDs.
- [ ] Ambiguous timeout does not duplicate order.
- [ ] Private stream and REST reconciliation work.
- [ ] Partial fills and rejects handled.
- [ ] Slippage and fill quality recorded.
- [ ] No blind market orders.

## Security / operations
- [ ] Withdrawal permission disabled.
- [ ] API key restricted by IP if supported.
- [ ] Secrets absent from Git/logs/images.
- [ ] MFA enabled.
- [ ] Backup and restore tested.
- [ ] Crash/restart recovery tested.
- [ ] Monitoring and alerts active.
- [ ] Operator kill switch tested.
- [ ] Legal, tax and venue eligibility checked.

---

# 20. Key research and documentation references

These links are the primary technical references used to shape the plan. Re-check live documentation because APIs, fees, access tiers and terms change.

1. X API overview and access/policy: https://help.x.com/en/rules-and-policies/x-api
2. X Search Posts documentation: https://docs.x.com/x-api/posts/search/introduction
3. X API sample filtered stream client: https://github.com/xdevplatform/samples
4. Binance official Spot API docs: https://github.com/binance/binance-spot-api-docs
5. Binance WebSocket streams: https://github.com/binance/binance-spot-api-docs/blob/master/web-socket-streams.md
6. Binance symbol filters: https://github.com/binance/binance-spot-api-docs/blob/master/filters.md
7. Bybit rate limits: https://bybit-exchange.github.io/docs/v5/rate-limit
8. Bybit order book stream: https://bybit-exchange.github.io/docs/v5/websocket/public/orderbook
9. Coinbase Advanced Trade WebSocket feeds: https://help.coinbase.com/en/developer-platform/websocket-feeds/advanced-trade
10. NautilusTrader: https://nautilustrader.io/docs/
11. NautilusTrader backtesting execution flow: https://nautilustrader.io/docs/latest/concepts/backtesting/execution-flow/
12. FRED API: https://fred.stlouisfed.org/docs/api/fred/
13. Trading Economics API: https://docs.tradingeconomics.com/
14. Whale Alert API: https://developer.whale-alert.io/api-account/documentation
15. Arkham API: https://arkm.com/api/docs

Research findings to keep in mind:
- Apparent triangular arbitrage can disappear after fees and executable liquidity are modelled.
- Retail taker orders can suffer adverse selection and execution shortfall.
- Cross-venue arbitrage requires capital, inventory and robust handling of one-leg fills.
- Public posts can be delayed, edited, deleted, misinterpreted or false; account identity and corroboration matter.
- A high historical win rate is not evidence of a durable edge without out-of-sample validation and uncertainty estimates.

---

# 21. Final implementation directive for the coding agent

Build in the order specified. Do not skip phases because a later component looks exciting.

The first runnable milestone is **not** an autonomous money-making bot. It is a deterministic market-data recorder that reconstructs a valid order book, records trades, timestamps every stage and replays the same data reproducibly.

The second milestone adds the X stream and event provenance. The third adds features and strategy research. The fourth adds the risk gate and paper execution. Live trading is a separately approved deployment stage.

When uncertain, the coding agent must:
- prefer official documented APIs,
- record assumptions and unresolved questions,
- add tests before implementation shortcuts,
- fail closed,
- keep the strategy disabled,
- never fabricate data or backtest fills,
- never claim a guaranteed return,
- never enable live orders without an explicit production configuration and operator sign-off.

**Definition of done:** Project Alpha is production-ready only when its data, strategy, risk, execution, security, recovery and monitoring gates all pass. A profitable backtest alone is not production readiness. Production readiness does not guarantee future profitability.
