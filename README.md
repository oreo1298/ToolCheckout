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
* [Development](#development)

---

## Download

Get the newest version from the
**[Releases page](https://github.com/oreo1298/ToolCheckout/releases/latest)**:

| File                                     | Use it for                                                        |
| ---------------------------------------- | ----------------------------------------------------------------- |
| `ToolCheckout_<version>_x64-setup.exe`   | **Windows 10/11 (recommended).** Installs for the current user, no administrator needed |
| `ToolCheckout_<version>_x64-portable.zip` | Windows, nothing installed: runs from a folder or USB stick       |
| `ToolCheckout_<version>_amd64.AppImage`  | Linux, any modern x86_64 distribution, no root needed             |
| `ToolCheckout_<version>_amd64.deb`       | Debian 12+, Ubuntu 22.04+, Linux Mint 21+ and other Ubuntu-based distributions |
| `SHA256SUMS.txt`                         | Checksums, if you want to verify a download                       |

The builds are **not code-signed**. Windows will warn about that the first time
(see below). To check that a download arrived intact, compare its checksum
with the one in `SHA256SUMS.txt`: in PowerShell run
`Get-FileHash .\ToolCheckout_<version>_x64-setup.exe`, and on Linux run
`sha256sum -c --ignore-missing SHA256SUMS.txt` in the download folder.

Every commit is also built by GitHub Actions (**Actions → CI →** a run **→
Artifacts**). Those builds are for testing. Use the Releases page for the
shop PC.

---

## Install on Windows (no administrator needed)

ToolCheckout never needs administrator rights. It doesn't install services,
doesn't write to `Program Files` or machine-wide registry keys, and doesn't
open network ports. It never shows a UAC prompt. If you ever see one, cancel
it: something is wrong.

### Option A: installer (recommended)

1. Download `ToolCheckout_<version>_x64-setup.exe` from the
   [Releases page](https://github.com/oreo1298/ToolCheckout/releases/latest).
2. Your browser may warn that the file "isn't commonly downloaded". In Edge,
   open the **…** menu on the download and choose **Keep**, then
   **Show more → Keep anyway**.
3. Double-click the downloaded file.
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

**Updating:** run the newer `setup.exe` the same way. Your data is kept.

**Uninstalling** (Settings → Apps → ToolCheckout → Uninstall) removes the
program but **keeps your data and backups**, even if you tick "Delete the
application data". That box only clears the WebView cache. To remove
everything, delete `%LOCALAPPDATA%\ToolCheckout` yourself afterwards.

### Option B: portable

The portable build runs from any folder you can write to, such as a USB stick,
`Documents\ToolCheckout` or the Desktop.

1. Download `ToolCheckout_<version>_x64-portable.zip` and unzip it. Before
   unzipping, you can right-click the .zip → **Properties** → tick
   **Unblock** → **OK**, so Windows doesn't warn about every file inside.
2. Keep `ToolCheckout.exe` and the file named `portable` together in the
   `ToolCheckout` folder you unzipped.
3. Double-click `ToolCheckout.exe`. If SmartScreen appears, click
   **More info → Run anyway**.

While the `portable` file sits next to the exe, *all* data (database, backups
and the WebView cache) goes into a `data` folder beside the exe, and nothing is
written anywhere else. Delete the `portable` file to use the normal per-user
data folder instead. To update, replace `ToolCheckout.exe` with the newer one
and keep the `data` folder.

---

## Install on Linux

There are three ways, for x86_64 PCs:

* **[`install.sh`](#install-with-installsh-any-distribution)** (recommended):
  builds ToolCheckout on your PC and installs it with a menu entry. It works on
  every common distribution and updates with one command.
* **[AppImage](#appimage)**: a single ready-made file from the Releases page.
  Runs on almost any current distribution, no root needed.
* **[.deb package](#deb-package-debian-ubuntu-mint)**: ready-made, for Debian,
  Ubuntu, Linux Mint and other Ubuntu-based distributions.

### Install with install.sh (any distribution)

```sh
git clone https://github.com/oreo1298/ToolCheckout.git
cd ToolCheckout
./install.sh            # for all users, into /usr/local (asks for your password)
```

or, just for your user account (into `~/.local`):

```sh
./install.sh --user
```

`install.sh` does the following:

1. Detects your distribution.
2. Installs what's missing with your package manager (`apt`, `dnf`, `pacman`,
   `zypper`, `xbps` or `emerge`), after asking you once.
3. Sets up Rust and Node.js just for your user (rustup and nvm) if your
   distribution's are missing or too old.
4. Builds ToolCheckout, checks that every library it needs is there, and
   installs it with a menu entry and icon.

On Arch-based distributions it builds a regular pacman package instead, so
pacman knows every file. It asks for your password only for the steps that
need it. Run it as your normal user, not with `sudo`. The first build takes 5
to 15 minutes and about 3 GB of disk space.

Then start **ToolCheckout** from your applications menu, or run `toolcheckout`.

If `git` is missing, install it first:

| Distribution | Command |
| --- | --- |
| Debian, Ubuntu, Linux Mint, Pop!\_OS, elementary OS, Zorin OS | `sudo apt install git` |
| Fedora, RHEL, AlmaLinux, Rocky Linux | `sudo dnf install git` |
| Arch, Manjaro, EndeavourOS, Garuda, CachyOS | `sudo pacman -S --needed git base-devel` |
| openSUSE | `sudo zypper install git` |
| Void | `sudo xbps-install -S git` |
| Gentoo | `sudo emerge --noreplace dev-vcs/git` |

#### Update and uninstall

```sh
./update.sh       # download the latest version and reinstall it the same way
./uninstall.sh    # remove ToolCheckout
```

Neither one ever touches your shop data (`~/.local/share/toolcheckout`, or
the folder chosen in Settings). If you deleted the `ToolCheckout` folder,
uninstall with the copy that `install.sh` keeps:
`sudo /usr/local/lib/toolcheckout/uninstall.sh`, or
`~/.local/lib/toolcheckout/uninstall.sh` for a `--user` install. On Arch, use
`sudo pacman -R toolcheckout`.

#### Options

| Option | Meaning |
| --- | --- |
| `--user` | install just for you into `~/.local`; no password needed for the install step |
| `--prefix DIR` | install somewhere else (default `/usr/local`) |
| `--update` | same as `./update.sh` |
| `--uninstall` | same as `./uninstall.sh` |
| `--no-deps` | don't install packages; you've installed the dependencies yourself |
| `--deps-only` | only install the dependencies, e.g. before building a .deb or .rpm |
| `--no-package` | on Arch-based distributions, install with `make` instead of a pacman package |
| `-y`, `--yes` | don't ask questions |

#### What it installs

These packages are installed automatically by `install.sh`:

| Distribution | Packages | Tested in CI |
| --- | --- | :-: |
| Ubuntu 22.04+, Linux Mint 21+, Pop!\_OS, elementary OS, Zorin OS, KDE neon, Debian 12+ | `build-essential pkg-config curl git libwebkit2gtk-4.1-dev libdbus-1-dev` (+ `nodejs npm` where new enough, e.g. Debian 13) | ✓ |
| Fedora 41+ | `gcc make pkgconf-pkg-config curl git webkit2gtk4.1-devel dbus-devel nodejs npm` | ✓ |
| RHEL, AlmaLinux, Rocky Linux 10 | the Fedora list, after enabling the EPEL and CRB repositories (`webkit2gtk4.1-devel` is in EPEL) | ✓ |
| Arch, Manjaro, EndeavourOS, Garuda, CachyOS | `base-devel git`, then `makepkg` adds `webkit2gtk-4.1 cargo nodejs npm` | ✓ |
| openSUSE Tumbleweed, Leap 16 | `gcc make pkg-config curl git 'pkgconfig(webkit2gtk-4.1)' 'pkgconfig(dbus-1)'` (+ `nodejs-default npm-default` on Tumbleweed) | ✓ |
| Void | `base-devel pkg-config curl git libwebkit2gtk41-devel dbus-devel nodejs` | ✓ |
| Gentoo | `net-libs/webkit-gtk:4.1 sys-apps/dbus dev-vcs/git net-misc/curl net-libs/nodejs` | – |

It also needs **Rust 1.90+** and **Node.js 20.19+ or 22.12+**. If yours are
missing or older, `install.sh` offers to install them just for you:

* Rust through [rustup](https://rustup.rs), into `~/.cargo`.
* Node.js 22 through [nvm](https://github.com/nvm-sh/nvm), into `~/.nvm`.
  This is needed on Ubuntu, Debian 12 and openSUSE Leap, and it doesn't touch
  the system's Node.js.

"Tested in CI" means that
[`linux-install.yml`](.github/workflows/linux-install.yml) runs `install.sh`,
`update.sh`, `uninstall.sh`, a `--user` install and the .deb/.rpm build scripts
on a clean container of that distribution, as a normal user with `sudo`. After
each install it checks that the program, its menu entry and every library it
needs are in place. Ubuntu-based distributions follow their Ubuntu base.
Gentoo isn't tested.

Older releases (Ubuntu 20.04, Debian 11, Linux Mint 20, Fedora 35 and
earlier) don't have WebKitGTK 4.1, so they can't build ToolCheckout. Their
glibc is also too old for the AppImage, so upgrade the distribution first.

#### Distribution notes

**Arch Linux, Manjaro, EndeavourOS, CachyOS:** `install.sh` builds the
package from [`packaging/arch/PKGBUILD`](packaging/arch/PKGBUILD) and installs
it with pacman. If you prefer doing it by hand, this is all it does:

```sh
cd ToolCheckout/packaging/arch
makepkg -si
```

**Debian, Ubuntu, Linux Mint, Pop!\_OS:** to install a regular .deb package
instead (the same package as on the Releases page):

```sh
./install.sh --deps-only
./packaging/deb/build-deb.sh
sudo apt install ./ToolCheckout_*_amd64.deb
```

Remove it with `sudo apt remove tool-checkout`.

**Fedora, RHEL, AlmaLinux, Rocky Linux, openSUSE:** to install an .rpm
package instead:

```sh
./install.sh --deps-only
./packaging/rpm/build-rpm.sh
sudo dnf install ./ToolCheckout-*.rpm
# openSUSE: sudo zypper install --allow-unsigned-rpm ./ToolCheckout-*.rpm
```

Remove it with `sudo dnf remove tool-checkout`
(`sudo zypper remove tool-checkout`).

**RHEL, AlmaLinux, Rocky Linux 10:** `install.sh` enables EPEL and the
CodeReady Builder (CRB) repository, after asking. On RHEL itself it uses
`subscription-manager` for CRB.

**NixOS, Fedora Silverblue, Kinoite, Bazzite and other immutable systems:**
use the [AppImage](#appimage).

**Gentoo:** not tested by us. WebKitGTK is large, so allow time for it to
compile unless you use the binary package host.

#### Building by hand

If you'd rather not use `install.sh`, install the packages and Rust/Node.js
listed above, then run:

```sh
make                      # build (as your normal user, not with sudo)
sudo make install         # into /usr/local; or: make install PREFIX=~/.local
sudo make uninstall       # remove again (same PREFIX)
```

`make check` runs the test suites. Packagers can use
`make install DESTDIR=… PREFIX=/usr`.

#### If something goes wrong

* **`Please run ./install.sh as your normal user`**: start it without `sudo`.
  It asks for your password when needed, and keeps the build files owned by
  you.
* **`The development files for webkit2gtk-4.1 were not found`**: run
  `./install.sh` without `--no-deps`, or your distribution is too old to have
  WebKitGTK 4.1.
* **`git pull failed`** during `./update.sh`: you changed files in the
  `ToolCheckout` folder. Run `git stash`, then `./update.sh` again.
* **The build is killed or the PC freezes** (low memory): build with fewer
  parallel jobs, e.g. `CARGO_BUILD_JOBS=2 ./install.sh`.
* **pacman can't find a package (404)**: update the system first with
  `sudo pacman -Syu`.

### AppImage

Download `ToolCheckout_<version>_amd64.AppImage` from the
[Releases page](https://github.com/oreo1298/ToolCheckout/releases/latest), then:

```sh
chmod +x ToolCheckout_*_amd64.AppImage
./ToolCheckout_*_amd64.AppImage
```

The AppImage carries its own copy of WebKitGTK, so it needs nothing else
installed except FUSE 2. It needs glibc 2.35 or newer (check with
`ldd --version`). That covers Ubuntu 22.04+, Debian 12+, Linux Mint 21+,
Fedora 36+, openSUSE Tumbleweed and Leap 16, Arch, and RHEL/AlmaLinux/Rocky 10.

If it won't start because FUSE is missing, install it
(`sudo apt install libfuse2`, `libfuse2t64` on Ubuntu 24.04+,
`sudo dnf install fuse-libs`, `sudo pacman -S fuse2`,
`sudo zypper install libfuse2`) or run it without FUSE:

```sh
./ToolCheckout_*_amd64.AppImage --appimage-extract-and-run
```

To get a menu entry for an AppImage, use a helper such as Gear Lever or
AppImageLauncher, or use [`install.sh`](#install-with-installsh-any-distribution)
instead.

**Fedora Silverblue, Kinoite, Bazzite and other immutable systems:** use the
AppImage. **NixOS:** run it with
`nix-shell -p appimage-run --run "appimage-run ./ToolCheckout_*_amd64.AppImage"`,
or set `programs.appimage.enable = true;` and
`programs.appimage.binfmt = true;` in your configuration to run AppImages
directly.

### .deb package (Debian, Ubuntu, Mint…)

Download `ToolCheckout_<version>_amd64.deb` from the
[Releases page](https://github.com/oreo1298/ToolCheckout/releases/latest), then:

```sh
sudo apt install ./ToolCheckout_*_amd64.deb
```

The `./` matters: it tells apt to install the downloaded file. This works on
Debian 12+, Ubuntu 22.04+, Linux Mint 21+, Pop!_OS 22.04+, elementary OS 7+,
Zorin OS 17+ and KDE neon. To update, install the newer .deb the same way.
To uninstall, run `sudo apt remove tool-checkout`. Your data is kept either
way.

### Theme, notifications and display problems

ToolCheckout follows the desktop's light/dark setting live, through the
freedesktop settings portal. It also picks up the accent color where the
desktop provides one (KDE, GNOME 47+). If the theme doesn't follow your
desktop, install the portal backend for it: `xdg-desktop-portal-gnome`,
`xdg-desktop-portal-kde` or `xdg-desktop-portal-gtk`. You can always pick
Light or Dark in **Settings → Appearance**. Overdue notifications use the
standard desktop notification service.

If the window stays blank or flickers (seen with some NVIDIA drivers and in
virtual machines), start it with WebKitGTK's DMA-BUF renderer turned off:
`WEBKIT_DISABLE_DMABUF_RENDERER=1 toolcheckout`. To make that permanent, copy
the menu entry to `~/.local/share/applications/` and change its `Exec=` line
to `Exec=env WEBKIT_DISABLE_DMABUF_RENDERER=1 toolcheckout`. The menu entry is
`io.github.oreo1298.ToolCheckout.desktop` for an `install.sh` install, or
`ToolCheckout.desktop` for a package.

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

## Development

Setting up a development machine:

* **Linux:** `./install.sh --deps-only` installs everything needed (see
  [What it installs](#what-it-installs)).
* **Windows:** [Rust](https://rustup.rs) with the "Desktop development with
  C++" workload of Visual Studio Build Tools (the rustup installer offers
  it), and Node.js 22. WebView2 is already part of Windows 10/11.

Then, in the repository:

```sh
npm ci
npx tauri dev                          # run with hot reload
npx tauri build --bundles deb          # Linux: or rpm, appimage; --no-bundle for just the program
npx tauri build --bundles nsis         # Windows installer
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

**Releasing:** bump `version` in `package.json`, `Cargo.toml` and
`src-tauri/tauri.conf.json` and commit. Then either push a tag such as
`v0.2.0`, or on GitHub open **Actions → CI → Run workflow**, tick **Publish a
GitHub Release** and run it (that creates the tag for you). CI builds and tests
everything and publishes a GitHub Release with the Windows installer, the
portable .zip, the AppImage, the .deb and `SHA256SUMS.txt`.

### Layout

| Path                    | What                                                                  |
| ----------------------- | --------------------------------------------------------------------- |
| `crates/core`           | All business logic and the SQLite data layer (no UI or Tauri code), so it can later sit behind a server |
| `src-tauri`             | The desktop shell: Tauri commands, OS theme/accent, notifications, background jobs |
| `src`                   | The Svelte + TypeScript UI                                            |
| `.github/workflows/ci.yml` | Tests on Linux and Windows; builds the NSIS installer, portable exe, AppImage and .deb; publishes releases for `v*` tags |
| `install.sh`, `update.sh`, `uninstall.sh` | The Linux installer (any distribution; pacman package on Arch) |
| `Makefile`, `data/` | Linux build/install rules, menu entry and AppStream metadata |
| `packaging/` | Arch `PKGBUILD`, .deb and .rpm build scripts, portable-build readme, release notes |
| `.github/workflows/linux-install.yml` | Tests the Linux installer and package scripts on ten distributions |
| `PLAN.md`               | Architecture, data model, milestones                                  |
| `DECISIONS.md`          | Every judgment call made along the way, and why                      |
