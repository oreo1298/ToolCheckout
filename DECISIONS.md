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
* **Releases** are published by CI when a `v*` tag is pushed, or from a
  manual "Run workflow" with *Publish a GitHub Release* ticked (CI then
  creates the `v<version>` tag on the built commit; this is also how v0.1.0
  was published, because this session could push only its branch, not tags).
  The release job checks that a pushed tag matches the version in
  `tauri.conf.json` and refuses to overwrite an existing release, then
  attaches the NSIS setup .exe, the portable build as a .zip (exe + `portable`
  marker + readme, so unzipping gives a ready folder), the AppImage, the .deb
  and `SHA256SUMS.txt`. No bare portable .exe: without its marker it would
  silently use the per-user data folder, which is what the installer is for.
  No .rpm in releases: Tauri's rpm has no distribution-neutral dependency
  names, so Fedora/openSUSE users build it from source (README).
* **Linux build-from-source docs are tested.** Each README command block is
  tagged `<!-- ci-linux: NAME -->`. `linux-source-build.yml` extracts the
  blocks a reader of each distribution follows and runs them unchanged (apart
  from auto-answering package-manager prompts) in clean containers, as a
  non-root user with sudo, then checks that the installed binary resolves all
  its libraries. It runs when the README, the install script or the
  dependency manifests change, and monthly to catch package renames.
* **Minimal Linux build dependencies.** The lockfile has no `openssl-sys` or
  `libxdo-sys`, and the tray library is loaded at runtime, so Tauri's generic
  list (OpenSSL, libxdo, appindicator, librsvg) isn't needed. What is needed:
  a C compiler (bundled SQLite), pkg-config, WebKitGTK 4.1 (which brings GTK
  3 and libsoup 3) and D-Bus (linked by `tao`).
* **Minimum Rust is 1.90** (`rust-version`), the newest requirement among the
  locked dependencies (Tauri 2.12, muda). Checked by compiling the workspace
  with Rust 1.90. The README recommends rustup because distribution Rust is
  often older.
* **`scripts/install-linux.sh`** installs a `--no-bundle` build for
  distributions without .deb/.rpm (Arch, Void, Gentoo…): binary, menu entry
  and icons under `~/.local` (or `/usr/local` with `--system`), mirroring the
  .deb's file names, with `--uninstall`.
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

## Borrowers

* **Never deleted, only deactivated**, so their history and incidents stay
  attributable. Deactivating someone who still has tools out is refused.
* **Borrower PIN is set by the borrower** on the counter PC while a technician
  has the Borrowers screen unlocked (enter twice). Required at creation while
  "Borrowers confirm with PIN" is on (the default). A technician can reset it,
  which also clears a lockout; the audit log records who reset whose PIN
  (never the PIN or hash).
* **Badge codes** are stored trimmed and upper-cased (scanner case doesn't
  matter), must be unique, and may not look like a tool tag (`T` + digits) so a
  scan is never ambiguous. Employee IDs also work as scan codes.
* **Picker order:** people who borrowed in the last 7 days (most recent
  first), then by number of checkouts in the last 90 days, then A–Z. The
  usual borrowers are one tap away without a search.
* **Borrower list/details are technician-only**; the checkout picker exposes
  only active names (and employee IDs, to tell two "Alex"es apart).

## Loans, returns and incidents

* **Who is the actor.** Checkout, problem reports and drop-offs are recorded
  with the *borrower* as actor (they proved it with their PIN). Inspections,
  lost/found and incident resolution are recorded with the *technician*. When
  a technician had to authorize a checkout (tech-only tool, or "require tech
  PIN" setting), their name is stored on the loan and in the audit entry.
* **Tech-only tools** can't be checked out by a borrower alone; a technician
  can authorize the loan with their PIN (recorded), rather than the tool being
  impossible to lend at all.
* **Checkout is all-or-nothing**: if any tool in the cart is no longer
  available, nothing is lent. One row per tool, grouped by a batch id.
* **The checkout itself is the condition confirmation.** The tool's condition
  at that moment is stored on the loan (`checkout_condition`) and in the audit
  entry as `conditionConfirmed`.
* **Overdue** means `now > due_at` while the tool is still out. A dropped-off
  tool is no longer overdue; its lateness is fixed at the drop-off time, and
  the inspection keeps that return time.
* **Damage incident** is opened automatically when the inspected condition is
  worse than the checkout condition (rank comparison New < … < Broken).
  Damaged/Broken returns go to "Out for repair".
* **Problem reports** are stored as incidents of kind `problem_report` (with
  the reporter as borrower, not at fault). The tool goes to "awaiting
  inspection"; the inspection resolves the report automatically (outcome
  "Inspected"). If the inspection finds it worse than its recorded condition,
  a damage incident is opened *without* a borrower: the tool was damaged
  between loans, after a technician last recorded its condition, so pinning it
  on the previous borrower would be unfair. The notes name the last borrower
  for context.
* **Lost:** a technician can mark a tool lost from any active status; if it
  was on loan the Lost incident names the borrower and the loan is closed as
  lost. **Found** resolves open Lost incidents ("Found") and, if the tool came
  back worse than when it was lent, opens a damage incident against that
  borrower.
* **Scan resolution order:** tool tag first, then borrower badge, then
  employee ID. Typing a tag and pressing Enter anywhere on the home screen
  works exactly like a scan (keyboard-only use).
* **Scan capture ignores text fields** (so typing notes never triggers an
  action) and only one screen listens at a time (sheets and dialogs on top
  silence the home screen's listener).

## Overdue, reports, printing, labels

* **Notifications** are scheduled by pure, unit-tested core logic and
  delivered by a background thread every 30 s through the notification plugin
  (Windows toast / freedesktop D-Bus on Linux, verified with `dbus-monitor`).
  Rules: one notification when loans go overdue (batched), a "still overdue"
  summary every N minutes (Settings, default 60), never two overdue
  notifications within 5 minutes, and an end-of-day summary once per day at
  closing time (default 17:30) if anything is still out. The same messages
  appear as in-app toasts (in case OS notifications are disabled), and the
  home screen shows an end-of-day banner after closing time.
* **Reports** are computed in the backend. "Late return" = returned
  (inspected or dropped off) after the due time; average lateness is over late
  returns only. "Cost of damaged/lost" sums replacement cost of damage/lost
  incidents attributed to the borrower, except those resolved as *no fault*,
  *found* or *inspected*. Date filters apply to checkout time for loans and to
  creation time for incidents; the open-incidents report ignores the date
  range (an open incident matters however old it is).
* **CSV export** is built in Rust and saved through the native save dialog:
  UTF-8 with BOM and CRLF so Excel opens it correctly, and cells starting with
  `= + - @` are prefixed with `'` (CSV/formula injection), except plain
  negative numbers.
* **Printing** uses the system print dialog (`window.print()`; "Save as PDF"
  covers PDF export). The printable content is mounted in its own root at the
  end of `<body>` and everything else is hidden, so dialogs never end up in
  the printout. Tauri's Rust-side `print()` only works on macOS, so it isn't
  used.
* **Labels**: Code 128 via JsBarcode (what USB scanners read), optional QR
  via `qrcode`. Stocks: Avery 5160/8160, 5167, L7160, L7651, with "skip used
  labels" for partial sheets and an outline mode for test prints. Verified by
  printing to PDF under WebKitGTK and decoding the barcode from the PDF with
  `zbarimg`. **WebKitGTK ignores CSS `@page` size and margins** and applies
  GTK's default 0.25″ margins (measured), so the label dialog has a per-PC
  position calibration (mm), defaulting to −6.35 mm on Linux and 0 on Windows
  (WebView2 honours `@page`). The page box is sized to end just past the last
  label so uneven engine margins can't push out a blank page.

## Backups, data folder, demo

* **Backups are single-file snapshots made with `VACUUM INTO`**, which is
  consistent while the app is running and includes photos (they live in the
  database). A snapshot is written to a temporary name and renamed, so a
  half-written file never looks like a backup.
* **Automatic backups**: once per local calendar day (checked a minute after
  start, then every 30 minutes), named `toolcheckout-auto-<stamp>.db`, in
  `<data folder>/backups` unless Settings names another folder. Only the
  newest N automatic backups are kept (default 30). Manual backups
  (`toolcheckout-backup-…`) and pre-restore safety copies are never pruned.
* **Restore** (tech PIN) first inspects the file: SQLite integrity check, it
  is a ToolCheckout database, not from a newer app version, not demo data,
  and its own audit chain verifies. A backup whose chain is broken is
  refused (the dialog shows why). The current data is saved as
  `toolcheckout-before-restore-<stamp>.db` before SQLite's online backup API
  copies the backup in; migrations then run in case the backup is from an
  older version. Restore is audited and ends tech mode, because the restored
  data can have different technicians and PINs.
* **Backups double as tamper evidence**: "Verify log integrity" also checks
  that the live audit chain extends the chain stored in the newest three
  backups. A log rewritten from scratch passes the chain check but fails this
  one. The README tells shops to send backups to a folder or drive the
  counter PC's users can't edit.
* **Moving the data folder** (Settings → Data & backups, tech PIN): the
  chosen folder is checked for write access and for existing ToolCheckout
  data. Empty folder → the database is copied there (audited before the copy,
  so the record travels with the data) and used from then on; the old copy is
  left in place as a fallback. Folder with data → the app switches to it
  without copying. The choice is stored in `data-location.json` in the
  default per-user folder, so each Windows account points at the shared
  folder once. "Back to default folder" removes the pointer. Moving is
  disabled in portable mode and when `TOOLCHECKOUT_DATA_DIR` is set.
* **Who may switch folders:** a technician of the current data, except when
  there is nothing to protect yet: a fresh account's first-run screen offers
  "Use its shared folder" (switch only, no copy), and the "can't open its
  data" screen offers Try again / use this account's own folder. Switching
  never deletes or overwrites data, so this can't be used to destroy
  anything; someone who switches away still needs a technician PIN for the
  shared data once they switch back.
* **Shared folder caveats**: SQLite (WAL, 5 s busy timeout) handles two
  accounts with the app open at the same time on one PC. Network drives are
  not supported (SQLite locking over SMB is unreliable); the README says so.
  Whether a second Windows account can write files the first one created
  depends on the folder's permissions; the writability check runs when each
  account picks the folder. This was verified on Linux only.
* **Demo mode** uses a separate database (`toolcheckout-demo.db`) and a
  `demo-mode` marker file next to the real one; the real database is never
  opened or touched while the demo is on. The sample data is generated
  through the normal business logic on a simulated clock (two weeks of
  history, an overdue loan, drop-offs, a problem report, damage and lost
  incidents), so its audit log is genuine and verifies. The demo is marked by
  a "DEMO DATA" badge and a banner with the demo PINs. Backups, restore and
  data-folder moves are off in demo mode. "Leave demo" deletes the demo files.
  Entering the demo needs a tech PIN once real data exists; from the first-run
  screen it doesn't.
* **In-app confirmations** replace `window.confirm()`, which looks different
  in each webview, isn't touch-sized, and returned immediately without asking
  under WebKitGTK during testing.
* **Background refreshes that hit "technician PIN required"** (tech mode
  timed out, or a restore ended it) just switch the UI to locked instead of
  showing an error.

## Testing notes

* Core: unit tests for every rule the brief lists (status transitions,
  condition downgrade → incident, overdue, PIN hashing and lockout, audit
  chain and verification, migrations) plus backups, restore, demo generation,
  reminders, reports and CSV. They run on Linux and Windows in CI.
* Frontend: Vitest + Testing Library for pure helpers (scan buffer, format,
  label layout) and smoke tests that render the whole app against a mocked
  backend (`@tauri-apps/api/mocks`): first-run setup, the home screen, the
  demo banner, and the tech-gated screens. One of them caught a real bug
  (relative times were computed from a stale clock until the first tick).
* Manual end-to-end runs of the app (debug build and the AppImage) under
  Xvfb with real GNOME and KDE portals covered first run, tools, photos,
  borrowers, scan-to-checkout, drop-off, inspection with damage, incidents,
  reports, CSV, label printing to PDF, notifications, backup, restore, moving
  the data folder, portable mode and demo mode. Windows behaviour is covered
  by CI builds and tests only.
