# ToolCheckout

ToolCheckout tracks who has which tool at a repair shop. It runs on the shared
counter PC (Windows 10/11 or Linux), works fully offline, and needs no server.

* **Check out in about three taps.** Scan a tool, scan or pick the borrower,
  they enter their PIN and confirm the tool's condition.
* **Accountable returns.** A technician inspects each return and records its
  condition. If a tool comes back in worse shape, ToolCheckout opens a damage
  incident against the borrower automatically.
* **Overdue tools stand out.** They show in red on the home screen, and the
  PC can send a desktop notification and an end-of-day reminder.
* **History you can check.** Every action is logged with who did it and which
  technician authorized it. The log is hash-chained, so later edits to it are
  detectable (see [What the audit log can and can't do](#what-the-audit-log-can-and-cant-do)).
* **Reports.** Per-borrower and per-tool summaries and open incidents, as CSV or
  printable pages. You can also print Code 128 asset labels on standard label
  sheets.

Want to look around before setting up? Choose **Try the demo** on the first
screen (see [Demo mode](#demo-mode)).

---

## Contents

* [Download](#download)
* [Install on Windows (no administrator needed)](#install-on-windows-no-administrator-needed)
* [Install on Linux](#install-on-linux)
* [First-run setup](#first-run-setup)
* [Everyday use](#everyday-use)
* [Barcode scanner tips](#barcode-scanner-tips)
* [Printing asset labels](#printing-asset-labels)
* [Backup and restore](#backup-and-restore)
* [Sharing data between Windows accounts](#sharing-data-between-windows-accounts)
* [PIN recovery](#pin-recovery)
* [What the audit log can and can't do](#what-the-audit-log-can-and-cant-do)
* [Demo mode](#demo-mode)
* [Where your data lives](#where-your-data-lives)
* [Building from source](#building-from-source)

---

## Download

Builds are produced by GitHub Actions on every push:

1. Open the repository on GitHub and go to **Actions → CI**.
2. Open the newest run with a green check mark.
3. Under **Artifacts**, download the build you need. Each one comes as a .zip
   file.

| Artifact                        | Contains                                              |
| ------------------------------- | ----------------------------------------------------- |
| `ToolCheckout-windows-installer` | `ToolCheckout_<version>_x64-setup.exe` (per-user installer) |
| `ToolCheckout-windows-portable`  | `ToolCheckout.exe`, a `portable` marker file, a short readme |
| `ToolCheckout-linux-appimage`    | `ToolCheckout_<version>_amd64.AppImage`              |
| `ToolCheckout-linux-deb`         | `ToolCheckout_<version>_amd64.deb`                   |

The builds are **not code-signed**. Windows will warn about that the first time
(see below).

---

## Install on Windows (no administrator needed)

ToolCheckout never needs administrator rights. It doesn't install services,
doesn't write to `Program Files` or machine-wide registry keys, and doesn't
open network ports. It never shows a UAC prompt. If you ever see one, cancel
it: something is wrong.

### Option A: installer (recommended)

1. Download and unzip `ToolCheckout-windows-installer`.
2. Your browser may warn that the file "isn't commonly downloaded". In Edge,
   open the **…** menu on the download and choose **Keep**, then
   **Show more → Keep anyway**.
3. Double-click `ToolCheckout_<version>_x64-setup.exe`.
4. If Windows shows **"Windows protected your PC"** (SmartScreen), click
   **More info**, then **Run anyway**. This happens because the build isn't
   code-signed. It's a one-time warning per file.
5. Click through the installer. It installs only for the Windows account
   you're signed in to, into `%LOCALAPPDATA%\ToolCheckout`, and adds a
   Start-menu shortcut.

ToolCheckout uses the Microsoft Edge WebView2 Runtime, which is part of
Windows 11 and of Windows 10 with current updates. On the rare PC without it,
the installer downloads Microsoft's per-user WebView2 installer, which also
doesn't need administrator rights (this needs an internet connection once).

**Uninstalling** (Settings → Apps → ToolCheckout → Uninstall) removes the
program but **keeps your data and backups**, even if you tick "Delete the
application data". That box only clears the WebView cache. To remove
everything, delete `%LOCALAPPDATA%\ToolCheckout` yourself afterwards.

### Option B: portable

The portable build runs from any folder you can write to, such as a USB stick,
`Documents\ToolCheckout` or the Desktop.

1. Download and unzip `ToolCheckout-windows-portable`.
2. Keep `ToolCheckout.exe` and the file named `portable` together in the same
   folder.
3. Double-click `ToolCheckout.exe`. If SmartScreen appears, click
   **More info → Run anyway**.

While the `portable` file sits next to the exe, *all* data (database, backups
and the WebView cache) goes into a `data` folder beside the exe, and nothing is
written anywhere else. Delete the `portable` file to use the normal per-user
data folder instead.

---

## Install on Linux

Use either the AppImage or the .deb on x86_64 Linux. Both were tested on
Ubuntu 22.04 with GNOME and KDE Plasma.

### AppImage (no root needed)

```sh
chmod +x ToolCheckout_*_amd64.AppImage
./ToolCheckout_*_amd64.AppImage
```

AppImages need FUSE 2. If the AppImage won't start, either install
it (`sudo apt install libfuse2`, or `libfuse2t64` on Ubuntu 24.04+) or run it
without FUSE:

```sh
./ToolCheckout_*_amd64.AppImage --appimage-extract-and-run
```

### .deb (Debian, Ubuntu, Mint…)

```sh
sudo apt install ./ToolCheckout_*_amd64.deb
```

Installing the .deb adds ToolCheckout to the application menu. Like any system
package it needs `sudo`. Use the AppImage if you don't have that.

### Theme and notifications

ToolCheckout follows the desktop's light/dark setting live, through the
freedesktop settings portal. It also picks up the accent color where the
desktop provides one (KDE, GNOME 47+). If the theme doesn't follow your
desktop, install the portal backend for it: `xdg-desktop-portal-gnome`,
`xdg-desktop-portal-kde` or `xdg-desktop-portal-gtk`. You can always pick
Light or Dark in **Settings → Appearance**. Overdue notifications use the
standard desktop notification service.

---

## First-run setup

The first time ToolCheckout starts, it asks for:

1. **Shop name and your name.** You become the first *technician*.
2. **Your technician PIN.** It can be 4 to 8 digits; 6 is recommended.
   Technicians are the admins. Their PIN is needed to add or edit tools,
   manage borrowers, inspect returns, resolve incidents, run reports, and
   change settings, backups and restores.
3. **A recovery code**, shown **once**. Write it down and keep it somewhere
   safe away from the counter, such as the office safe. It's the only way back
   in if every technician forgets their PIN (see [PIN recovery](#pin-recovery)).

After setup, unlock tech mode and:

* **Tools → Add tool.** Each tool gets an asset tag automatically (`T-0001`,
  `T-0002`, …). Add a photo and the replacement cost if you want to track
  damage costs.
* **Borrowers → Add borrower.** Enter their name, an optional employee ID, an
  optional badge barcode, and a 4-digit PIN that they choose.
* **Tools → Print labels.** Stick a label on each tool.
* **Settings.** Check the default loan period, the auto-lock time, reminders
  and the backup folder.

---

## Everyday use

**Tech mode.** Tap **Tech mode** (top right), pick your name and enter your
PIN. A green **Unlocked: your name** badge shows while it's on. Tech mode
locks again after 2 minutes without activity (you can change this in
Settings), or immediately when you tap **Lock**. Every technician action is
recorded under the technician who unlocked it.

**Check out.** Scan a tool's label anywhere on the home screen, or tap
**Check out**.

1. Add more tools if needed by scanning them.
2. Choose the borrower: scan their badge, or tap their name (recent and
   frequent borrowers come first).
3. The borrower enters their 4-digit PIN.
4. They check the condition and photo shown, adjust the due-back time if
   needed, and tap **I confirm this condition**. If something is already
   wrong with a tool, they tap **Report a problem** instead. That flags the
   tool for a technician to look at, and the borrower isn't blamed for it.

Borrowers can't check out tools marked *tech only*. Settings can also require
a technician PIN for every checkout.

**Return.** Scan the tool (or tap **Return**). There are two ways to finish:

* **A technician inspects it now:** they set the condition, add notes or a
  photo if needed, and enter their PIN. Tools in good order go back to
  *Available*. *Damaged* or *Broken* tools go to *Out for repair*.
* **The borrower drops it off:** they tap **Drop off for inspection** and
  enter their PIN. The tool shows under *Awaiting inspection* and stays
  attributed to them until a technician inspects it.

If the tool comes back in worse condition than when it went out, ToolCheckout
opens a **damage incident** with both conditions, the notes and the photos.
Technicians resolve incidents with an outcome and a note under **Incidents**.
A tool marked *Lost* opens a lost-tool incident against the borrower.

**Overdue.** A tool is overdue once its due-back time has passed. Overdue
loans appear in red at the top of the home screen. If desktop notifications
are on, the PC also reminds you, at most once an hour by default. After
closing time, an end-of-day reminder lists everything that's still out.

---

## Barcode scanner tips

Any USB (or Bluetooth HID) barcode scanner that acts like a keyboard works.
You don't need a driver or any setup in ToolCheckout.

* **Test the scanner in Notepad first.** Scan a label: you should see
  `T-0001` followed by a new line. If there's no new line, set the scanner to
  send an **Enter (CR) suffix**. The scanner's manual has a barcode for that.
  A Tab suffix also works.
* **Labels are Code 128**, which every scanner reads out of the box. QR codes
  on the labels are optional and need a 2D (imager) scanner.
* **Scan anywhere.** On the home screen and in the checkout and return
  screens, you don't need to click into a search box first. Scans typed while
  a text field is focused (notes, names) stay in that field, so a scan never
  triggers an action while you're writing.
* **Keyboard layout.** The scanner "types" using the keyboard layout set in
  Windows or Linux. If the scanner is set for a different country, letters and
  symbols can come out wrong. ToolCheckout ignores dashes and case in asset
  tags (`T-0001`, `t0001` and `T0001` all match), which covers the most common
  mismatch. On layouts where digits need Shift (French AZERTY, for example),
  set the scanner to the same layout as the PC.
* **Badges.** To link a borrower's ID badge, open the borrower, click into the
  **Badge code** field and scan the badge. Scanning that badge on the home
  screen or in checkout then selects that person. An employee ID typed on the
  keyboard works too.
* **Typing instead of scanning.** You can type a tag and press Enter
  anywhere you could scan.

---

## Printing asset labels

**Tools → Print labels** (or **Print label** on a tool's page):

1. Choose the tools, the label stock (Avery 5160/8160, Avery 5167, Avery
   L7160 or L7651) and optionally a QR code.
2. On a partly used sheet, set **Skip** to the number of labels already used.
3. Print. In the print dialog, set scale to **100% / Actual size** (not "Fit to
   page") and turn off headers and footers.

The first time, print one page on plain paper with **Outline** on, hold it
against a label sheet up to the light, and use **Calibrate position** to nudge
everything into place. The offset is remembered on this PC. On Linux it
starts at −6.35 mm, because the Linux print system adds its own page margins.

---

## Backup and restore

Everything ToolCheckout knows, including photos, is in one database file. Back
it up.

**Automatic backups.** Once a day ToolCheckout saves a copy into the backup
folder, keeping the newest 30 (you can change that). By default the backup
folder is `backups` inside the data folder. **Point it somewhere else**:
another drive, a USB stick that stays plugged in, or a folder synced with
OneDrive, Google Drive or Dropbox. A backup on the same disk won't survive the
disk dying. Change it in **Settings → Data & backups → Backups → Change…**.

**Manual backups** (Settings → Data & backups, technician PIN):

* **Back up now** saves a copy into the backup folder.
* **Save a copy to…** saves a copy anywhere you choose, such as a USB stick.

Manual backups are never deleted automatically.

**Restore** (Settings → Data & backups → **Restore from backup…**, technician
PIN):

1. Pick the backup file (`.db`). ToolCheckout checks it first and shows the
   shop name, last activity, counts, and whether its audit log verifies.
   Damaged files and backups whose audit log fails verification are refused.
2. Click **Replace data with this backup**. Your current data is first saved
   to the backup folder as `toolcheckout-before-restore-<date>.db`, so a
   restore can be undone by restoring that file.
3. Tech mode locks, because the restored data may have different technicians
   and PINs. The restore is recorded in the audit log.

**Moving to a new PC.** On the old PC, use **Save a copy to…** onto a USB
stick. On the new PC, install ToolCheckout, complete first-run setup with any
name and PIN, then restore the copy. Your old technicians, PINs and recovery
code apply again after the restore.

---

## Sharing data between Windows accounts

By default each Windows account has its own data. If several people sign in
to the counter PC with their own Windows accounts, point them all at one
folder:

1. In the first account: unlock tech mode, open **Settings → Data & backups →
   Use a different folder…** and choose (or create) a folder that every
   account can write to, for example `C:\Users\Public\ToolCheckout`.
   ToolCheckout copies its data there and uses it from then on. The old copy
   is left where it was as a fallback.
2. In each other account: install ToolCheckout and, on the first-run screen,
   choose **Already set up in another account? Use its shared folder**, then
   pick the **same folder**. (If that account is already set up, use **Use a
   different folder…** in Settings instead; ToolCheckout sees the data that's
   already there and offers **Switch to that data**.)

When you pick a folder, ToolCheckout checks that the current account can write
to it. Notes:

* **Same PC only.** Don't put the data folder on a network drive or a NAS. The
  database can be corrupted when it's shared over a network. To keep copies
  off the PC, use the backup folder instead.
* Two accounts can have ToolCheckout open at the same time (with fast user
  switching, for example). Each one sees the other's changes the next time its
  screen refreshes.
* **Back to default folder** in the same place switches an account back to
  its own per-user data.
* If the shared folder can't be opened at startup (for example, it's on a
  drive that isn't connected), ToolCheckout says so and offers **Try again**
  and **Use this account's own data folder instead**.

---

## PIN recovery

**A technician forgot their PIN.** Another technician can set a new one in
**Settings → Technicians → Change PIN**.

**No technician can unlock.** Use the recovery code from setup:

1. Tap **Tech mode → Forgot PIN?**
2. Enter the recovery code and choose the technician.
3. Set a new PIN.
4. A **new recovery code** is shown. Write it down: each code works only once.

If the recovery code may have been seen by someone else, make a new one in
**Settings → Technicians → New recovery code**. The old code stops working
immediately.

**Too many wrong PINs.** After 5 wrong PINs in a row, that PIN is locked for
30 seconds. Each further wrong PIN locks it for longer: 1 minute, 5 minutes,
15 minutes, then 1 hour. Lockouts are recorded in the audit log. A correct PIN
resets the count. Recovery-code attempts have their own lockout.

**A borrower forgot their PIN.** A technician opens **Borrowers →** the
person **→ Reset PIN**, and the borrower chooses a new one.

**The recovery code is lost too.** There's no back door, which is what keeps
a borrower from resetting a technician PIN. Keep the recovery code safe.

---

## What the audit log can and can't do

ToolCheckout's audit log is **tamper-evident, not tamper-proof**. It's honest
to spell out the difference:

* Every action is written to the log in the same database transaction as the
  change itself, with the time, who did it and the technician who authorized
  it.
* Each entry includes a SHA-256 hash of the previous entry. The database
  refuses ordinary edits and deletions of log entries.
* **Settings → Audit log → Verify log integrity** re-checks the whole chain.
  If any entry was edited, removed or inserted, it reports the first broken
  entry and why.
* It also compares the log with the newest backups. If the live log no longer
  extends what a backup recorded, it says so.

What it can't stop: anyone who can copy or edit the database file on the PC
(any Windows account with access to the data folder) can, with enough effort,
rewrite the *whole* log consistently, or delete the file. The chain check alone
can't catch a complete rewrite. A comparison with an **older backup that they
couldn't reach** can. So keep backups in a place that counter-PC users can't
modify (see [Backup and restore](#backup-and-restore)), and verify the log now
and then.

PINs are stored only as salted Argon2id hashes, never in readable form.

---

## Demo mode

**Try the demo** (on the first screen, or **Settings → Data & backups → Open
demo**) loads a sample shop with two weeks of history: tools out, an overdue
multimeter, drop-offs awaiting inspection, a problem report, damage and lost
incidents. It's useful for training and for trying features safely.

* A **DEMO DATA** badge and a banner make it obvious. The banner lists the demo
  PINs: technician `123456`, borrowers `1234`, badges `DEMO-01` to `DEMO-06`,
  and tool tags `T-0001` to `T-0015`. Type a tag and press Enter to simulate a
  scan.
* Demo data lives in a separate file. Your real data is never opened or
  changed while the demo is on.
* **Leave demo** (on the banner, then Settings) wipes the demo data and
  returns to your shop. **Reset demo data** starts it over.
* Backups, restore and moving the data folder are switched off in the demo.

---

## Where your data lives

| Situation                       | Data folder                                    |
| ------------------------------- | ---------------------------------------------- |
| Windows (installer)             | `%LOCALAPPDATA%\ToolCheckout`                  |
| Windows portable                | `data\` next to `ToolCheckout.exe`             |
| Linux                           | `~/.local/share/toolcheckout` (or `$XDG_DATA_HOME/toolcheckout`) |
| Custom folder chosen in Settings | wherever you chose (remembered in `data-location.json` in the default folder) |

Inside it:

* `toolcheckout.db` is the database: tools, people, loans, photos, settings
  and the audit log. (The `-wal` and `-shm` files next to it are part of it
  while the app is running.)
* `backups\` holds the automatic and manual backups, unless you chose another
  backup folder.
* `toolcheckout-demo.db` and `demo-mode` exist only while the demo is on.

The current folder is shown under **Settings → Data & backups**. For testing,
setting the environment variable `TOOLCHECKOUT_DATA_DIR` overrides the
location.

---

## Building from source

You need [Rust](https://rustup.rs) (stable) and Node.js 22 or later. On Linux
you also need the WebKitGTK development packages:

```sh
sudo apt install libwebkit2gtk-4.1-dev librsvg2-dev libssl-dev libxdo-dev patchelf
```

Then:

```sh
npm ci
npm run tauri dev                     # run with hot reload
npm run tauri build                   # installers into target/release/bundle/
```

Tests and checks (the same ones CI runs):

```sh
cargo test --workspace                # business logic, database, audit, backups…
npm test                              # frontend unit and smoke tests (Vitest)
npm run check                         # Svelte/TypeScript type check
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

`scripts/linux-theme-check.sh` reproduces the live light/dark check against
real GNOME and KDE portal backends in a headless X session.

### Layout

| Path                    | What                                                                  |
| ----------------------- | --------------------------------------------------------------------- |
| `crates/core`           | All business logic and the SQLite data layer (no UI or Tauri code), so it can later sit behind a server |
| `src-tauri`             | The desktop shell: Tauri commands, OS theme/accent, notifications, background jobs |
| `src`                   | The Svelte + TypeScript UI                                            |
| `.github/workflows/ci.yml` | Tests on Linux and Windows; builds the NSIS installer, portable exe, AppImage and .deb |
| `PLAN.md`               | Architecture, data model, milestones                                  |
| `DECISIONS.md`          | Every judgment call made along the way, and why                      |
