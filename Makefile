# ToolCheckout - build and install on Linux.
#
#   make                            build an optimized binary (run as your normal user)
#   sudo make install               install to /usr/local
#   make install PREFIX=~/.local    install for your user only (no sudo needed)
#   sudo make uninstall             remove again (use the same PREFIX as for install)
#   make check                      run the test suites
#
# Most people want ./install.sh instead: it also installs the build dependencies, keeps a list of
# the installed files for ./uninstall.sh and builds a pacman package on Arch.
#
# Packagers: make install DESTDIR="$pkgdir" PREFIX=/usr BIN_SRC=path/to/toolcheckout

APP_ID   := io.github.oreo1298.ToolCheckout
PREFIX   ?= /usr/local
DESTDIR  ?=
BINDIR   ?= $(PREFIX)/bin
DATADIR  ?= $(PREFIX)/share
LIBDIR   ?= $(PREFIX)/lib/toolcheckout
NPM      ?= npm
BIN_SRC  ?= target/release/toolcheckout

.PHONY: all build check install uninstall clean refresh-caches

all: build

# npm writes this file on every install, so node_modules is refreshed when the lockfile changes.
node_modules/.package-lock.json: package-lock.json
	$(NPM) ci --no-audit --no-fund

build: node_modules/.package-lock.json
	$(NPM) run tauri -- build --no-bundle

check: node_modules/.package-lock.json
	cargo test --workspace --locked
	$(NPM) test

install:
	@test -x "$(BIN_SRC)" || { echo "error: $(BIN_SRC) not found. Run 'make' first (without sudo)."; exit 1; }
	install -Dm755 "$(BIN_SRC)" "$(DESTDIR)$(BINDIR)/toolcheckout"
	install -d "$(DESTDIR)$(DATADIR)/applications"
	sed 's|@BINDIR@|$(BINDIR)|g' data/$(APP_ID).desktop.in > "$(DESTDIR)$(DATADIR)/applications/$(APP_ID).desktop"
	chmod 644 "$(DESTDIR)$(DATADIR)/applications/$(APP_ID).desktop"
	install -Dm644 data/$(APP_ID).metainfo.xml "$(DESTDIR)$(DATADIR)/metainfo/$(APP_ID).metainfo.xml"
	install -Dm644 src-tauri/icons/icon.svg "$(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/$(APP_ID).svg"
	install -Dm755 uninstall.sh "$(DESTDIR)$(LIBDIR)/uninstall.sh"
	@$(MAKE) --no-print-directory refresh-caches
	@echo "Installed ToolCheckout to $(DESTDIR)$(PREFIX)"

uninstall:
	rm -f "$(DESTDIR)$(BINDIR)/toolcheckout"
	rm -f "$(DESTDIR)$(DATADIR)/applications/$(APP_ID).desktop"
	rm -f "$(DESTDIR)$(DATADIR)/metainfo/$(APP_ID).metainfo.xml"
	rm -f "$(DESTDIR)$(DATADIR)/icons/hicolor/scalable/apps/$(APP_ID).svg"
	rm -rf "$(DESTDIR)$(LIBDIR)"
	@$(MAKE) --no-print-directory refresh-caches
	@echo "Removed ToolCheckout from $(DESTDIR)$(PREFIX) (your shop data was not touched)"

# Only refresh an icon cache that already exists: creating one in a directory that never had one
# (e.g. ~/.local) would hide icons that other programs add later without updating it.
refresh-caches:
	@if [ -z "$(DESTDIR)" ]; then \
		if [ -f "$(DATADIR)/icons/hicolor/icon-theme.cache" ]; then \
			gtk-update-icon-cache -qtf "$(DATADIR)/icons/hicolor" 2>/dev/null || true; \
		fi; \
		update-desktop-database -q "$(DATADIR)/applications" 2>/dev/null || true; \
	fi

clean:
	cargo clean
	rm -rf dist
