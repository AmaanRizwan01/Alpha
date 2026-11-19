#!/usr/bin/env bash
# Project Alpha — Automated Debian 12 LXC Provisioning Script
set -euo pipefail

echo "=== Initializing Project Alpha LXC Setup ==="

# 1. Require root
if [[ $EUID -ne 0 ]]; then
   echo "Error: this setup script must be executed as root inside the LXC container."
   exit 1
fi

# 2. System dependencies
echo "Updating apt and installing dependencies..."
apt-get update -y
apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    sqlite3 \
    libsqlite3-dev \
    build-essential \
    pkg-config

# 3. Create dedicated unprivileged alpha system user
if ! id -u alpha >/dev/null 2>&1; then
    echo "Creating system user 'alpha'..."
    useradd -r -s /bin/bash -d /opt/alpha -m alpha
fi

# 4. Directory structure
echo "Configuring /opt/alpha hierarchy..."
mkdir -p /opt/alpha/bin
mkdir -p /opt/alpha/configs
mkdir -p /opt/alpha/data
mkdir -p /opt/alpha/scripts

# 5. Copy scripts if executing from cloned repo
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

if [[ -f "$SCRIPT_DIR/scripts/heartbeat.sh" ]]; then
    cp "$SCRIPT_DIR/scripts/heartbeat.sh" /opt/alpha/scripts/
    cp "$SCRIPT_DIR/scripts/emergency_flatten.sh" /opt/alpha/scripts/
    chmod +x /opt/alpha/scripts/*.sh
fi

if [[ -f "$REPO_DIR/configs/paper.toml" ]]; then
    cp "$REPO_DIR/configs/paper.toml" /opt/alpha/configs/
fi

if [[ -f "$REPO_DIR/configs/calendar_blackouts.json" ]]; then
    cp "$REPO_DIR/configs/calendar_blackouts.json" /opt/alpha/configs/
fi

# 6. Copy systemd units
if [[ -d "$SCRIPT_DIR/systemd" ]]; then
    echo "Installing systemd units..."
    cp "$SCRIPT_DIR/systemd/"* /etc/systemd/system/
    systemctl daemon-reload
    systemctl enable alpha-heartbeat.timer
fi

# 7. Restrict permissions
chown -R alpha:alpha /opt/alpha
chmod 700 /opt/alpha/data
chmod 750 /opt/alpha/scripts

echo "=== LXC Provisioning Complete ==="
echo "Next steps on container:"
echo "1. Place compiled binary at /opt/alpha/bin/alpha-node (chmod +x)"
echo "2. Populate /opt/alpha/.env (chmod 600)"
echo "3. Run: systemctl start alpha-heartbeat.timer"
echo "4. Run: systemctl start alpha-node.service"
