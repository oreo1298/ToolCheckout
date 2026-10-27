#!/usr/bin/env bash
# ToolCheckout - uninstaller.
#
#   ./uninstall.sh              remove ToolCheckout (finds the installation automatically)
#   ./uninstall.sh --yes        without asking
#
# A copy of this script is installed as <prefix>/lib/toolcheckout/uninstall.sh, so you can uninstall
# even after deleting the downloaded folder:  sudo /usr/local/lib/toolcheckout/uninstall.sh
#
# Your shop data (tools, people, history, backups) is never removed. It lives in
# ~/.local/share/toolcheckout, or in the folder chosen in Settings → Data & backups.
set -euo pipefail

PKG_NAME="toolcheckout"
DISTRO_PKG="tool-checkout"
SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX=""
ASSUME_YES=0

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    B=$'\e[1m'; D=$'\e[2m'; R=$'\e[0m'; RED=$'\e[91m'; GRN=$'\e[92m'; YEL=$'\e[93m'; CYN=$'\e[96m'
else
    B=""; D=""; R=""; RED=""; GRN=""; YEL=""; CYN=""
fi
info() { printf '  %s\n' "$*"; }
ok()   { printf '  %s✔%s %s\n' "$GRN" "$R" "$*"; }
warn() { printf '  %s▲ %s%s\n' "$YEL" "$*" "$R"; }
fail() { printf '\n%s✘ %s%s\n' "$RED$B" "$*" "$R" >&2; exit 1; }
run()  { printf '  %s$ %s%s\n' "$D" "$*" "$R"; "$@"; }
have() { command -v "$1" >/dev/null 2>&1; }

SUDO=()
if [[ $EUID -ne 0 ]]; then
    if have sudo; then SUDO=(sudo)
    elif have doas; then SUDO=(doas)
    fi
fi
root_quiet() { if [[ ${#SUDO[@]} -gt 0 ]]; then "${SUDO[@]}" "$@"; else "$@"; fi; }
AS_USER=0
inst() { if [[ $AS_USER -eq 1 ]]; then "$@"; else root_quiet "$@"; fi; }

confirm() {
    [[ $ASSUME_YES -eq 1 || ! -t 0 ]] && return 0
    local reply
    read -r -p "  $1 [y/N] " reply || true
    [[ "$reply" =~ ^[Yy] ]]
}

data_note() {
    info "Your shop data was kept. It is in ~/.local/share/toolcheckout, or in the folder chosen in"
    info "Settings → Data & backups. Delete it yourself only if you are sure you no longer need it."
}

remove_pacman_package() {
    info "Found the ${B}$(pacman -Q "$PKG_NAME")${R} pacman package."
    confirm "Remove it?" || { info "Nothing was changed."; exit 0; }
    if [[ $EUID -eq 0 ]]; then run pacman -R --noconfirm "$PKG_NAME"
    else run "${SUDO[@]}" pacman -R --noconfirm "$PKG_NAME"
    fi
    printf '\n%s✔ ToolCheckout has been removed.%s\n\n' "$GRN$B" "$R"
    data_note
    echo
}

main() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --prefix)   [[ $# -ge 2 ]] || fail "--prefix needs a directory"; PREFIX="${2%/}"; shift ;;
            --prefix=*) PREFIX="${1#*=}"; PREFIX="${PREFIX%/}" ;;
            -y|--yes)   ASSUME_YES=1 ;;
            -h|--help)
                cat <<EOF
ToolCheckout uninstaller

Usage: ./uninstall.sh [--prefix DIR] [--yes]

  --prefix DIR   where ToolCheckout was installed (found automatically)
  -y, --yes      do not ask

Your shop data is never removed.
EOF
                exit 0 ;;
            *) fail "Unknown option: $1 (see --help)" ;;
        esac
        shift
    done

    printf '%sToolCheckout%s uninstaller\n\n' "$B$CYN" "$R"

    # ---- find the installation ----------------------------------------------------------------------
    local manifest="" m
    local candidates=()
    [[ -n "$PREFIX" ]] && candidates+=("$PREFIX/lib/toolcheckout/install-manifest.txt")
    candidates+=("$SELF_DIR/install-manifest.txt" /usr/local/lib/toolcheckout/install-manifest.txt
                 "$HOME/.local/lib/toolcheckout/install-manifest.txt" /usr/lib/toolcheckout/install-manifest.txt)
    for m in "${candidates[@]}"; do
        if [[ -f "$m" ]]; then manifest="$m"; break; fi
    done

    if [[ -z "$manifest" ]]; then
        if [[ -z "$PREFIX" ]] && have pacman && pacman -Q "$PKG_NAME" >/dev/null 2>&1; then
            remove_pacman_package
            return
        fi
        if have dpkg && dpkg -s "$DISTRO_PKG" >/dev/null 2>&1; then
            fail "ToolCheckout was installed as a .deb package. Remove it with:  sudo apt remove $DISTRO_PKG"
        elif have rpm && rpm -q "$DISTRO_PKG" >/dev/null 2>&1; then
            fail "ToolCheckout was installed as an RPM package. Remove it with:  sudo dnf remove $DISTRO_PKG  (openSUSE: sudo zypper remove $DISTRO_PKG)"
        fi
        fail "No installation of ToolCheckout was found${PREFIX:+ in $PREFIX}.
  If you installed it somewhere else, pass the location:  ./uninstall.sh --prefix /path"
    fi

    local header
    header="$(sed -n '2p' "$manifest")"
    if [[ -z "$PREFIX" ]]; then
        PREFIX="$(sed -n 's/.*prefix: //p' <<<"$header")"
        [[ -n "$PREFIX" ]] || PREFIX="$(dirname "$(dirname "$(dirname "$manifest")")")"
    fi
    [[ "$PREFIX" == /* && "$PREFIX" != "/" ]] || fail "Refusing to uninstall from '$PREFIX'."

    local files=()
    mapfile -t files < <(grep -v '^#' "$manifest" | grep -v '^$')
    info "Found ToolCheckout in ${B}$PREFIX${R} (${#files[@]} files)."
    info "${D}${header#\# }${R}"

    if [[ $EUID -ne 0 && -w "$(dirname "$manifest")" && -w "$PREFIX/bin" ]]; then
        AS_USER=1                       # e.g. installed with --user into ~/.local
    fi
    if [[ $EUID -ne 0 && ${#SUDO[@]} -eq 0 && $AS_USER -eq 0 ]]; then
        fail "Administrator rights are needed. Run:  su -c '$0 --prefix $PREFIX'"
    fi

    confirm "Remove ToolCheckout from $PREFIX?" || { info "Nothing was changed."; exit 0; }

    # ---- remove ----------------------------------------------------------------------------------------
    local removed=0 f
    for f in "${files[@]}"; do
        case "$f" in
            "$PREFIX"/*) ;;
            *) warn "Skipping $f (outside $PREFIX)"; continue ;;
        esac
        if [[ -f "$f" || -L "$f" ]]; then
            inst rm -f -- "$f"
            removed=$((removed + 1))
        fi
    done
    [[ -d "$PREFIX/lib/toolcheckout" ]] && inst find "$PREFIX/lib/toolcheckout" -depth -type d -empty -delete
    ok "Removed $removed files."

    if have gtk-update-icon-cache && [[ -f "$PREFIX/share/icons/hicolor/icon-theme.cache" ]]; then
        inst gtk-update-icon-cache -q -t -f "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
    fi
    if have update-desktop-database && [[ -d "$PREFIX/share/applications" ]]; then
        inst update-desktop-database -q "$PREFIX/share/applications" >/dev/null 2>&1 || true
    fi

    printf '\n%s✔ ToolCheckout has been removed.%s\n\n' "$GRN$B" "$R"
    data_note
    if have pacman && pacman -Q "$PKG_NAME" >/dev/null 2>&1; then
        warn "The $PKG_NAME pacman package is still installed. Remove it with: sudo pacman -R $PKG_NAME"
    fi
    echo
}

# Runs inside a function: the installed copy of this script deletes itself.
main "$@"; exit $?
