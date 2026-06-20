#!/bin/bash
# LaRuche Linux Service Installer (systemd)
# Usage: sudo ./install-service.sh [--uninstall]

SERVICE_NAME="laruche"
BINARY_PATH="$(cd "$(dirname "$0")/.." && pwd)/target/release/laruche-node"
WORK_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SERVICE_FILE="/etc/systemd/system/${SERVICE_NAME}.service"

if [ "$1" = "--uninstall" ]; then
    echo "Uninstalling $SERVICE_NAME service..."
    systemctl stop "$SERVICE_NAME" 2>/dev/null
    systemctl disable "$SERVICE_NAME" 2>/dev/null
    rm -f "$SERVICE_FILE"
    systemctl daemon-reload
    echo "Service removed."
    exit 0
fi

# Build release binary
echo "Building release binary..."
cd "$WORK_DIR" && cargo build --release -p laruche-node

if [ ! -f "$BINARY_PATH" ]; then
    echo "ERROR: Binary not found at $BINARY_PATH"
    exit 1
fi

# Create systemd service
cat > "$SERVICE_FILE" << EOF
[Unit]
Description=LaRuche AI Agent - Essaim with Miel Protocol
After=network.target ollama.service
Wants=ollama.service

[Service]
Type=simple
ExecStart=$BINARY_PATH
WorkingDirectory=$WORK_DIR
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl start "$SERVICE_NAME"

echo ""
echo "Service '$SERVICE_NAME' installed and started."
echo "Dashboard: http://localhost:8419/dashboard"
echo "Chatbot:   http://localhost:8419/chat"
echo ""
echo "Status:  systemctl status $SERVICE_NAME"
echo "Logs:    journalctl -u $SERVICE_NAME -f"
echo "Remove:  sudo $0 --uninstall"
