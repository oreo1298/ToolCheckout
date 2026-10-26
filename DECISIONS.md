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

## Security & accountability

* **Tech unlock = pick your name, then PIN.** With several technicians, a
  bare PIN would have to be tried against every hash, and a lockout could not
  be tied to one account. With one technician the name is pre-selected, so
  it's just the PIN.
* **Lockout escalation:** 4 wrong PINs only count; the 5th locks for 30 s;
  each further wrong PIN in the same streak locks at the next step (1 m,
  5 m, 15 m, then 1 h each). A correct PIN resets the streak; so does a day
  without lockouts (so one typo next week doesn't re-lock instantly). The
  lockout state is committed even when the attempt fails, so retrying can't
  roll it back. Lockouts are written to the audit log; individual wrong PINs
  are not (they'd swamp the log, and the lockout entry says how many).
* **PINs:** Argon2id (RustCrypto defaults: 19 MiB, t=2, p=1), random salt per
  hash. Technician PINs 4–8 digits (6 suggested), borrower PINs exactly 4.
* **Recovery code:** 16 Crockford-base32 characters (80 bits) shown once at
  setup, stored only as an Argon2id hash. Using it sets a new PIN for a
  chosen technician, clears that technician's lockout, and immediately
  issues a replacement code (each code works once). Recovery attempts have
  their own lockout. A technician can regenerate the code in Settings.
* **Borrower confirmation tokens:** entering a borrower PIN mints a random
  128-bit token valid for 3 minutes. Checkout, problem reports and drop-offs
  require a valid token, so the multi-step UI can't skip the PIN check and the
  PIN is hashed only once per checkout.
* **Tech session** lives only in memory: restarting the app locks it. The UI
  sends a throttled "activity" ping on any tap/keypress, which extends the
  window; every tech-only command also checks and extends it in the backend.
* **Audit log:** one row per action, written in the same transaction as the
  change. `hash = SHA-256("toolcheckout-audit-v1" ‖ length-prefixed fields ‖
  prev_hash)`. Triggers block UPDATE/DELETE. "Verify log integrity" walks the
  chain and names the first broken entry and why (edited, missing, relinked).
  Limitation, stated in the README too: someone with file access can drop the
  triggers and rewrite the *entire* chain consistently. Comparing against an
  older backup catches that (`audit::verify_prefix_matches`), so keep backups
  somewhere the counter PC's users can't edit.
* **Settings** are stored one key per row; a malformed value falls back to its
  default instead of preventing startup. Changes are audited with only the
  keys that changed.

## Data

* **Timestamps** are UTC milliseconds (INTEGER); money is integer cents.
  All tables are `STRICT`. Every migration runs in its own transaction and
  bumps `PRAGMA user_version`; a database from a newer app version is refused
  with a clear message instead of being opened.
* **Data folder** resolution order: `portable` marker → `TOOLCHECKOUT_DATA_DIR`
  env var (for testing) → custom folder from a pointer file in the default
  folder → default per-user folder. `%LOCALAPPDATA%\ToolCheckout` on Windows,
  `$XDG_DATA_HOME/toolcheckout` on Linux (rather than Tauri's
  identifier-named folder, which is less discoverable).
* If the database can't be opened, the app shows the reason instead of
  crashing (e.g. a custom folder on a disconnected drive).

## Tools

* **The Tools menu is technician-only** (as specified). Borrowers never need
  it: the home screen's search and the checkout/return pickers use a separate
  read-only list command.
* **Asset tags** are sequential (`T-0001`…), assigned by the backend, and never
  reused, even after a tool is retired. Lookup ignores case and dashes, so a
  scanner on a different keyboard layout that mangles `-` still finds the tool.
* **Condition only changes through dedicated actions** that carry context:
  added (initial), "Update condition" (reason required), return/inspection,
  back from repair, reinstate, found. Editing a tool can't touch condition or
  status. Manual updates are allowed only while the tool is on the shelf or
  out for repair; a tool on loan gets its condition at return inspection.
* **Back from repair / reinstate** refuse Damaged/Broken: such a tool should
  stay in repair or be retired, not go back on the shelf.
* **Duplicate** opens the Add form pre-filled from the source tool, minus
  serial number and condition (they belong to the physical item), so the tech
  must choose the new tool's condition.
* **Photos** are stored in the database as immutable BLOBs (one-file backups;
  evidence can't be swapped). The UI re-encodes uploads to JPEG ≤ 1280 px
  (~25–150 KB); the backend sniffs the bytes and only accepts JPEG/PNG/WebP,
  4 MB max.
* **Status changes use compare-and-set** (`… WHERE status = <expected>`), so two
  windows (or two Windows accounts sharing the data folder) can't both act on
  the same stale tool.
