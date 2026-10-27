#!/usr/bin/env bash
# ToolCheckout - installer for every common Linux distribution.
#
#   ./install.sh              install for all users into /usr/local (asks for your password when needed)
#   ./install.sh --user       install just for you into ~/.local
#   ./install.sh --update     download the latest version and reinstall it the same way
#   ./install.sh --uninstall  remove ToolCheckout again (your shop data is always kept)
#   ./install.sh --help       all options
#
# On Arch Linux and derivatives it builds a regular pacman package (packaging/arch/PKGBUILD), so
# pacman keeps track of every file. Elsewhere it builds with cargo and npm and installs with make.
set -euo pipefail

APP="ToolCheckout"
PKG_NAME="toolcheckout"        # the pacman package built from packaging/arch/PKGBUILD
DISTRO_PKG="tool-checkout"     # the .deb/.rpm from the Releases page or packaging/{deb,rpm}
REPO_URL="https://github.com/oreo1298/ToolCheckout"
MIN_RUST="1.90"
NODE_MAJOR="22"
NVM_VERSION="v0.40.3"
NVM_DIR="${NVM_DIR:-$HOME/.nvm}"

SRC_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Not called PREFIX: nvm (sourced into this shell) refuses to work when a PREFIX variable is set.
INSTALL_PREFIX=""
USER_INSTALL=0
INSTALL_DEPS=1
DEPS_ONLY=0
USE_PKG="auto"
ASSUME_YES=0
MODE="install"
STAGE=""

# ---- output helpers --------------------------------------------------------------------------------

if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    B=$'\e[1m'; D=$'\e[2m'; R=$'\e[0m'; RED=$'\e[91m'; GRN=$'\e[92m'; YEL=$'\e[93m'; CYN=$'\e[96m'
else
    B=""; D=""; R=""; RED=""; GRN=""; YEL=""; CYN=""
fi
STEP=0
STEPS=6
step()  { STEP=$((STEP + 1)); printf '\n%s[%d/%d] %s%s\n' "$B$CYN" "$STEP" "$STEPS" "$1" "$R"; }
info()  { printf '  %s\n' "$*"; }
ok()    { printf '  %s✔%s %s\n' "$GRN" "$R" "$*"; }
warn()  { printf '  %s▲ %s%s\n' "$YEL" "$*" "$R"; }
fail()  { printf '\n%s✘ %s%s\n' "$RED$B" "$*" "$R" >&2; exit 1; }
run()   { printf '  %s$ %s%s\n' "$D" "$*" "$R"; "$@"; }
have()  { command -v "$1" >/dev/null 2>&1; }

usage() {
    cat <<EOF
ToolCheckout installer

Usage: ./install.sh [options]

  --user           install just for you into ~/.local (no password needed for the install step)
  --prefix DIR     install location (default: /usr/local, or ~/.local with --user)
  --update         download the latest version and reinstall it the same way (same as ./update.sh)
  --uninstall      remove ToolCheckout (same as ./uninstall.sh); your shop data is kept
  --no-deps        do not install dependencies with the package manager
  --deps-only      only install the dependencies (e.g. before packaging/deb/build-deb.sh)
  --no-package     on Arch-based distributions, install with make instead of a pacman package
  -y, --yes        do not ask questions
  -h, --help       show this help
EOF
}

# ---- privileges ------------------------------------------------------------------------------------

SUDO=()
if [[ $EUID -ne 0 ]]; then
    if have sudo; then SUDO=(sudo)
    elif have doas; then SUDO=(doas)
    fi
fi
as_root() {
    if [[ $EUID -eq 0 ]]; then run "$@"
    elif [[ ${#SUDO[@]} -gt 0 ]]; then run "${SUDO[@]}" "$@"
    else fail "This step needs administrator rights, but neither sudo nor doas is installed."
    fi
}
root_quiet() { if [[ ${#SUDO[@]} -gt 0 ]]; then "${SUDO[@]}" "$@"; else "$@"; fi; }
INSTALL_AS_USER=0
inst() { if [[ $INSTALL_AS_USER -eq 1 ]]; then "$@"; else root_quiet "$@"; fi; }

confirm() {
    [[ $ASSUME_YES -eq 1 || ! -t 0 ]] && return 0
    local reply
    read -r -p "  $1 [Y/n] " reply || true
    [[ -z "$reply" || "$reply" =~ ^[Yy] ]]
}

# version_ge A B: true if version A >= version B.
version_ge() { [[ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n1)" == "$2" ]]; }

# ---- distribution ------------------------------------------------------------------------------------

DISTRO_NAME="Linux"
DISTRO_ID=""
DISTRO_VERSION=""
FAMILY="unknown"
detect_distro() {
    local ids="" x
    if [[ -r /etc/os-release ]]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        DISTRO_NAME="${PRETTY_NAME:-${NAME:-Linux}}"
        DISTRO_ID="${ID:-}"
        DISTRO_VERSION="${VERSION_ID:-}"
        ids="${ID:-} ${ID_LIKE:-}"
    fi
    for x in $ids; do
        case "$x" in
            arch|archarm|manjaro|endeavouros|garuda|artix|cachyos|arcolinux|steamos) FAMILY=arch; return ;;
            debian|ubuntu|linuxmint|pop|elementary|zorin|neon|kali|raspbian|deepin|mx|devuan|pureos) FAMILY=debian; return ;;
            rhel|centos|rocky|almalinux|ol|circle) FAMILY=rhel; return ;;
            fedora|nobara|ultramarine) FAMILY=fedora; return ;;
            opensuse*|suse|sles|sled) FAMILY=suse; return ;;
            void) FAMILY=void; return ;;
            gentoo|funtoo) FAMILY=gentoo; return ;;
            nixos) FAMILY=nixos; return ;;
        esac
    done
}

# ---- Node.js and Rust ----------------------------------------------------------------------------------

# Vite needs Node.js 20.19+ or 22.12+.
node_version_ok() {
    local v="${1#v}"
    version_ge "$v" 22.12.0 && return 0
    [[ "${v%%.*}" == 20 ]] && version_ge "$v" 20.19.0
}
node_ok() { have node && have npm && node_version_ok "$(node --version 2>/dev/null)"; }

load_nvm() {
    [[ -s "$NVM_DIR/nvm.sh" ]] || return 1
    # nvm.sh is not written for "set -eu".
    set +eu
    # shellcheck disable=SC1091
    . "$NVM_DIR/nvm.sh" >/dev/null 2>&1
    set -eu
}

cargo_version() { cargo --version 2>/dev/null | awk '{print $2}'; }
rust_ok() { have cargo && version_ge "$(cargo_version)" "$MIN_RUST"; }
load_cargo_env() {
    # shellcheck disable=SC1091
    if [[ -f "$HOME/.cargo/env" ]]; then . "$HOME/.cargo/env"; fi
}

# ---- packages --------------------------------------------------------------------------------------------

PM=""
PKGS=()
PRE_NOTE=""
pick_packages() {
    local arch_pkg=$1
    case "$FAMILY" in
        arch)
            PM=pacman
            if [[ $arch_pkg -eq 1 ]]; then
                PKGS=(base-devel git)      # makepkg -s installs the rest from the PKGBUILD
            else
                PKGS=(base-devel git curl webkit2gtk-4.1 nodejs npm)
                have cargo || PKGS+=(rust)
            fi ;;
        debian)
            PM=apt
            PKGS=(build-essential pkg-config curl git libwebkit2gtk-4.1-dev libdbus-1-dev) ;;
        fedora)
            PM=dnf
            PKGS=(gcc make pkgconf-pkg-config curl git webkit2gtk4.1-devel dbus-devel)
            node_ok || PKGS+=(nodejs npm) ;;
        rhel)
            PM=dnf
            PKGS=(gcc make pkgconf-pkg-config curl git webkit2gtk4.1-devel dbus-devel)
            node_ok || PKGS+=(nodejs npm)
            PRE_NOTE="WebKitGTK 4.1 comes from EPEL, which also needs the CodeReady Builder (CRB) repository. Both will be enabled." ;;
        suse)
            PM=zypper
            # Asked for by what they provide: openSUSE renamed the WebKitGTK packages between releases.
            PKGS=(gcc make pkg-config curl git 'pkgconfig(webkit2gtk-4.1)' 'pkgconfig(dbus-1)')
            if ! node_ok && [[ "$DISTRO_ID" == opensuse-tumbleweed || "$DISTRO_ID" == opensuse-slowroll ]]; then
                PKGS+=(nodejs-default npm-default)
            fi ;;
        void)
            PM=xbps
            PKGS=(base-devel pkg-config curl git libwebkit2gtk41-devel dbus-devel)
            node_ok || PKGS+=(nodejs) ;;
        gentoo)
            PM=emerge
            PKGS=(net-libs/webkit-gtk:4.1 sys-apps/dbus dev-vcs/git net-misc/curl)
            node_ok || PKGS+=(net-libs/nodejs) ;;
    esac
}

install_packages() {
    case "$PM" in
        pacman)
            as_root pacman -S --needed --noconfirm "${PKGS[@]}" ||
                fail "pacman could not install the packages. If it reported missing files (404), update your system with 'sudo pacman -Syu' and try again." ;;
        apt)
            as_root apt-get update
            # Use the distribution's Node.js only if it is new enough (Debian 13 yes, Ubuntu 24.04 no).
            if ! node_ok; then
                local cand
                cand="$(apt-cache policy nodejs 2>/dev/null | sed -n 's/^ *Candidate: *//p')"
                cand="${cand#*:}"; cand="${cand%%[-+~]*}"
                if [[ -n "$cand" && "$cand" != "(none)" ]] && node_version_ok "$cand"; then
                    PKGS+=(nodejs npm)
                fi
            fi
            as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y "${PKGS[@]}" ;;
        dnf)
            if [[ "$FAMILY" == rhel ]]; then
                local major="${DISTRO_VERSION%%.*}"
                if [[ "$DISTRO_ID" == rhel ]]; then
                    as_root dnf install -y "https://dl.fedoraproject.org/pub/epel/epel-release-latest-$major.noarch.rpm"
                    as_root subscription-manager repos --enable "codeready-builder-for-rhel-$major-$(uname -m)-rpms"
                else
                    as_root dnf install -y dnf-plugins-core epel-release
                    as_root dnf config-manager --set-enabled crb ||
                        as_root dnf config-manager --set-enabled "ol${major}_codeready_builder"
                fi
            fi
            as_root dnf install -y "${PKGS[@]}" ;;
        zypper)
            as_root zypper --non-interactive install "${PKGS[@]}" ;;
        xbps)
            as_root xbps-install -Sy "${PKGS[@]}" ;;
        emerge)
            as_root emerge --noreplace "${PKGS[@]}" ;;
    esac
}

# ---- steps ------------------------------------------------------------------------------------------------

ensure_rust() {
    load_cargo_env
    if rust_ok; then ok "Rust $(cargo_version)"; return; fi
    if have cargo; then
        info "Your Rust ($(cargo_version)) is older than $MIN_RUST, which ToolCheckout needs."
    else
        info "Rust is not installed."
    fi
    if have rustup; then
        run rustup toolchain install stable --profile minimal
        export RUSTUP_TOOLCHAIN=stable
    else
        info "rustup, the official Rust installer, can install it just for you (into ~/.cargo)."
        confirm "Install Rust with rustup?" || fail "Rust $MIN_RUST or newer is needed. Install it, then run ./install.sh again."
        have curl || fail "curl is needed to download rustup."
        printf '  %s$ curl https://sh.rustup.rs | sh -s -- -y --profile minimal%s\n' "$D" "$R"
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
        load_cargo_env
    fi
    rust_ok || fail "Could not get Rust $MIN_RUST or newer."
    ok "Rust $(cargo_version)"
}

ensure_node() {
    if ! node_ok; then load_nvm || true; fi
    if node_ok; then ok "Node.js $(node --version)"; return; fi
    if have node; then
        info "Your Node.js ($(node --version)) is too old; ToolCheckout's build tools need 20.19+ or 22.12+."
    else
        info "Node.js is not installed."
    fi
    if ! [[ -s "$NVM_DIR/nvm.sh" ]]; then
        info "nvm can install Node.js $NODE_MAJOR just for you (into $NVM_DIR). It doesn't touch the system's Node.js."
        confirm "Install Node.js $NODE_MAJOR with nvm?" || fail "Node.js 20.19+ or 22.12+ is needed. Install it, then run ./install.sh again."
        have curl || fail "curl is needed to download nvm."
        mkdir -p "$NVM_DIR"
        printf '  %s$ curl https://raw.githubusercontent.com/nvm-sh/nvm/%s/install.sh | bash%s\n' "$D" "$NVM_VERSION" "$R"
        curl -fsSL "https://raw.githubusercontent.com/nvm-sh/nvm/$NVM_VERSION/install.sh" |
            PROFILE=/dev/null NVM_DIR="$NVM_DIR" bash >/dev/null
    fi
    load_nvm || fail "nvm did not install correctly (see the messages above)."
    printf '  %s$ nvm install %s%s\n' "$D" "$NODE_MAJOR" "$R"
    set +eu
    nvm install "$NODE_MAJOR" >/dev/null
    local rc=$?
    set -eu
    [[ $rc -eq 0 ]] && node_ok || fail "Could not install Node.js $NODE_MAJOR with nvm."
    ok "Node.js $(node --version) (from nvm)"
}

check_libs() {
    have pkg-config || fail "pkg-config is missing. Run ./install.sh without --no-deps, or install it yourself."
    local pc
    for pc in webkit2gtk-4.1 dbus-1; do
        pkg-config --exists "$pc" ||
            fail "The development files for $pc were not found. Run ./install.sh without --no-deps, or install them yourself (see the README)."
    done
    ok "WebKitGTK $(pkg-config --modversion webkit2gtk-4.1) and D-Bus development files"
}

git_version() {
    if git -C "$SRC_DIR" rev-parse --git-dir >/dev/null 2>&1; then
        git -C "$SRC_DIR" describe --tags --always --dirty 2>/dev/null || true
    fi
}

# Where is ToolCheckout installed already? Sets INST_KIND (pacman, make, none) and INST_PREFIX.
INST_KIND=none
INST_PREFIX=""
detect_install() {
    if have pacman && pacman -Q "$PKG_NAME" >/dev/null 2>&1; then
        INST_KIND=pacman; INST_PREFIX=/usr; return
    fi
    local p
    for p in ${INSTALL_PREFIX:+"$INSTALL_PREFIX"} /usr/local "$HOME/.local" /usr /opt/toolcheckout; do
        if [[ -f "$p/lib/toolcheckout/install-manifest.txt" ]]; then
            INST_KIND=make; INST_PREFIX="$p"; return
        fi
    done
}

install_with_pacman() {
    step "Building and installing the pacman package"
    have makepkg || fail "makepkg was not found. Install base-devel, or use --no-package."
    info "makepkg builds packaging/arch/PKGBUILD (5 to 15 minutes the first time) and installs the"
    info "package with pacman, which asks for your password."
    local flags=(-s -i -f)
    [[ $ASSUME_YES -eq 1 ]] && flags+=(--noconfirm)
    (cd "$SRC_DIR/packaging/arch" && run makepkg "${flags[@]}") ||
        fail "Building the package failed (see the messages above)."
    pacman -Q "$PKG_NAME" >/dev/null 2>&1 || fail "The $PKG_NAME package was not installed."
    ok "Installed the $(pacman -Q "$PKG_NAME") package."
}

install_with_make() {
    step "Building ToolCheckout"
    info "The first build takes 5 to 15 minutes and about 3 GB of disk space; later builds are faster."
    run make -C "$SRC_DIR" build
    local built="$SRC_DIR/target/release/toolcheckout"
    [[ -x "$built" ]] || fail "The build finished, but $built is missing."
    ok "Built $built"

    step "Checking the build"
    local missing
    missing="$(ldd "$built" | grep 'not found' || true)"
    [[ -z "$missing" ]] || fail "These libraries are missing:
$missing"
    ok "All $(ldd "$built" | wc -l) libraries it needs are installed."

    step "Installing to $INSTALL_PREFIX"
    local stage manifest_tmp count=0 rel old mode
    STAGE="$(mktemp -d)"
    trap 'rm -rf "$STAGE" "$STAGE.manifest"' EXIT
    stage="$STAGE"
    make -C "$SRC_DIR" install PREFIX="$INSTALL_PREFIX" DESTDIR="$stage" BIN_SRC="$built" >/dev/null
    chmod -R u+rwX,go+rX "$stage"

    local manifest="$INSTALL_PREFIX/lib/toolcheckout/install-manifest.txt"
    if [[ -f "$manifest" ]]; then
        # Remove files of the previous version that no longer exist.
        while IFS= read -r old; do
            [[ -z "$old" || "$old" == \#* || -e "$stage$old" ]] && continue
            [[ "$old" == "$INSTALL_PREFIX"/* ]] || continue
            if [[ -e "$old" || -L "$old" ]]; then inst rm -f -- "$old"; fi
        done <"$manifest"
    fi

    manifest_tmp="$stage.manifest"
    {
        echo "# ToolCheckout install manifest - used by uninstall.sh"
        local ver gv
        ver="$(sed -n 's/^  "version": "\(.*\)",$/\1/p' "$SRC_DIR/src-tauri/tauri.conf.json" | head -n1)"
        gv="$(git_version || true)"
        echo "# version: ${ver}${gv:+ (git $gv)} installed: $(date '+%Y-%m-%d %H:%M:%S') prefix: $INSTALL_PREFIX"
        (cd "$stage" && find . -type f | sed 's|^\.||' | sort)
        echo "$manifest"
    } >"$manifest_tmp"

    while IFS= read -r rel; do
        mode="$(stat -c '%a' "$stage$rel")"
        inst install -D -m "$mode" "$stage$rel" "$rel"
        count=$((count + 1))
    done < <(cd "$stage" && find . -type f | sed 's|^\.||' | sort)
    inst install -D -m 644 "$manifest_tmp" "$manifest"
    ok "Installed $count files."

    # Refresh menus and icons. Only an icon cache that already exists is updated: creating one in a
    # folder that never had one (e.g. ~/.local) would hide icons other programs add later.
    if have gtk-update-icon-cache && [[ -f "$INSTALL_PREFIX/share/icons/hicolor/icon-theme.cache" ]]; then
        inst gtk-update-icon-cache -q -t -f "$INSTALL_PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
    fi
    if have update-desktop-database; then
        inst update-desktop-database -q "$INSTALL_PREFIX/share/applications" >/dev/null 2>&1 || true
    fi
}

# ---- install -----------------------------------------------------------------------------------------------

do_install() {
    detect_distro
    if [[ -n "$INSTALL_PREFIX" ]]; then
        [[ "$INSTALL_PREFIX" == /* ]] || fail "--prefix must be an absolute path"
        INSTALL_PREFIX="${INSTALL_PREFIX%/}"
    elif [[ $USER_INSTALL -eq 1 ]]; then
        INSTALL_PREFIX="$HOME/.local"
    else
        INSTALL_PREFIX=/usr/local
    fi
    if [[ $EUID -ne 0 ]] && { [[ "$INSTALL_PREFIX" == "$HOME"/* ]] || [[ -d "$INSTALL_PREFIX" && -w "$INSTALL_PREFIX" ]]; }; then
        INSTALL_AS_USER=1
    fi

    if [[ "$USE_PKG" == auto ]]; then
        if [[ "$FAMILY" == arch && $USER_INSTALL -eq 0 && "$INSTALL_PREFIX" == /usr/local && $EUID -ne 0 && $DEPS_ONLY -eq 0 ]]; then
            USE_PKG=1
        else
            USE_PKG=0
        fi
    fi
    # make: check, dependencies, Rust/Node.js, build, check build, install. pacman: check, dependencies, package.
    if [[ $DEPS_ONLY -eq 1 || $USE_PKG -eq 1 ]]; then STEPS=3; fi

    if [[ -z "${TOOLCHECKOUT_UPDATING:-}" ]]; then
        printf '%s%s%s installer\n' "$B$CYN" "$APP" "$R"
    fi

    # 1. check
    step "Checking your system"
    ok "Distribution: $DISTRO_NAME"
    if [[ $EUID -eq 0 ]]; then
        if [[ -n "${SUDO_USER:-}" && "$SUDO_USER" != root ]]; then
            fail "Please run ./install.sh as your normal user, without sudo. It asks for your password
  only for the steps that need it, and the build files stay yours."
        fi
        warn "Running as root: ToolCheckout will be built as root."
    fi
    if [[ $DEPS_ONLY -eq 1 ]]; then
        ok "Only installing the dependencies (--deps-only)."
    elif [[ $USE_PKG -eq 1 ]]; then
        ok "Install method: a pacman package built from packaging/arch/PKGBUILD"
    else
        ok "Install location: $INSTALL_PREFIX"
    fi
    detect_install
    if [[ $DEPS_ONLY -eq 0 ]]; then
        if [[ $INST_KIND == pacman && $USE_PKG -eq 0 ]]; then
            warn "The $PKG_NAME pacman package is installed too. Two copies can be confusing;"
            warn "remove the package first with: sudo pacman -R $PKG_NAME"
        elif [[ $INST_KIND == make && $USE_PKG -eq 1 ]]; then
            warn "An earlier installation exists in $INST_PREFIX. Remove it with ./uninstall.sh --prefix $INST_PREFIX"
        elif [[ $INST_KIND == make && "$INST_PREFIX" == "$INSTALL_PREFIX" ]]; then
            info "An existing installation was found - it will be updated."
        fi
        if { have dpkg && dpkg -s "$DISTRO_PKG" >/dev/null 2>&1; } || { have rpm && rpm -q "$DISTRO_PKG" >/dev/null 2>&1; }; then
            warn "ToolCheckout is also installed as a .deb/.rpm package ($DISTRO_PKG)."
            warn "Two copies can be confusing - consider removing the package first."
        fi
    fi

    # 2. dependencies
    step "Installing dependencies"
    load_cargo_env
    node_ok || load_nvm || true
    pick_packages "$USE_PKG"
    if [[ $INSTALL_DEPS -eq 0 ]]; then
        info "Skipped (--no-deps)."
    elif [[ "$FAMILY" == nixos ]]; then
        fail "NixOS cannot install packages this way. Use the AppImage from the Releases page:
    nix-shell -p appimage-run --run 'appimage-run ./ToolCheckout_*_amd64.AppImage'"
    elif [[ -z "$PM" ]]; then
        warn "Unknown distribution - please install these yourself, then run: ./install.sh --no-deps"
        info "  a C compiler, make, pkg-config, git and curl"
        info "  the development files for WebKitGTK 4.1 (webkit2gtk-4.1) and D-Bus (dbus-1)"
        info "  Rust $MIN_RUST+ and Node.js 20.19+ / 22.12+ (or let this script install them for you)"
        exit 1
    else
        [[ -n "$PRE_NOTE" ]] && info "$PRE_NOTE"
        info "Packages ($PM): ${PKGS[*]}"
        confirm "Install them now?" ||
            fail "Cancelled. Install them yourself, then run: ./install.sh --no-deps"
        install_packages ||
            fail "Installing the dependencies failed. Fix the problem above, or install them yourself and run: ./install.sh --no-deps"
        ok "Dependencies are installed."
    fi
    # 3. Rust and Node.js (on Arch, makepkg takes care of them)
    if [[ $USE_PKG -eq 0 ]]; then
        step "Checking Rust, Node.js and libraries"
        have make || fail "'make' is missing - please install it."
        ensure_rust
        ensure_node
        check_libs
    fi

    if [[ $DEPS_ONLY -eq 1 ]]; then
        printf '\n%s✔ Everything needed to build ToolCheckout is installed.%s\n\n' "$GRN$B" "$R"
        info "Build a package with ./packaging/deb/build-deb.sh or ./packaging/rpm/build-rpm.sh,"
        info "or install with ./install.sh --no-deps."
        echo
        return
    fi

    # 4.-6. build and install (pacman: step 3)
    if [[ $USE_PKG -eq 1 ]]; then
        install_with_pacman
    else
        install_with_make
    fi

    # done
    local done_msg="ToolCheckout is installed."
    [[ -n "${TOOLCHECKOUT_UPDATING:-}" ]] && done_msg="ToolCheckout is updated."
    printf '\n%s✔ %s%s\n\n' "$GRN$B" "$done_msg" "$R"
    info "Start it from your applications menu (${B}ToolCheckout${R}) or run:  ${B}toolcheckout${R}"
    info "Update later:   ${B}./update.sh${R}"
    if [[ $USE_PKG -eq 1 ]]; then
        info "Uninstall:      ${B}./uninstall.sh${R}   or   ${B}sudo pacman -R $PKG_NAME${R}"
    else
        local sudo_hint=""
        [[ $INSTALL_AS_USER -eq 0 && $EUID -ne 0 ]] && sudo_hint="sudo "
        info "Uninstall:      ${B}./uninstall.sh${R}   or   ${B}${sudo_hint}$INSTALL_PREFIX/lib/toolcheckout/uninstall.sh${R}"
        case ":$PATH:" in
            *":$INSTALL_PREFIX/bin:"*) ;;
            *) warn "$INSTALL_PREFIX/bin is not in your PATH - the menu entry works, but to start it from a"
               warn "terminal, add that folder to your PATH or run $INSTALL_PREFIX/bin/toolcheckout" ;;
        esac
    fi
    info "Your shop data is kept in ~/.local/share/toolcheckout (or the folder chosen in Settings);"
    info "updating or uninstalling never touches it."
    echo
}

# ---- update ------------------------------------------------------------------------------------------------

do_update() {
    printf '%s%s%s updater\n' "$B$CYN" "$APP" "$R"
    detect_install
    git -C "$SRC_DIR" rev-parse --git-dir >/dev/null 2>&1 ||
        fail "This folder is not a git checkout. Download the latest version with:
    git clone $REPO_URL.git && cd ToolCheckout && ./install.sh"
    printf '\n%s%s%s\n' "$B$CYN" "Downloading the latest version" "$R"
    local before after
    before="$(git -C "$SRC_DIR" rev-parse HEAD)"
    run git -C "$SRC_DIR" pull --ff-only ||
        fail "git pull failed. If you changed files in this folder, run 'git stash' first and try again."
    after="$(git -C "$SRC_DIR" rev-parse HEAD)"
    if [[ "$before" == "$after" ]]; then
        ok "Already up to date ($(git_version)) - reinstalling anyway."
    else
        ok "Updated from ${before:0:7} to $(git_version)."
    fi

    # Re-run the freshly downloaded installer, the same way ToolCheckout was installed before.
    local args=()
    case "$INST_KIND" in
        pacman) ;;
        make)
            args+=(--no-package --prefix "$INST_PREFIX")
            [[ "$INST_PREFIX" == "$HOME"/* ]] && args+=(--user) ;;
        none)
            warn "ToolCheckout does not seem to be installed yet - installing it now."
            [[ -n "$INSTALL_PREFIX" ]] && args+=(--prefix "$INSTALL_PREFIX")
            [[ $USER_INSTALL -eq 1 ]] && args+=(--user) ;;
    esac
    [[ $INSTALL_DEPS -eq 0 ]] && args+=(--no-deps)
    [[ $ASSUME_YES -eq 1 ]] && args+=(--yes)
    export TOOLCHECKOUT_UPDATING=1
    exec "$SRC_DIR/install.sh" "${args[@]}"
}

# ---- main ---------------------------------------------------------------------------------------------------

main() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --user)        USER_INSTALL=1 ;;
            --prefix)      [[ $# -ge 2 ]] || fail "--prefix needs a directory"; INSTALL_PREFIX="$2"; shift ;;
            --prefix=*)    INSTALL_PREFIX="${1#*=}" ;;
            --update|update)       MODE=update ;;
            --uninstall|uninstall) MODE=uninstall ;;
            --no-deps)     INSTALL_DEPS=0 ;;
            --deps-only)   DEPS_ONLY=1 ;;
            --no-package)  USE_PKG=0 ;;
            -y|--yes)      ASSUME_YES=1 ;;
            -h|--help)     usage; exit 0 ;;
            *)             usage >&2; fail "Unknown option: $1" ;;
        esac
        shift
    done

    case "$MODE" in
        uninstall)
            local args=()
            [[ -n "$INSTALL_PREFIX" ]] && args+=(--prefix "$INSTALL_PREFIX")
            [[ $ASSUME_YES -eq 1 ]] && args+=(--yes)
            exec "$SRC_DIR/uninstall.sh" "${args[@]}" ;;
        update) do_update ;;
        install) do_install ;;
    esac
}

# Everything runs inside functions, so "git pull" replacing this file during an update cannot
# confuse the running shell.
main "$@"; exit $?
