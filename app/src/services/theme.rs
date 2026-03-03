// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::process::Command;

#[cfg(target_os = "linux")]
use zbus::blocking::Connection as BlockingConnection;

/// Detects the current system theme using a robust cross-platform strategy.
/// Returns "dark" or "light".
pub fn get_system_theme() -> String {
    #[cfg(target_os = "linux")]
    {
        get_linux_theme()
    }
    #[cfg(target_os = "windows")]
    {
        get_windows_theme()
    }
    #[cfg(target_os = "macos")]
    {
        get_macos_theme()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        "light".to_string()
    }
}

// ==========================
// Linux Implementation
// ==========================

#[cfg(target_os = "linux")]
fn get_linux_theme() -> String {
    // Priority 1: XDG Desktop Portal (Universal)
    if let Some(theme) = check_portal() {
        return theme;
    }

    // Priority 2: DE-specific Fallbacks
    if let Some(theme) = check_gsettings() {
        return theme;
    }
    if let Some(theme) = check_kreadconfig() {
        return theme;
    }

    // Default
    "light".to_string()
}

#[cfg(target_os = "linux")]
fn check_portal() -> Option<String> {
    let connection = BlockingConnection::session().ok()?;

    let reply = connection.call_method(
        Some("org.freedesktop.portal.Desktop"),
        "/org/freedesktop/portal/desktop",
        Some("org.freedesktop.portal.Settings"),
        "Read",
        &("org.freedesktop.appearance", "color-scheme"),
    );

    match reply {
        Ok(msg) => {
            if let Ok((val,)) = msg.body().deserialize::<(zbus::zvariant::OwnedValue,)>() {
                if let Ok(u) = val.downcast_ref::<u32>() {
                    return match u {
                        1 => Some("dark".to_string()),
                        2 => Some("light".to_string()),
                        _ => None,
                    };
                }

                if let Ok(inner) = val.downcast_ref::<zbus::zvariant::Value>() {
                    if let Ok(u) = inner.downcast_ref::<u32>() {
                        return match u {
                            1 => Some("dark".to_string()),
                            2 => Some("light".to_string()),
                            _ => None,
                        };
                    }
                }
            }
            None
        }
        Err(_) => None,
    }
}

#[cfg(target_os = "linux")]
fn check_gsettings() -> Option<String> {
    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let raw_theme = stdout.trim_matches('\'').trim_matches('"');
        if is_dark_heuristic(raw_theme) {
            return Some("dark".to_string());
        }
    }

    let output = Command::new("gsettings")
        .args(&["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let raw = stdout.trim_matches('\'').trim_matches('"');
        if raw.contains("dark") {
            return Some("dark".to_string());
        }
        if raw == "default" || raw.contains("light") {
            return Some("light".to_string());
        }
    }

    None
}

#[cfg(target_os = "linux")]
fn check_kreadconfig() -> Option<String> {
    let output = Command::new("kreadconfig5")
        .args(&["--group", "General", "--key", "ColorScheme"])
        .output()
        .ok()
        .or_else(|| {
            Command::new("kreadconfig6")
                .args(&["--group", "General", "--key", "ColorScheme"])
                .output()
                .ok()
        })?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if is_dark_heuristic(&stdout) {
            return Some("dark".to_string());
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn is_dark_heuristic(theme_name: &str) -> bool {
    let lower = theme_name.to_lowercase();
    lower.contains("dark")
        || lower.contains("black")
        || lower.contains("night")
        || lower.contains("noir")
        || lower.contains("shadow")
}

// ==========================
// Windows Implementation
// ==========================

#[cfg(target_os = "windows")]
fn get_windows_theme() -> String {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize";

    if let Ok(key) = hkcu.open_subkey(path) {
        if let Ok(val) = key.get_value::<u32, _>("AppsUseLightTheme") {
            return if val == 0 {
                "dark".to_string()
            } else {
                "light".to_string()
            };
        }
    }
    "light".to_string()
}

// ==========================
// macOS Implementation
// ==========================

#[cfg(target_os = "macos")]
fn get_macos_theme() -> String {
    let output = Command::new("defaults")
        .args(&["read", "-g", "AppleInterfaceStyle"])
        .output();

    if let Ok(o) = output {
        let stdout = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if stdout == "Dark" {
            return "dark".to_string();
        }
    }
    "light".to_string()
}
