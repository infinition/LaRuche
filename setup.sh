#!/usr/bin/env bash
# =============================================================
#  LaRuche - Quick Setup Script
# =============================================================
set -euo pipefail

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${YELLOW}"
echo "  ██╗      █████╗ ██████╗ ██╗   ██╗ ██████╗██╗  ██╗███████╗"
echo "  ██║     ██╔══██╗██╔══██╗██║   ██║██╔════╝██║  ██║██╔════╝"
echo "  ██║     ███████║██████╔╝██║   ██║██║     ███████║█████╗  "
echo "  ██║     ██╔══██║██╔══██╗██║   ██║██║     ██╔══██║██╔══╝  "
echo "  ███████╗██║  ██║██║  ██║╚██████╔╝╚██████╗██║  ██║███████╗"
echo "  ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝"
echo -e "${NC}"
echo "  Quick Setup - LAND Protocol v0.1.0"
echo ""

# Check Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED} Rust not found.${NC}"
    echo "   Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
echo -e "${GREEN} Rust $(rustc --version | cut -d' ' -f2)${NC}"

# Check Ollama
if ! command -v ollama &> /dev/null; then
    echo -e "${YELLOW}  Ollama not found (optional for inference).${NC}"
    echo "   Install: curl -fsSL https://ollama.com/install.sh | sh"
else
    echo -e "${GREEN} Ollama $(ollama --version 2>/dev/null || echo 'installed')${NC}"

    # Check if a model is available
    if ollama list 2>/dev/null | grep -q "mistral"; then
        echo -e "${GREEN} Model 'mistral' available${NC}"
    else
        echo -e "${YELLOW}  No 'mistral' model found. Pulling...${NC}"
        ollama pull mistral
    fi
fi

echo ""
echo " Building LaRuche workspace..."
cargo build --release 2>&1 | tail -3

echo ""
echo -e "${GREEN} Build complete!${NC}"
echo ""
echo " Quick Start:"
echo ""
echo "  1. Start the LaRuche node:"
echo "     ${YELLOW}cargo run -p laruche-node --release${NC}"
echo ""
echo "  2. In another terminal, use the CLI:"
echo "     ${YELLOW}cargo run -p laruche-cli --release -- discover${NC}"
echo "     ${YELLOW}cargo run -p laruche-cli --release -- ask \"Bonjour LaRuche !\"${NC}"
echo "     ${YELLOW}cargo run -p laruche-cli --release -- chat${NC}"
echo ""
echo "  3. Open the dashboard:"
echo "     ${YELLOW}cargo run -p laruche-dashboard --release${NC}"
echo "     Then open http://localhost:8420"
echo ""
echo "   Full documentation: README.md"
echo ""
