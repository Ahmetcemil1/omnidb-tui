#!/bin/bash
# 🚀 OmniDB TUI — Launch & Play Demo in Alacritty Terminal

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"

echo "============================================================"
echo " 🚀 Launching OmniDB TUI in Alacritty Terminal"
echo "============================================================"
echo ""
echo "Select Option:"
echo " 1) Play Recorded Terminal Walkthrough (demo.cast)"
echo " 2) Run Live OmniDB TUI in Alacritty Terminal"
echo " 3) Run Full System Test Suite (cargo test)"
echo ""
read -p "Enter choice [1-3]: " choice

case $choice in
    1)
        alacritty --title "OmniDB TUI — Recorded Demo Video" -e asciinema play demo.cast
        ;;
    2)
        alacritty --title "OmniDB TUI Workspace" -e ./target/release/omnidb-tui
        ;;
    3)
        alacritty --title "OmniDB TUI Test Suite" -e cargo test -- --nocapture
        ;;
    *)
        echo "Invalid option. Running live app by default..."
        alacritty --title "OmniDB TUI Workspace" -e ./target/release/omnidb-tui
        ;;
esac
