#!/usr/bin/env bash
# Verifies that ToolCheckout follows the OS light/dark preference LIVE on Linux,
# using the real xdg-desktop-portal with the GNOME and KDE backends, inside a
# headless X server and a private D-Bus session (no real desktop needed).
#
# Ubuntu 24.04 prerequisites:
#   sudo apt-get install xvfb xdotool imagemagick x11-utils dbus-daemon \
#     xdg-desktop-portal xdg-desktop-portal-gnome xdg-desktop-portal-gtk \
#     dconf-cli gsettings-desktop-schemas \
#     xdg-desktop-portal-kde plasma-integration libkf5config-bin breeze
#
# Usage: scripts/linux-theme-check.sh path/to/ToolCheckout [outdir]
# Exit code 0 when both desktops switch light → dark → light without restart.
set -euo pipefail

APP=${1:?path to the ToolCheckout binary}
OUT=${2:-$(mktemp -d)}
mkdir -p "$OUT"

export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-$(mktemp -d)}
chmod 700 "$XDG_RUNTIME_DIR"
export DISPLAY=:97
Xvfb :97 -screen 0 1400x900x24 -nolisten tcp >/dev/null 2>&1 &
XVFB=$!
eval "$(dbus-launch --sh-syntax)"
trap 'kill $XVFB $DBUS_SESSION_BUS_PID 2>/dev/null || true' EXIT
for _ in $(seq 1 50); do xdpyinfo >/dev/null 2>&1 && break; sleep 0.1; done
export WEBKIT_DISABLE_DMABUF_RENDERER=1 NO_AT_BRIDGE=1

# A real desktop ships these (gnome-session / plasma-workspace).
mkdir -p "$HOME/.config/xdg-desktop-portal"
printf '[preferred]\ndefault=gnome;gtk;\n' >"$HOME/.config/xdg-desktop-portal/gnome-portals.conf"
printf '[preferred]\ndefault=kde;\n' >"$HOME/.config/xdg-desktop-portal/kde-portals.conf"

bg_pixel() { convert "$1" -format '%[pixel:p{700,500}]' info:; }
read_portal() {
  gdbus call --session --dest org.freedesktop.portal.Desktop \
    --object-path /org/freedesktop/portal/desktop \
    --method org.freedesktop.portal.Settings.ReadOne org.freedesktop.appearance color-scheme
}
start_app() {
  "$APP" >"$OUT/app-$1.log" 2>&1 &
  APP_PID=$!
  for _ in $(seq 1 60); do xdotool search --name ToolCheckout >/dev/null 2>&1 && break; sleep 0.5; done
  sleep 4
}
stop_all() {
  kill "$APP_PID" 2>/dev/null || true
  pkill -x xdg-desktop-por 2>/dev/null || true
  sleep 1
}
check_cycle() { # name set_light set_dark
  local name=$1 light=$2 dark=$3 a b c
  $light
  read_portal
  start_app "$name"
  import -window root "$OUT/$name-1-light.png"
  $dark; sleep 2
  import -window root "$OUT/$name-2-dark.png"
  $light; sleep 2
  import -window root "$OUT/$name-3-light.png"
  a=$(bg_pixel "$OUT/$name-1-light.png") b=$(bg_pixel "$OUT/$name-2-dark.png") c=$(bg_pixel "$OUT/$name-3-light.png")
  echo "$name: light=$a dark=$b light-again=$c"
  stop_all
  [[ $a == "srgb(243,244,246)" && $b == "srgb(15,17,21)" && $c == "srgb(243,244,246)" ]]
}

# --- GNOME: the libadwaita "Dark style" toggle writes this GSettings key.
export XDG_CURRENT_DESKTOP=GNOME
dbus-update-activation-environment XDG_CURRENT_DESKTOP=GNOME DISPLAY="$DISPLAY"
gnome_light() { gsettings set org.gnome.desktop.interface color-scheme default; }
gnome_dark() { gsettings set org.gnome.desktop.interface color-scheme prefer-dark; }
check_cycle gnome gnome_light gnome_dark

# --- KDE Plasma: what the Colors KCM does when applying a color scheme.
export XDG_CURRENT_DESKTOP=KDE KDE_SESSION_VERSION=5 QT_QPA_PLATFORM=xcb
dbus-update-activation-environment XDG_CURRENT_DESKTOP=KDE KDE_SESSION_VERSION=5 \
  QT_QPA_PLATFORM=xcb QT_QPA_PLATFORMTHEME=kde DISPLAY="$DISPLAY"
kde_apply() {
  local src=/usr/share/color-schemes/$1.colors grp="" line
  while IFS= read -r line; do
    case "$line" in
      \[*\]) grp="${line#[}" grp="${grp%]}" ;;
      *=*) [[ $grp == Colors:* ]] && kwriteconfig5 --file kdeglobals --group "$grp" --key "${line%%=*}" "${line#*=}" ;;
    esac
  done <"$src"
  kwriteconfig5 --file kdeglobals --group General --key ColorScheme "$1"
  gdbus emit --session --object-path /KGlobalSettings --signal org.kde.KGlobalSettings.notifyChange 0 0 >/dev/null
}
kde_light() { kde_apply BreezeLight; }
kde_dark() { kde_apply BreezeDark; }
check_cycle kde kde_light kde_dark

echo "OK: live theme switching verified on GNOME and KDE portals. Screenshots in $OUT"
