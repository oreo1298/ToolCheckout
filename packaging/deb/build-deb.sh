#!/usr/bin/env bash
# Build a .deb package for Debian, Ubuntu, Linux Mint, Pop!_OS and friends.
#
#   ./install.sh --deps-only                       once: build dependencies, Rust and Node.js
#   ./packaging/deb/build-deb.sh
#   sudo apt install ./ToolCheckout_*_amd64.deb    install / update
#   sudo apt remove tool-checkout                  uninstall (your shop data is kept)
#
# This is the same package as the .deb on the Releases page.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# Rust and Node.js installed by install.sh (rustup / nvm) are not always on PATH yet.
# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"
if ! command -v npm >/dev/null 2>&1 && [[ -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]]; then
    set +eu
    # shellcheck disable=SC1091
    . "${NVM_DIR:-$HOME/.nvm}/nvm.sh"
    set -eu
fi
for tool in cargo npm dpkg; do
    command -v "$tool" >/dev/null 2>&1 || { echo "error: $tool is missing. Run ./install.sh --deps-only first." >&2; exit 1; }
done

cd "$ROOT"
npm ci --no-audit --no-fund
npm run tauri -- build --bundles deb
rm -f "$ROOT"/ToolCheckout_*.deb
cp target/release/bundle/deb/ToolCheckout_*.deb "$ROOT/"
OUT="$(ls "$ROOT"/ToolCheckout_*.deb)"
echo
echo "Built $OUT"
echo "Install it with:  sudo apt install $OUT"
