//! OS appearance (light/dark preference and accent color), pushed live to the UI.
//!
//! * Linux: WebKitGTK only reports `prefers-color-scheme` from the GTK theme,
//!   which GNOME's libadwaita "dark style" toggle does not change. So we read
//!   the freedesktop portal (`org.freedesktop.appearance` `color-scheme` and
//!   `accent-color`), which both GNOME and KDE implement, and subscribe to
//!   `SettingChanged`.
//! * Windows: WebView2 already follows the app theme live; we additionally read
//!   the accent color (and theme, for the native title bar) from HKCU.
//!
//! The UI merges this with its own `prefers-color-scheme` media query.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

pub const EVENT: &str = "os-appearance";

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OsAppearance {
    /// `"dark"`, `"light"`, or `None` when the OS states no preference.
    pub color_scheme: Option<&'static str>,
    /// Accent color as `#rrggbb`, when the OS exposes one.
    pub accent: Option<String>,
    /// Where the values came from (for diagnostics in Settings).
    pub source: &'static str,
}

#[derive(Default)]
pub struct AppearanceState(pub Mutex<OsAppearance>);

#[tauri::command]
pub fn get_os_appearance(state: tauri::State<'_, AppearanceState>) -> OsAppearance {
    state.0.lock().map(|a| a.clone()).unwrap_or_default()
}

/// Starts the platform watcher on a background thread.
pub fn start(app: AppHandle) {
    platform::start(app);
}

#[allow(dead_code)] // unused on platforms without a watcher
fn publish(app: &AppHandle, next: OsAppearance) {
    let state = app.state::<AppearanceState>();
    let changed = match state.0.lock() {
        Ok(mut cur) if *cur != next => {
            *cur = next.clone();
            true
        }
        _ => false,
    };
    if changed {
        let _ = app.emit(EVENT, next);
    }
}

/// Portal `color-scheme`: 0 = no preference, 1 = prefer dark, 2 = prefer light.
#[allow(dead_code)]
pub fn scheme_from_portal(value: u32) -> Option<&'static str> {
    match value {
        1 => Some("dark"),
        2 => Some("light"),
        _ => None,
    }
}

/// Portal `accent-color`: RGB doubles in [0, 1]; anything out of range means "unset".
#[allow(dead_code)]
pub fn accent_from_portal(r: f64, g: f64, b: f64) -> Option<String> {
    let ok = |c: f64| (0.0..=1.0).contains(&c);
    if !(ok(r) && ok(g) && ok(b)) {
        return None;
    }
    let to8 = |c: f64| (c * 255.0).round() as u8;
    Some(format!("#{:02x}{:02x}{:02x}", to8(r), to8(g), to8(b)))
}

/// Windows stores `DWM\AccentColor` as 0xAABBGGRR.
#[allow(dead_code)]
pub fn accent_from_abgr(abgr: u32) -> String {
    let r = abgr & 0xff;
    let g = (abgr >> 8) & 0xff;
    let b = (abgr >> 16) & 0xff;
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{OsAppearance, accent_from_portal, publish, scheme_from_portal};
    use tauri::AppHandle;
    use zbus::blocking::{Connection, Proxy};
    use zbus::zvariant::{OwnedValue, Value};

    const DEST: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const IFACE: &str = "org.freedesktop.portal.Settings";
    const NS: &str = "org.freedesktop.appearance";

    pub fn start(app: AppHandle) {
        let _ = std::thread::Builder::new()
            .name("portal-appearance".into())
            .spawn(move || {
                if let Err(err) = run(&app) {
                    eprintln!("[appearance] freedesktop portal unavailable: {err}");
                }
            });
    }

    fn run(app: &AppHandle) -> zbus::Result<()> {
        let conn = Connection::session()?;
        let proxy = Proxy::new(&conn, DEST, PATH, IFACE)?;
        // Subscribe before the initial read so no change can slip between them.
        let signals = proxy.receive_signal("SettingChanged")?;
        let mut current = OsAppearance {
            source: "portal",
            ..Default::default()
        };
        if let Some(v) = read(&proxy, "color-scheme") {
            current.color_scheme = parse_scheme(&v);
        }
        if let Some(v) = read(&proxy, "accent-color") {
            current.accent = parse_accent(&v);
        }
        publish(app, current.clone());

        for msg in signals {
            let Ok((ns, key, value)) = msg.body().deserialize::<(String, String, OwnedValue)>()
            else {
                continue;
            };
            if ns != NS {
                continue;
            }
            match key.as_str() {
                "color-scheme" => current.color_scheme = parse_scheme(&value),
                "accent-color" => current.accent = parse_accent(&value),
                _ => continue,
            }
            publish(app, current.clone());
        }
        Ok(())
    }

    fn read(proxy: &Proxy<'_>, key: &str) -> Option<OwnedValue> {
        // Portal v2 has ReadOne; older portals only have the deprecated Read,
        // which wraps the value in an extra variant (handled by `unwrap`).
        proxy
            .call::<_, _, OwnedValue>("ReadOne", &(NS, key))
            .or_else(|_| proxy.call::<_, _, OwnedValue>("Read", &(NS, key)))
            .ok()
    }

    fn unwrap<'a>(v: &'a Value<'a>) -> &'a Value<'a> {
        match v {
            Value::Value(inner) => unwrap(inner),
            other => other,
        }
    }

    fn parse_scheme(v: &Value<'_>) -> Option<&'static str> {
        match unwrap(v) {
            Value::U32(n) => scheme_from_portal(*n),
            _ => None,
        }
    }

    fn parse_accent(v: &Value<'_>) -> Option<String> {
        let Value::Structure(s) = unwrap(v) else {
            return None;
        };
        match s.fields() {
            [Value::F64(r), Value::F64(g), Value::F64(b)] => accent_from_portal(*r, *g, *b),
            _ => None,
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::{OsAppearance, accent_from_abgr, publish};
    use std::time::Duration;
    use tauri::AppHandle;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    pub fn start(app: AppHandle) {
        let _ = std::thread::Builder::new()
            .name("registry-appearance".into())
            .spawn(move || {
                loop {
                    publish(&app, read());
                    std::thread::sleep(Duration::from_secs(3));
                }
            });
    }

    fn read() -> OsAppearance {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let color_scheme = hkcu
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
            .ok()
            .map(|v| if v == 0 { "dark" } else { "light" });
        let accent = hkcu
            .open_subkey(r"Software\Microsoft\Windows\DWM")
            .and_then(|k| k.get_value::<u32, _>("AccentColor"))
            .ok()
            .map(accent_from_abgr);
        OsAppearance {
            color_scheme,
            accent,
            source: "registry",
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod platform {
    pub fn start(_app: tauri::AppHandle) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portal_scheme_values() {
        assert_eq!(scheme_from_portal(0), None);
        assert_eq!(scheme_from_portal(1), Some("dark"));
        assert_eq!(scheme_from_portal(2), Some("light"));
        assert_eq!(scheme_from_portal(7), None);
    }

    #[test]
    fn portal_accent_values() {
        assert_eq!(
            accent_from_portal(1.0, 0.5, 0.0).as_deref(),
            Some("#ff8000")
        );
        assert_eq!(accent_from_portal(-1.0, 0.5, 0.0), None);
        assert_eq!(accent_from_portal(0.2, 1.5, 0.0), None);
    }

    #[test]
    fn windows_abgr_accent() {
        // 0xAABBGGRR: R=0x00, G=0x78, B=0xd4 (Windows default blue)
        assert_eq!(accent_from_abgr(0xffd47800), "#0078d4");
    }
}
