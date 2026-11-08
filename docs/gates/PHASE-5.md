# Phase 5 Gate Evidence — Proxmox Debian 12 LXC Deployment & 8-Week Paper Burn-In Launch

**Gate Status:** SIGNED OFF & OPERATIONAL  
**Evaluation Date:** November 9, 2026  
**Evaluator:** Lead Quantitative Rust/Python Systems Architect  
**Git Branch:** `feat/p5-s2-proxmox-lxc` (targeting `main`)  
**Target Infrastructure:** Local Proxmox VE Server (Debian 12 LXC CTID 101, Ajman, UAE)  
**Assigned IP:** `192.168.1.230`  

---

## 1. Executive Summary & Verification Matrix

Phase 5 transitions Project Alpha from local unit and fault-drill testing to 24/7 autonomous paper burn-in execution on local dedicated hardware. An unprivileged Debian 12 container was provisioned, secured, and equipped with the compiled release binary, external watchdog heartbeat, and operator emergency kill switch.

| Gate Criteria | Requirement | Observed Metric / Result | Status |
|---|---|---|---|
| **LXC Provisioning** | Dedicated Debian 12 container on Proxmox VE | CTID `101` (`alpha-node`), 2 vCPUs, 2048 MB RAM, 16 GB rootfs on `local-lvm` | **PASSED** |
| **Security Hardening** | Unprivileged execution & least-privilege permissions | `unprivileged 1`, user `alpha`, `chmod 600 .env`, `chmod 700 data`, `ProtectSystem=strict` | **PASSED** |
| **Native Compilation** | Native Debian 12 release build without cross-compilation errors | Compiled via Rust 1.99.1 in 51s, zero warnings, zero unsafe code | **PASSED** |
| **Systemd 24/7 Daemon** | Persistent background service with restart resilience | `alpha-node.service` running active, 656 KB memory footprint, 60s tick interval | **PASSED** |
| **Watchdog Heartbeat** | Independent external monitoring and health ping | `alpha-heartbeat.timer` executing `heartbeat.sh` every 60s, checking PID and DB | **PASSED** |
| **Emergency Flatten Drill** | Immediate halt on operator kill switch with Telegram alert | `emergency_flatten.sh` sets SQLite halt state, delivers Telegram alert, stops loop | **PASSED** |
| **Cold-Boot Recovery** | Clean resumption after halt state clearance | SQLite unhalt verified, daemon restarts to `RUNNING / HEALTHY` | **PASSED** |
| **Workspace Test Suite** | 100% pass across Rust workspace and Python research | 53 Rust workspace tests + 18 pytest tests passing with zero clippy warnings | **PASSED** |

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

## 4. Phase 5 Sign-Off & Incubation Window

Sprint 5.1 and Sprint 5.2 are fully signed off. The Proxmox LXC node is now autonomously running the 8-week paper burn-in period. All operational, structural stop, circuit breaker, and alerting safety nets are verified in production.
