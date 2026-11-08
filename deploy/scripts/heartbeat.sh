#!/usr/bin/env bash
# Project Alpha — 60s Watchdog Heartbeat for Proxmox Debian 12 LXC
set -euo pipefail

ENV_FILE="/opt/alpha/.env"
if [[ -f "$ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$ENV_FILE"
fi

DB_PATH="/opt/alpha/data/alpha_state.db"
NODE_BIN="/opt/alpha/bin/alpha-node"

send_telegram_alert() {
    local message="$1"
    if [[ -n "${TG_TOKEN:-}" && -n "${TG_CHAT_ID:-}" ]]; then
        curl -s -X POST "https://api.telegram.org/bot${TG_TOKEN}/sendMessage" \
            -H "Content-Type: application/json" \
            -d "{\"chat_id\": \"${TG_CHAT_ID}\", \"text\": \"🚨 Alpha Alert: ${message}\"}" >/dev/null || true
    fi
}

# 1. Check if alpha-node service or process is active
if ! pgrep -f "alpha-node" >/dev/null; then
    send_telegram_alert "CRITICAL: alpha-node process is DOWN on Proxmox LXC!"
    if [[ -n "${HEALTHCHECKS_URL:-}" && "$HEALTHCHECKS_URL" != *"your-uuid-here"* ]]; then
        curl -fsS --retry 3 "${HEALTHCHECKS_URL}/fail" >/dev/null || true
    fi
    exit 1
fi

# 2. Check engine state via alpha-node status if binary & db exist
if [[ -x "$NODE_BIN" && -f "$DB_PATH" ]]; then
    STATUS_OUT=$("$NODE_BIN" status "$DB_PATH" 2>&1 || true)
    if echo "$STATUS_OUT" | grep -q "HALTED"; then
        send_telegram_alert "WARNING: alpha-node is in HALTED state! Check circuit breaker or operator flatten."
        if [[ -n "${HEALTHCHECKS_URL:-}" && "$HEALTHCHECKS_URL" != *"your-uuid-here"* ]]; then
            curl -fsS --retry 3 "${HEALTHCHECKS_URL}/fail" >/dev/null || true
        fi
        exit 1
    fi
fi

# 3. Healthy: ping healthchecks.io
if [[ -n "${HEALTHCHECKS_URL:-}" && "$HEALTHCHECKS_URL" != *"your-uuid-here"* ]]; then
    curl -fsS -m 10 --retry 3 "$HEALTHCHECKS_URL" >/dev/null || true
fi

exit 0
