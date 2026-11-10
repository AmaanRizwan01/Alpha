# Phase 5 Gate Evidence — Proxmox Debian 12 LXC Deployment & 8-Week Paper Burn-In Launch

**Gate Status:** SIGNED OFF & OPERATIONAL  
**Evaluation Date:** November 9, 2026  
**Evaluator:** Lead Quantitative Rust/Python Systems Architect  
**Git Branch:** `feat/p5-s3-live-ws-paper-sim` (targeting `main`)  
**Target Infrastructure:** Local Proxmox VE Server (Debian 12 LXC CTID 101, Ajman, UAE)  
**Assigned IP:** `192.168.1.230`  

---

## 1. Executive Summary & Verification Matrix

Phase 5 transitions Project Alpha from local unit and fault-drill testing to 24/7 autonomous paper burn-in execution on local dedicated hardware. An unprivileged Debian 12 container was provisioned, secured, and equipped with the compiled release binary, external watchdog heartbeat, and operator emergency kill switch. Sprint 5.3 wired real-time WebSocket market feeds and a deterministic paper fill matching engine with zero unwrap/expect and pure-Rust TLS.

| Gate Criteria | Requirement | Observed Metric / Result | Status |
|---|---|---|---|
| **LXC Provisioning** | Dedicated Debian 12 container on Proxmox VE | CTID `101` (`alpha-node`), 2 vCPUs, 2048 MB RAM, 16 GB rootfs on `local-lvm` | **PASSED** |
| **Security Hardening** | Unprivileged execution & least-privilege permissions | `unprivileged 1`, user `alpha`, `chmod 600 .env`, `chmod 700 data`, `ProtectSystem=strict` | **PASSED** |
| **Native Compilation** | Native Debian 12 release build without cross-compilation errors | Compiled via Rust 1.99.1 in 51s, zero warnings, zero unsafe code | **PASSED** |
| **Systemd 24/7 Daemon** | Persistent background service with restart resilience | `alpha-node.service` running active, 656 KB memory footprint, async runtime | **PASSED** |
| **Watchdog Heartbeat** | Independent external monitoring and health ping | `alpha-heartbeat.timer` executing `heartbeat.sh` every 60s, checking PID and DB | **PASSED** |
| **Emergency Flatten Drill** | Immediate halt on operator kill switch with Telegram alert | `emergency_flatten.sh` sets SQLite halt state, delivers Telegram alert, stops loop | **PASSED** |
| **Cold-Boot Recovery** | Clean resumption after halt state clearance | SQLite unhalt verified, daemon restarts to `RUNNING / HEALTHY` | **PASSED** |
| **Live WebSocket Feeds** | Combined stream parsing (`bookTicker`, `trade`, `kline`) with pure-Rust TLS | Verified live against Binance public stream; zero-loss JSON deserialization | **PASSED** |
| **Paper Fill Simulator** | Deterministic passive fill matching, atomic stops, fee & slippage tracking | Maker queue simulation, E0/E1/E2 trailing ratchet, 0 panics | **PASSED** |
| **Workspace Test Suite** | 100% pass across Rust workspace and Python research | 69 Rust workspace tests + 18 pytest tests passing with zero clippy warnings | **PASSED** |

---

## 2. Infrastructure & Systemd Configuration

### Container Specification
* **Host:** Proxmox VE 7.0.14-17 (Ajman node)
* **CTID / Hostname:** `101` / `alpha-node`
* **Network:** `eth0` attached to `vmbr0` (`192.168.1.230/24`)
* **Storage:** 16 GB rootfs on thin pool `local-lvm`
* **User:** `alpha:alpha` (unprivileged system daemon user, UID mapped)

### Active Services
1. **`alpha-node.service`:**
   ```text
   Active: active (running)
   Main PID: 8770 (alpha-node)
   Memory: 656.0K
   Nov 08 22:53:25 alpha-node alpha-node[8770]: Alpha trade daemon initialized on database: data/alpha_state.db (mode=paper, symbols=["BTCUSDT", "ETHUSDT"], max_risk=0.01, cb_drawdown=0.06)
   Nov 08 22:53:25 alpha-node alpha-node[8770]: Entering 24/7 daemon monitoring loop (interval: 60s)...
   ```
2. **`alpha-heartbeat.timer`:**
   ```text
   NEXT                        LEFT     LAST                        PASSED   UNIT                  ACTIVATES
   Sun 2026-11-08 22:55:12 UTC 40s left Sun 2026-11-08 22:54:12 UTC 20s ago  alpha-heartbeat.timer alpha-heartbeat.service
   ```

---

## 3. Emergency Flatten & Circuit Breaker Verification

The emergency kill switch was tested in the live container environment:
1. Triggered `/opt/alpha/scripts/emergency_flatten.sh` as user `alpha`.
2. Verified that `is_halted = 1` and `halt_reason = "operator emergency flatten"` were written to `/opt/alpha/data/alpha_state.db`.
3. Verified real Telegram alert delivery to the operator mobile device.
4. Confirmed `alpha-node` detected the halt on its monitoring tick and exited cleanly.
5. Confirmed that when systemd automatically attempted a restart 5s later, the node cold-boot check read SQLite and refused to trade:
   ```text
   Nov 08 22:51:26 alpha-node alpha-node[8680]: Refusing to start trade loop: Engine is halted (operator emergency flatten)
   ```
6. Executed recovery drill: cleared halt state in SQLite and restarted service. The node successfully resumed `RUNNING / HEALTHY` without state corruption.

---

## 4. Sprint 5.3: Live WebSocket Feeds & Deterministic Paper Matching

### Architectural Highlights
1. **Zero-Loss WebSocket Deserializer (`alpha-venues`):**
   - High-throughput deserialization for Binance combined streams: `bookTicker`, `trade`, `kline_1h`.
   - All monetary fields, prices, and quantities strictly parsed to `rust_decimal::Decimal`.
2. **Async Event Subscriber & Pure-Rust TLS (`alpha-node`):**
   - Pure-Rust TLS using `tokio-tungstenite` and `rustls` with `ring` crypto provider (eliminating external C library or OpenSSL requirements on Proxmox LXC).
   - Live stream smoke test (`tests/test_live_ws_smoke.rs`) connected to Binance Spot (`wss://stream.binance.com:9443`) and parsed real-world live quotes.
3. **Deterministic Paper Fill Matching Engine:**
   - Maker queue simulation: buy limit orders fill only when `ask <= limit_price` or live trade occurs at/below limit price.
   - Atomic stops: simulated positions maintain atomic stop-loss triggers with high-water mark ratchets ($E_0$ Chandelier ATR trail, $E_1$ stagnation cut, $E_2$ breakeven).
   - Accurate fee accounting: maker fee (0 bps for FDUSD, 10 bps for USDT) and taker fee deducted per trade, tracking exact net PnL and equity.
   - Synchronized SQLite WAL logging: state transitions (`PENDING`, `OPEN`, `FILLED`, `CLOSED`) persisted atomically with crash safety.

---

## 5. Sprint 5.4: FIFO Bar Buffer, REST Bootstrap & Live $H_1$ Signal Evaluator

### Architectural Highlights
1. **FIFO Bar Buffer (`buffer.rs`):**
   - High-performance, memory-capped 50-bar ring buffer rejecting duplicates and out-of-order bars.
2. **REST Kline Bootstrap (`bootstrap.rs`):**
   - Pre-warms the rolling buffer on cold boot via Binance REST (`GET /api/v3/klines?symbol={}&interval=4h&limit=50`).
3. **Live $H_1$ Evaluator (`signal.rs`):**
   - Pure-Rust evaluation of $H_1$ regime, ER chop filter ($\text{ER}_{24} \ge 0.45$), ATR volatility expansion ($\text{ATR}_{14} \ge 1.10 \times \text{ATR}_{48}$), 20-bar Donchian breakout, and taker volume ratio ($\ge 0.52$).
4. **End-to-End Order Execution:**
   - Evaluated on closed 4h bars, sizing risk at 1.0% ($r = 0.01$) with structural stop floor ($\ge 2.20\%$), dispatching atomic paper limit entry and protective stops into SQLite.

---

## 6. Sprint 5.5: Telegram Trade Alerts & Node Telemetry CLI

### Architectural Highlights
1. **Non-Blocking Alert Dispatcher (`telegram.rs`):**
   - Asynchronous `tokio::spawn` notification delivery via `reqwest::Client`. Acts as safe no-op when unconfigured, protecting market event loops from blocking.
   - Real-time formatters for startup, signal approvals, order submissions, maker fills, trailing stops, and circuit breaker halts.
2. **Live Telemetry CLI (`status.rs`):**
   - Operator command (`alpha-node status <config_or_db>`) querying SQLite WAL for engine state, equity HWM, drawdown headroom, active positions, and recent 5 orders.
   - Automatic parent-directory database path resolution for out-of-tree invocation.

---

## 7. Sprint 5.6: Interactive Telegram Bot Controls & Reply Buttons

### Architectural Highlights
1. **Long-Polling Listener (`telegram_listener.rs`):**
   - Zero-inbound-port HTTPS long-polling (`getUpdates`) running inside Proxmox LXC behind NAT.
   - Strict authorization guarding: accepts commands exclusively from configured `TG_CHAT_ID`.
2. **Interactive Slash Commands & Persistent Reply Buttons:**
   - Overwrote old bot slash commands with native Project Alpha menu (`/status`, `/positions`, `/pnl`, `/orders`, `/flatten`, `/help`).
   - Attached persistent custom reply keyboard (`📊 Status`, `📈 Positions`, `💰 PnL`, `📋 Orders`, `🚨 Flatten`, `ℹ️ Help`) ensuring instant mobile management without SSH.
3. **Emergency Telegram Flatten:**
   - Allows operator to trip the equity drawdown circuit breaker directly from Telegram with instant confirmation.

---

## 8. Phase 5 Sign-Off & Incubation Window

Sprints 5.1 through 5.6 are signed off. The Proxmox LXC node executes the autonomous 8-week paper burn-in period with live market streaming, deterministic matching, structural stop floors, interactive Telegram bot management, and real-time monitoring.
