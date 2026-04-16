// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Local shutter sound suppressor for Qt capture sidecar.
//!
//! Suppresses shutter sounds around capture events on:
//! - macOS: CoreGraphics always plays a sound
//! - Linux/Wayland: Portal may play a sound
//!
//! Windows and X11 are silent by default, so no action needed.

#[cfg(target_os = "linux")]
use std::env;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::process::Command;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::sync::Mutex;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
static HAS_WPCTL: OnceLock<bool> = OnceLock::new();
#[cfg(target_os = "linux")]
static HAS_PACTL: OnceLock<bool> = OnceLock::new();
#[cfg(any(target_os = "macos", target_os = "linux"))]
static AUDIO_STATE: OnceLock<Mutex<AudioState>> = OnceLock::new();

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[derive(Default)]
struct AudioState {
    depth: usize,
    session: Option<SuppressionSession>,
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
enum SuppressionSession {
    Disabled,
    Managed {
        backend: Backend,
        changed_by_us: bool,
    },
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum Backend {
    Wpctl,
    Pactl,
    Amixer,
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum Backend {
    OsaScript,
}

pub struct AudioGuard;

impl AudioGuard {
    #[inline]
    pub fn mute() {
        #[cfg(target_os = "macos")]
        Self::mute_scoped();

        #[cfg(target_os = "linux")]
        if Self::is_wayland() {
            Self::mute_scoped();
        }
    }

    #[inline]
    pub fn unmute() {
        #[cfg(target_os = "macos")]
        Self::unmute_scoped();

        #[cfg(target_os = "linux")]
        if Self::is_wayland() {
            Self::unmute_scoped();
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn state() -> &'static Mutex<AudioState> {
        AUDIO_STATE.get_or_init(|| Mutex::new(AudioState::default()))
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn mute_scoped() {
        let mut state = Self::state().lock().unwrap_or_else(|e| e.into_inner());
        state.depth = state.depth.saturating_add(1);
        if state.depth > 1 {
            return;
        }

        state.session = Some(match Self::detect_backend_and_state() {
            Ok((backend, already_muted)) => {
                if already_muted {
                    SuppressionSession::Managed {
                        backend,
                        changed_by_us: false,
                    }
                } else if Self::set_muted(backend, true) {
                    SuppressionSession::Managed {
                        backend,
                        changed_by_us: true,
                    }
                } else {
                    eprintln!(
                        "[qt-capture] Audio suppression disabled: failed to mute output device"
                    );
                    SuppressionSession::Disabled
                }
            }
            Err(reason) => {
                eprintln!("[qt-capture] Audio suppression disabled: {}", reason);
                SuppressionSession::Disabled
            }
        });
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn unmute_scoped() {
        let session = {
            let mut state = Self::state().lock().unwrap_or_else(|e| e.into_inner());
            if state.depth == 0 {
                return;
            }

            state.depth -= 1;
            if state.depth > 0 {
                return;
            }

            state.session.take()
        };

        if let Some(SuppressionSession::Managed {
            backend,
            changed_by_us: true,
        }) = session
        {
            if !Self::set_muted(backend, false) {
                eprintln!("[qt-capture] Failed to restore previous audio mute state");
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn detect_backend_and_state() -> Result<(Backend, bool), String> {
        Self::query_macos_mute_state()
            .map(|muted| (Backend::OsaScript, muted))
            .ok_or_else(|| "unable to query macOS mute state via osascript".to_string())
    }

    #[cfg(target_os = "macos")]
    fn query_macos_mute_state() -> Option<bool> {
        let output = Command::new("osascript")
            .args(["-e", "output muted of (get volume settings)"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        match String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn set_muted(_backend: Backend, muted: bool) -> bool {
        let script = if muted {
            "set volume with output muted"
        } else {
            "set volume without output muted"
        };

        Command::new("osascript")
            .args(["-e", script])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    fn detect_backend_and_state() -> Result<(Backend, bool), String> {
        if *HAS_WPCTL.get_or_init(|| Self::has_cmd("wpctl")) {
            return Self::query_wpctl_mute_state()
                .map(|muted| (Backend::Wpctl, muted))
                .ok_or_else(|| "unable to query mute state via wpctl".to_string());
        }

        if *HAS_PACTL.get_or_init(|| Self::has_cmd("pactl")) {
            return Self::query_pactl_mute_state()
                .map(|muted| (Backend::Pactl, muted))
                .ok_or_else(|| "unable to query mute state via pactl".to_string());
        }

        if let Some(muted) = Self::query_amixer_mute_state() {
            return Ok((Backend::Amixer, muted));
        }

        Err(
            "no supported mixer command found or mute state query failed (wpctl, pactl, amixer)"
                .to_string(),
        )
    }

    #[cfg(target_os = "linux")]
    fn set_muted(backend: Backend, muted: bool) -> bool {
        match backend {
            Backend::Wpctl => Command::new("wpctl")
                .args([
                    "set-mute",
                    "@DEFAULT_AUDIO_SINK@",
                    if muted { "1" } else { "0" },
                ])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Backend::Pactl => Command::new("pactl")
                .args([
                    "set-sink-mute",
                    "@DEFAULT_SINK@",
                    if muted { "1" } else { "0" },
                ])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Backend::Amixer => Command::new("amixer")
                .args([
                    "-q",
                    "sset",
                    "Master",
                    if muted { "mute" } else { "unmute" },
                ])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
        }
    }

    #[cfg(target_os = "linux")]
    fn query_wpctl_mute_state() -> Option<bool> {
        let output = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if text.contains("muted") {
            Some(true)
        } else if text.contains("volume:") {
            Some(false)
        } else {
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn query_pactl_mute_state() -> Option<bool> {
        let output = Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(value) = line.trim().to_ascii_lowercase().strip_prefix("mute:") {
                let status = value.trim();
                if status == "yes" {
                    return Some(true);
                }
                if status == "no" {
                    return Some(false);
                }
            }
        }

        None
    }

    #[cfg(target_os = "linux")]
    fn query_amixer_mute_state() -> Option<bool> {
        let output = Command::new("amixer")
            .args(["sget", "Master"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }

        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if text.contains("[off]") {
            Some(true)
        } else if text.contains("[on]") {
            Some(false)
        } else {
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn is_wayland() -> bool {
        env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
    }

    #[cfg(target_os = "linux")]
    fn has_cmd(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
