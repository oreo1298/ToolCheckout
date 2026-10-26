//! Where ToolCheckout keeps its data.
//!
//! 1. Portable: a file named `portable` next to the executable → `<exe dir>/data`.
//! 2. `TOOLCHECKOUT_DATA_DIR` environment variable (handy for testing).
//! 3. A custom folder chosen in Settings, remembered in a small pointer file
//!    inside the default folder (so several Windows accounts can share e.g.
//!    `C:\Users\Public\ToolCheckout`).
//! 4. Default per-user folder: `%LOCALAPPDATA%\ToolCheckout` on Windows,
//!    `$XDG_DATA_HOME/toolcheckout` (usually `~/.local/share/toolcheckout`) on Linux.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, invalid};

pub const DB_FILE: &str = "toolcheckout.db";
pub const DEMO_DB_FILE: &str = "toolcheckout-demo.db";
pub const DEMO_MARKER: &str = "demo-mode";
pub const PORTABLE_MARKER: &str = "portable";
pub const POINTER_FILE: &str = "data-location.json";
pub const ENV_OVERRIDE: &str = "TOOLCHECKOUT_DATA_DIR";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationMode {
    Default,
    Custom,
    Portable,
    Environment,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataLocation {
    pub dir: PathBuf,
    pub mode: LocationMode,
    pub default_dir: PathBuf,
}

impl DataLocation {
    pub fn db_path(&self) -> PathBuf {
        self.dir.join(DB_FILE)
    }
    pub fn demo_db_path(&self) -> PathBuf {
        self.dir.join(DEMO_DB_FILE)
    }
    pub fn demo_marker(&self) -> PathBuf {
        self.dir.join(DEMO_MARKER)
    }
    pub fn default_backup_dir(&self) -> PathBuf {
        self.dir.join("backups")
    }
    /// WebView profile folder; only overridden in portable mode.
    pub fn webview_dir(&self) -> Option<PathBuf> {
        (self.mode == LocationMode::Portable).then(|| self.dir.join("webview"))
    }
}

/// The per-user default folder for this OS.
pub fn default_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("ToolCheckout"))
    } else {
        let xdg = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute());
        xdg.or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .map(|d| d.join("toolcheckout"))
    }
}

#[derive(Serialize, Deserialize)]
struct Pointer {
    data_dir: PathBuf,
}

pub fn read_pointer(default_dir: &Path) -> Option<PathBuf> {
    let text = std::fs::read_to_string(default_dir.join(POINTER_FILE)).ok()?;
    let p: Pointer = serde_json::from_str(&text).ok()?;
    p.data_dir.is_absolute().then_some(p.data_dir)
}

/// Remembers a custom data folder (None = back to the default).
pub fn write_pointer(default_dir: &Path, custom: Option<&Path>) -> Result<()> {
    let file = default_dir.join(POINTER_FILE);
    match custom {
        Some(dir) if dir != default_dir => {
            if !dir.is_absolute() {
                return invalid("Choose a full folder path");
            }
            std::fs::create_dir_all(default_dir)?;
            let json = serde_json::to_string_pretty(&Pointer {
                data_dir: dir.to_path_buf(),
            })
            .expect("pointer serializes");
            std::fs::write(file, json)?;
        }
        _ => {
            if file.exists() {
                std::fs::remove_file(file)?;
            }
        }
    }
    Ok(())
}

pub fn resolve(
    exe_dir: Option<&Path>,
    env_override: Option<PathBuf>,
    default_dir: &Path,
) -> DataLocation {
    let loc = |dir: PathBuf, mode| DataLocation {
        dir,
        mode,
        default_dir: default_dir.to_path_buf(),
    };
    if let Some(exe) = exe_dir
        && exe.join(PORTABLE_MARKER).is_file()
    {
        return loc(exe.join("data"), LocationMode::Portable);
    }
    if let Some(dir) = env_override.filter(|d| d.is_absolute()) {
        return loc(dir, LocationMode::Environment);
    }
    if let Some(dir) = read_pointer(default_dir) {
        return loc(dir, LocationMode::Custom);
    }
    loc(default_dir.to_path_buf(), LocationMode::Default)
}

/// Resolves using the real executable path and environment.
pub fn resolve_current() -> Result<DataLocation> {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf));
    let Some(default) = default_dir() else {
        return invalid("Could not find the per-user data folder (LOCALAPPDATA/HOME is not set)");
    };
    Ok(resolve(
        exe_dir.as_deref(),
        std::env::var_os(ENV_OVERRIDE).map(PathBuf::from),
        &default,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_when_nothing_else() {
        let tmp = tempfile::tempdir().unwrap();
        let def = tmp.path().join("default");
        let l = resolve(Some(&tmp.path().join("exe")), None, &def);
        assert_eq!(l.mode, LocationMode::Default);
        assert_eq!(l.dir, def);
        assert_eq!(l.db_path(), def.join(DB_FILE));
        assert_eq!(l.webview_dir(), None);
    }

    #[test]
    fn portable_marker_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = tmp.path().join("app");
        std::fs::create_dir_all(&exe).unwrap();
        std::fs::write(exe.join(PORTABLE_MARKER), "").unwrap();
        let def = tmp.path().join("default");
        write_pointer(&def, Some(&tmp.path().join("shared"))).unwrap();
        let l = resolve(Some(&exe), Some(tmp.path().join("env")), &def);
        assert_eq!(l.mode, LocationMode::Portable);
        assert_eq!(l.dir, exe.join("data"));
        assert_eq!(l.webview_dir(), Some(exe.join("data").join("webview")));
    }

    #[test]
    fn custom_pointer_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let def = tmp.path().join("default");
        let shared = tmp.path().join("Public").join("ToolCheckout");
        write_pointer(&def, Some(&shared)).unwrap();
        let l = resolve(None, None, &def);
        assert_eq!(l.mode, LocationMode::Custom);
        assert_eq!(l.dir, shared);
        write_pointer(&def, None).unwrap();
        assert_eq!(resolve(None, None, &def).mode, LocationMode::Default);
        assert!(write_pointer(&def, Some(Path::new("relative/dir"))).is_err());
    }

    #[test]
    fn env_override_beats_pointer() {
        let tmp = tempfile::tempdir().unwrap();
        let def = tmp.path().join("default");
        write_pointer(&def, Some(&tmp.path().join("shared"))).unwrap();
        let l = resolve(None, Some(tmp.path().join("env")), &def);
        assert_eq!(l.mode, LocationMode::Environment);
        // relative env paths are ignored
        assert_eq!(
            resolve(None, Some("rel".into()), &def).mode,
            LocationMode::Custom
        );
    }

    #[test]
    fn garbage_pointer_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(POINTER_FILE), "{not json").unwrap();
        assert_eq!(resolve(None, None, tmp.path()).mode, LocationMode::Default);
    }
}
