#!/usr/bin/env bash
# Project Alpha — Latched 60s Watchdog Heartbeat for Proxmox Debian 12 LXC
set -euo pipefail

ENV_FILE="/opt/alpha/.env"
if [[ -f "$ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$ENV_FILE"
fi

DB_PATH="/opt/alpha/data/alpha_state.db"
NODE_BIN="/opt/alpha/bin/alpha-node"
FLAG_FILE="/opt/alpha/data/alert_active.flag"

send_telegram() {
    local text="$1"
    if [[ -n "${TG_TOKEN:-}" && -n "${TG_CHAT_ID:-}" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TG_TOKEN}/sendMessage" \
            -H "Content-Type: application/json" \
            -d "{\"chat_id\": \"${TG_CHAT_ID}\", \"text\": \"${text}\"}" >/dev/null || true
    fi
}

handle_failure() {
    local reason="$1"
    # Send alert only once if not already alerted
    if [[ ! -f "$FLAG_FILE" ]]; then
        send_telegram "🚨 Alpha Alert: ${reason}"
        echo "${reason}" > "$FLAG_FILE"
    fi

    if [[ -n "${HEALTHCHECKS_URL:-}" && "$HEALTHCHECKS_URL" != *"your-uuid-here"* ]]; then
        curl -fsS --retry 3 "${HEALTHCHECKS_URL}/fail" >/dev/null || true
    fi
    exit 1
}

# 1. Check if alpha-node process is running
if ! pgrep -f "alpha-node" >/dev/null; then
    handle_failure "CRITICAL: alpha-node process is DOWN on Proxmox LXC!"
fi

# 2. Check engine state via SQLite
if [[ -x "$NODE_BIN" && -f "$DB_PATH" ]]; then
    STATUS_OUT=$("$NODE_BIN" status "$DB_PATH" 2>&1 || true)
    if echo "$STATUS_OUT" | grep -q "HALTED"; then
        handle_failure "WARNING: alpha-node is in HALTED state! Check circuit breaker or operator flatten."
    fi
fi

# 3. Healthy: check if recovering from previous alert
if [[ -f "$FLAG_FILE" ]]; then
    send_telegram "✅ Alpha Resolved: alpha-node is back UP and RUNNING / HEALTHY on Proxmox LXC."
    rm -f "$FLAG_FILE"
fi

# 4. Silent Healthchecks.io ping
if [[ -n "${HEALTHCHECKS_URL:-}" && "$HEALTHCHECKS_URL" != *"your-uuid-here"* ]]; then
    curl -fsS -m 10 --retry 3 "$HEALTHCHECKS_URL" >/dev/null || true
fi

exit 0
