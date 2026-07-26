#!/usr/bin/env bash
# =============================================================
#  LaRuche - Quick Setup
#  Checks the toolchain, builds the workspace, tells you what to
#  run next. It never downloads a model without asking.
# =============================================================
set -euo pipefail

YELLOW='\033[1;33m'
GREEN='\033[0;32m'
RED='\033[0;31m'
DIM='\033[2m'
NC='\033[0m'

PORT="${LARUCHE_PORT:-8419}"

echo -e "${YELLOW}"
echo "  ██╗      █████╗ ██████╗ ██╗   ██╗ ██████╗██╗  ██╗███████╗"
echo "  ██║     ██╔══██╗██╔══██╗██║   ██║██╔════╝██║  ██║██╔════╝"
echo "  ██║     ███████║██████╔╝██║   ██║██║     ███████║█████╗  "
echo "  ██║     ██╔══██║██╔══██╗██║   ██║██║     ██╔══██║██╔══╝  "
echo "  ███████╗██║  ██║██║  ██║╚██████╔╝╚██████╗██║  ██║███████╗"
echo "  ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝  ╚═════╝╚═╝  ╚═╝╚══════╝"
echo -e "${NC}"
echo "  Quick setup"
echo ""

# --- Rust (required) ----------------------------------------------------
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Rust not found.${NC}"
    echo "  Install: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
echo -e "${GREEN}ok${NC}   Rust $(rustc --version | cut -d' ' -f2)"

# --- Model server (optional) -------------------------------------------
# LaRuche talks to llama.cpp, Ollama, or any OpenAI-compatible endpoint.
# Nothing here is mandatory and nothing is pulled for you: a model can weigh
# several gigabytes, so that choice stays yours.
if command -v ollama &> /dev/null; then
    echo -e "${GREEN}ok${NC}   Ollama detected"
    if ! ollama list 2>/dev/null | grep -q "nomic-embed-text"; then
        echo -e "${DIM}     semantic memory recall wants an embedding model:${NC}"
        echo -e "${DIM}     ollama pull nomic-embed-text${NC}"
    fi
else
    echo -e "${YELLOW}--${NC}   No Ollama on PATH (optional)"
    echo -e "${DIM}     Any llama.cpp server or OpenAI-compatible endpoint works too.${NC}"
    echo -e "${DIM}     Configure it in Settings after first boot, no restart needed.${NC}"
fi

# --- Build --------------------------------------------------------------
echo ""
echo "Building the workspace (first build takes a while)..."
cargo build --release

echo ""
echo -e "${GREEN}Build complete.${NC}"
echo ""
echo "Next:"
echo ""
echo "  1. Start the node. It serves the API and the web UI."
echo -e "     ${YELLOW}cargo run --release -p laruche-node${NC}"
echo ""
echo "  2. Open the dashboard in your browser."
echo -e "     ${YELLOW}http://localhost:${PORT}${NC}"
echo ""
echo "  3. Or drive it from the terminal, in another shell."
echo -e "     ${YELLOW}cargo run --release -p laruche-cli -- chat${NC}"
echo -e "     ${YELLOW}cargo run --release -p laruche-cli -- ask \"Bonjour LaRuche\"${NC}"
echo ""
echo "First boot walks you through model, embeddings and voice, each probed"
echo "for real. Everything else is configured live in Settings."
echo ""
echo "Documentation: README.md and the wiki."
echo ""
