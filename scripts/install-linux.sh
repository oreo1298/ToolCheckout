#!/usr/bin/env bash
# Installs a ToolCheckout you built from source (`npx tauri build --no-bundle`)
# with a menu entry and icon, the same layout the .deb uses.
#
#   scripts/install-linux.sh                     for you only, into ~/.local (no root)
#   sudo scripts/install-linux.sh --system       for all users, into /usr/local
#   scripts/install-linux.sh --uninstall         remove it again (add --system if
#                                                you installed with --system)
#
# Uninstalling removes the program only. Your data stays in
# ~/.local/share/toolcheckout (or the folder chosen in Settings).
set -euo pipefail

prefix="$HOME/.local"
action=install
for arg in "$@"; do
  case "$arg" in
    --system) prefix=/usr/local ;;
    --uninstall) action=uninstall ;;
    -h | --help)
      sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Unknown option: $arg (try --help)" >&2
      exit 2
      ;;
  esac
done

repo="$(cd "$(dirname "$0")/.." && pwd)"
bin="$prefix/bin/ToolCheckout"
desktop="$prefix/share/applications/ToolCheckout.desktop"
icons="$prefix/share/icons/hicolor"
sizes=(32x32 64x64 128x128 256x256 512x512)

refresh_caches() {
  command -v update-desktop-database >/dev/null && update-desktop-database -q "$prefix/share/applications" || true
  command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$icons" || true
}

if [ "$action" = uninstall ]; then
  rm -f "$bin" "$desktop"
  for s in "${sizes[@]}"; do rm -f "$icons/$s/apps/ToolCheckout.png"; done
  refresh_caches
  echo "ToolCheckout was removed from $prefix. Your data was not touched."
  exit 0
fi

built="$repo/target/release/ToolCheckout"
if [ ! -x "$built" ]; then
  echo "No build found at $built" >&2
  echo "Build it first:  npm ci && npx tauri build --no-bundle" >&2
  exit 1
fi

install -Dm755 "$built" "$bin"
src="$repo/src-tauri/icons"
install -Dm644 "$src/32x32.png" "$icons/32x32/apps/ToolCheckout.png"
install -Dm644 "$src/64x64.png" "$icons/64x64/apps/ToolCheckout.png"
install -Dm644 "$src/128x128.png" "$icons/128x128/apps/ToolCheckout.png"
install -Dm644 "$src/128x128@2x.png" "$icons/256x256/apps/ToolCheckout.png"
install -Dm644 "$src/icon.png" "$icons/512x512/apps/ToolCheckout.png"
mkdir -p "$(dirname "$desktop")"
cat >"$desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ToolCheckout
Comment=Tool checkout for a repair shop
Exec=$bin
Icon=ToolCheckout
StartupWMClass=ToolCheckout
Categories=Utility;
Terminal=false
EOF
refresh_caches

echo "Installed ToolCheckout to $bin"
echo "It is in your application menu (you may need to log out and back in once)."
case ":$PATH:" in
  *":$prefix/bin:"*) echo "Start it from a terminal with: ToolCheckout" ;;
  *) echo "To start it from a terminal, run $bin (or add $prefix/bin to your PATH)." ;;
esac
