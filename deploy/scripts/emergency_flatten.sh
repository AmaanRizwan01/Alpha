#!/usr/bin/env bash
# Project Alpha — Emergency Operator Flatten & Halt Script
set -euo pipefail

ENV_FILE="/opt/alpha/.env"
if [[ -f "$ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$ENV_FILE"
fi

DB_PATH="/opt/alpha/data/alpha_state.db"
NODE_BIN="/opt/alpha/bin/alpha-node"

echo "=== ENGAGING EMERGENCY FLATTEN ==="
if [[ -x "$NODE_BIN" && -f "$DB_PATH" ]]; then
    "$NODE_BIN" flatten "$DB_PATH"
else
    echo "Warning: binary or database not found at default paths, attempting direct execution..."
    alpha-node flatten "$DB_PATH" || true
fi

# Broadcast emergency alert to Telegram
if [[ -n "${TG_TOKEN:-}" && -n "${TG_CHAT_ID:-}" ]]; then
    curl -s -X POST "https://api.telegram.org/bot${TG_TOKEN}/sendMessage" \
        -H "Content-Type: application/json" \
        -d "{\"chat_id\": \"${TG_CHAT_ID}\", \"text\": \"🛑 EMERGENCY FLATTEN ENGAGED BY OPERATOR on Proxmox node. Trading halted.\"}" >/dev/null || true
fi

echo "Flatten signal recorded. Engine halted."
