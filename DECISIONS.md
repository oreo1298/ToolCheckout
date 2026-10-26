# Decisions

Choices made without asking, with the reasoning. Newest sections are appended
at the bottom of each group.

## Stack

* **Tauri 2 (2.12) + Rust + Svelte 5 + TypeScript + Vite 8**, SQLite through
  `rusqlite` (bundled), Argon2id through the RustCrypto `argon2` crate. This is
  the recommended stack; no blocker was found, so nothing was swapped.
* **Svelte over React** — less runtime and boilerplate for a single-window app,
  and runes make the live panels simple.
* **TypeScript 6.x, not 7.x** — `svelte-check` (type-checking `.svelte`
  files) only supports TS `^5 || ^6` today. TS 7 adds nothing this project
  needs.
* **Business logic lives in a separate crate, `toolcheckout-core`**, which
  has no Tauri/WebKit dependency, so its tests run headlessly on plain CI
  runners (Linux and Windows) and the UI cannot bypass it.

## Naming

* Product name **ToolCheckout** (not "Toolcrib", per the owner). Bundle
  identifier `com.toolcheckout.desktop` (Tauri warns against identifiers
  ending in `.app`).

## Theme

* **Live theme on Linux reads the freedesktop portal** (`org.freedesktop.appearance`
  `color-scheme` and `accent-color`) through zbus in addition to
  `prefers-color-scheme`. WebKitGTK derives `prefers-color-scheme` from the GTK
  theme name, which GNOME's libadwaita "Dark style" switch does not change, so
  the media query alone misses it. zbus 5 is already in the dependency tree
  (notification and single-instance plugins), so this adds no new stack.
* **Verified, not assumed.** `scripts/linux-theme-check.sh` starts the real
  `xdg-desktop-portal` with the GNOME backend (toggling the same GSettings key
  as the Settings app's Dark style switch) and with the KDE backend plus the
  Plasma platform theme (writing Breeze Light/Dark into `kdeglobals` and
  emitting `KGlobalSettings.notifyChange(PaletteChanged)`, which is what
  Plasma's Colors settings page does). In both, the running app changed
  light → dark → light without a restart (checked by screenshot pixels).
  This runs headless under Xvfb with a private D-Bus session, not a full
  gnome-shell/plasmashell session, so it proves the portal path end to end
  but not, for example, a distro that ships a broken portal config.
* **Resolution order:** Settings override (System/Light/Dark) → OS value
  (portal on Linux, registry on Windows) when it states a preference →
  `prefers-color-scheme`. The native window theme is set to match so GTK
  header bars and Windows title bars follow too.
* **Accent color:** from the portal (`accent-color`; KDE Plasma and GNOME 47+)
  and from `HKCU\Software\Microsoft\Windows\DWM\AccentColor` on Windows,
  polled every 3 s (a cheap HKCU read; avoids a hidden message window). The
  accent is nudged toward black or white until it reaches 3:1 contrast with
  the surface, so any OS accent stays legible. Default accent `#1f6feb`.

## Packaging

* **NSIS `installMode: currentUser`** → installs to `%LOCALAPPDATA%`, writes
  only HKCU, `RequestExecutionLevel user`. CI fails the build if the installer
  manifest is not `asInvoker`.
* **WebView2:** left at Tauri's default (download bootstrapper, only run if
  WebView2 is missing). Windows 10 (with current updates) and 11 already
  include WebView2. Microsoft's Evergreen bootstrapper does a per-user install
  when it is not elevated, so even the fallback path should not prompt for UAC.
* **Portable build** = the same `ToolCheckout.exe` plus a `portable` marker
  file and a short README, uploaded as a folder artifact. The marker switches
  all data (database, backups, WebView2 profile) to `data\` beside the exe.
* **Mobile crate types dropped.** The template's `staticlib`/`cdylib` are only
  for iOS/Android and roughly doubled link time and disk use.
