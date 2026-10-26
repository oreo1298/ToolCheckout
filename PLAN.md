# ToolCheckout — Plan

A desktop tool-checkout app for a computer/electronics repair shop's shared
counter PC. Sales associates borrow the lead tech's tools; the app makes
borrowing fast (target: under 10 seconds) and leaves an attributable,
tamper-evident record of who has what, for how long, and in what condition it
came back.

## 1. Architecture

```
┌──────────────────────── Tauri 2 process ─────────────────────────┐
│                                                                   │
│  WebView (WebView2 / WebKitGTK)        Rust                       │
│  ┌──────────────────────────┐   IPC   ┌─────────────────────────┐ │
│  │ Svelte 5 + TypeScript    │ ──────▶ │ src-tauri (app shell)   │ │
│  │  - screens, dialogs      │ invoke  │  - thin #[command]s     │ │
│  │  - scan capture          │ ◀────── │  - session/token state  │ │
│  │  - theme tokens          │ events  │  - theme watcher        │ │
│  │  - print layouts, labels │         │  - overdue notifier     │ │
│  └──────────────────────────┘         │  - backup scheduler     │ │
│                                       └───────────┬─────────────┘ │
│                                                   │ Rust API      │
│                                       ┌───────────▼─────────────┐ │
│                                       │ toolcheckout-core       │ │
│                                       │  ALL business logic:    │ │
│                                       │  status transitions,    │ │
│                                       │  PIN/lockout, sessions, │ │
│                                       │  audit chain, loans,    │ │
│                                       │  incidents, reports,    │ │
│                                       │  CSV, backups, demo     │ │
│                                       └───────────┬─────────────┘ │
│                                                   │ rusqlite      │
│                                            SQLite (WAL, FK on)    │
└───────────────────────────────────────────────────────────────────┘
```

* **`crates/core` (`toolcheckout-core`)** — a plain Rust library with no
  Tauri/WebKit dependency. Every rule lives here: valid status transitions,
  condition-downgrade → incident, overdue math, Argon2id PINs and lockout,
  technician unlock sessions, borrower confirmation tokens, the hash-chained
  audit log, reports and CSV. It is unit-tested headlessly (`cargo test -p
  toolcheckout-core`) on both Linux and Windows CI runners.
* **`src-tauri` (`toolcheckout`)** — the desktop shell. Commands are thin
  wrappers that lock the core service, call one method, and emit a
  `data-changed` event. The frontend never passes a technician id: privileged
  commands read the unlocked technician from backend session state, so the UI
  cannot bypass the PIN.
* **Frontend (`src/`)** — Svelte 5 (runes) + TypeScript + Vite. Presentation
  only: layouts, scan-buffer capture, theme tokens, printable layouts, and
  barcode rendering for labels.
* **Data-layer isolation** — the Tauri layer never touches SQL. The
  `core::Service` facade is the only entry point; a future shared/server mode
  would host the same core behind a network API and swap the desktop facade
  for a client, without touching screens.

### Platform specifics
* **Windows**: NSIS `installMode: currentUser` (installs under
  `%LOCALAPPDATA%`, HKCU only, `RequestExecutionLevel user`, no UAC). Portable
  build = the raw `.exe` plus a `portable` marker file. No services, no
  listening ports (Tauri serves assets through a custom protocol, not a
  localhost server). Accent color read from `HKCU\Software\Microsoft\Windows\DWM`.
* **Linux**: AppImage + .deb. Live theme comes from
  `prefers-color-scheme` *and* the freedesktop portal
  (`org.freedesktop.portal.Settings`, `org.freedesktop.appearance`
  `color-scheme` / `accent-color`) via zbus, because GTK3/WebKitGTK does not see
  GNOME's libadwaita dark-style toggle.
* **Theme resolution** (frontend): user override (System/Light/Dark) →
  portal value if it states a preference → `prefers-color-scheme`. Applied as
  `data-theme` on `<html>`; CSS tokens on `:root`. Native window theme is set
  to match so title bars follow too.

## 2. Data model (SQLite)

All timestamps are UTC Unix milliseconds (`INTEGER`). Money is integer cents.
`PRAGMA journal_mode=WAL`, `foreign_keys=ON`, schema version in
`PRAGMA user_version`, migrations embedded in the binary and applied in a
transaction at startup.

| Table | Purpose / key columns |
|---|---|
| `settings` | `key TEXT PK`, `value TEXT` (JSON). Shop name, loan period, toggles, timeouts, backup config, next asset number. |
| `technicians` | `id`, `name`, `pin_hash` (Argon2id PHC), `active`, `failed_attempts`, `lockout_level`, `locked_until`, `created_at`. |
| `borrowers` | `id`, `name`, `employee_id`, `badge_code UNIQUE`, `pin_hash NULL`, `active`, lockout columns, `created_at`. |
| `recovery_codes` | `id`, `code_hash`, `created_at`, `used_at`, `used_by_tech`; single active row; lockout columns. |
| `photos` | `id`, `mime`, `data BLOB`, `created_at`. Immutable (downscaled JPEG from the UI). |
| `tools` | `id`, `asset_tag UNIQUE` (T-0001…), `name`, `category`, `description`, `serial`, `model`, `photo_id`, `replacement_cost_cents`, `home_location`, `default_loan_minutes`, `tech_only`, `condition`, `condition_notes`, `status`, `created_at`, `updated_at`. |
| `condition_events` | Append-only. `tool_id`, `condition`, `notes`, `photo_id`, `reason`, `source` (initial / manual / return_inspection / found / repair), `checkout_id`, `tech_id`, `created_at`. UPDATE/DELETE blocked by triggers. |
| `checkouts` | One row per tool per loan. `batch_id`, `tool_id`, `borrower_id`, `checked_out_at`, `due_at`, `checkout_condition`, `authorized_by_tech`, `state` (out / awaiting_inspection / closed / lost), `returned_at`, `dropoff_note`, `inspected_at`, `inspected_by_tech`, `return_condition`, `return_notes`, `return_photo_id`. |
| `incidents` | `kind` (damage / lost / problem_report), `tool_id`, `borrower_id NULL`, `checkout_id NULL`, `before_condition`, `after_condition`, `notes`, `photo_ids` (JSON), `created_at`, `created_by_kind/_id`, `status` (open / resolved), `outcome`, `resolution_note`, `resolved_at`, `resolved_by_tech`. |
| `audit_log` | Append-only. `id`, `ts`, `actor_kind` (technician / borrower / system), `actor_id`, `actor_name`, `action`, `entity_kind`, `entity_id`, `before_json`, `after_json`, `prev_hash`, `hash`. UPDATE/DELETE blocked by triggers; `hash = SHA-256(length-prefixed fields ‖ prev_hash)`. |

### Enumerations
* **Condition** (ordered): New < Good < Fair < Worn < Damaged < Broken.
  A return is a *downgrade* when `rank(return) > rank(checkout)`.
* **Tool status**: `available`, `checked_out`, `awaiting_inspection`,
  `out_for_repair`, `lost`, `retired`.

### Status transitions (enforced in `core::status`)

| From \ To | available | checked_out | awaiting_inspection | out_for_repair | lost | retired |
|---|---|---|---|---|---|---|
| available | – | checkout | problem report | send to repair | mark lost | retire |
| checked_out | inspected return | – | drop-off | inspected return (Damaged/Broken) | mark lost | – |
| awaiting_inspection | inspection | – | – | inspection (Damaged/Broken) | mark lost | – |
| out_for_repair | back from repair | – | – | – | mark lost | retire |
| lost | found | – | – | found (Damaged/Broken) | – | retire |
| retired | reinstate | – | – | – | – | – |

## 3. Screens

1. **First-run setup** — shop name, first technician name + PIN (twice), then
   the one-time recovery code ("I wrote it down" checkbox). Or "Try the demo".
2. **App shell** — header with shop name, DEMO banner when applicable,
   `Unlocked: <name>` badge + countdown + one-tap Lock, nav (Home, Tools,
   Borrowers, Incidents, Reports, Settings). Tech-only sections show a lock
   glyph and prompt for a PIN.
3. **Home** — big *Check out* / *Return* buttons, search, always-on scan
   capture; panels: Overdue (red), Currently out, Awaiting inspection.
4. **Checkout sheet** — tool cart (scan/search to add) → borrower picker
   (recent/frequent first, badge scan) → PIN pad → condition confirmation with
   photos + due-back per tool → *I confirm this condition* / *Report a problem*.
5. **Return sheet** — tools being returned with borrower, checkout time,
   checkout condition → *Inspect now* (tech: condition, notes, photo) or
   *Drop off for inspection* (borrower PIN).
6. **Inspection sheet** — for awaiting-inspection items.
7. **Tools** — list/search/filter; tool detail with history tabs (loans,
   conditions, incidents); add/edit/duplicate/retire; update condition with
   reason; send to repair / back from repair; mark lost / found; label printing.
8. **Borrowers** — list, add/edit, badge code, activate/deactivate, reset PIN,
   per-borrower history.
9. **Incidents** — open/resolved, resolve with outcome + note.
10. **Reports** — per-borrower summary, per-tool history, open incidents;
    filters (date range, borrower, tool); CSV export; printable layout.
11. **Labels** — choose tools + label stock; Code 128 (optional QR) sheet.
12. **Settings** — all settings, technicians, data location, backups
    (manual backup/restore), verify log integrity, audit log viewer, recovery
    code regeneration, demo controls.

## 4. Milestones (one commit or more each)

1. **Scaffold + CI + theme-following shell** — workspace, Tauri app, Svelte
   shell, theme tokens + live switching (media query + portal + Windows
   accent), GitHub Actions: tests, Windows NSIS (per-user) + portable, Linux
   AppImage + deb, uploaded as artifacts.
2. **Database, migrations, audit chain, first-run, PINs** — migrations,
   WAL/FK, hash chain + verify, Argon2id, lockout escalation, recovery code,
   tech unlock session with inactivity timeout, first-run UI, unlock UI.
3. **Tools menu + condition history** — CRUD, duplicate, retire, photos,
   condition updates with reason, status actions, history views.
4. **Borrowers** — CRUD, badges, PINs, reset, active flag, avatars.
5. **Home + flows** — scan-to-act, checkout, return inspection, drop-off,
   problem reports, damage/lost incidents, resolution.
6. **Overdue + reports + labels** — overdue panel and throttled
   notifications, end-of-day reminder, reports with filters, CSV + print,
   Code 128/QR label sheets.
7. **Backups, settings, portable, polish, README** — daily backups (keep N),
   manual backup/restore, data-folder relocation, portable mode, demo mode,
   README, final test pass, CI green on both platforms.

## 5. Testing

* **Rust unit tests (core)** — status transitions, condition downgrade →
  incident, overdue calculation, PIN hashing + lockout escalation, sessions
  and tokens, audit chain + tamper detection, migrations (fresh + idempotent +
  constraints), flows end-to-end against an in-memory/temp DB, reports, CSV,
  backup/restore.
* **Frontend (Vitest + Testing Library, jsdom)** — theme resolution, scan
  buffer heuristics, PIN pad, formatting helpers, and a smoke test rendering
  the shell/home against a mocked Tauri IPC.
* **Manual/automated smoke** — the real app run under Xvfb on Linux with
  screenshots of key screens, and a mock freedesktop portal to prove live theme
  switching.
