#!/usr/bin/env bash
# Build an RPM package for Fedora, RHEL/AlmaLinux/Rocky, openSUSE and friends.
#
#   ./install.sh --deps-only                       once: build dependencies, Rust and Node.js
#   ./packaging/rpm/build-rpm.sh
#   sudo dnf install ./ToolCheckout-*.rpm          install / update
#        (openSUSE: sudo zypper install --allow-unsigned-rpm ./ToolCheckout-*.rpm)
#   sudo dnf remove tool-checkout                  uninstall (your shop data is kept)
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
for tool in cargo npm; do
    command -v "$tool" >/dev/null 2>&1 || { echo "error: $tool is missing. Run ./install.sh --deps-only first." >&2; exit 1; }
done

cd "$ROOT"
npm ci --no-audit --no-fund
npm run tauri -- build --bundles rpm
rm -f "$ROOT"/ToolCheckout-*.rpm
cp target/release/bundle/rpm/ToolCheckout-*.rpm "$ROOT/"
OUT="$(ls "$ROOT"/ToolCheckout-*.rpm)"
echo
echo "Built $OUT"
if command -v zypper >/dev/null 2>&1; then
    echo "Install it with:  sudo zypper install --allow-unsigned-rpm $OUT"
else
    echo "Install it with:  sudo dnf install $OUT"
fi
