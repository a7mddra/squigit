use std::env;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static HAS_WPCTL: OnceLock<bool> = OnceLock::new();
static HAS_PACTL: OnceLock<bool> = OnceLock::new();
static AUDIO_STATE: OnceLock<Mutex<AudioState>> = OnceLock::new();

#[derive(Default)]
struct AudioState {
    depth: usize,
    session: Option<SuppressionSession>,
}

enum SuppressionSession {
    Disabled,
    Managed {
        backend: Backend,
        changed_by_us: bool,
    },
}

#[derive(Clone, Copy)]
enum Backend {
    Wpctl,
    Pactl,
    Amixer,
}

pub struct AudioGuard;

impl AudioGuard {
    #[inline]
    pub fn mute() {
        if Self::is_wayland() {
            Self::mute_scoped();
        }
    }

    #[inline]
    pub fn unmute() {
        if Self::is_wayland() {
            Self::unmute_scoped();
        }
    }

    fn state() -> &'static Mutex<AudioState> {
        AUDIO_STATE.get_or_init(|| Mutex::new(AudioState::default()))
    }

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
                    eprintln!("[squigit] Audio suppression disabled: failed to mute output device");
                    SuppressionSession::Disabled
                }
            }
            Err(reason) => {
                eprintln!("[squigit] Audio suppression disabled: {}", reason);
                SuppressionSession::Disabled
            }
        });
    }

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
                eprintln!("[squigit] Failed to restore previous audio mute state");
            }
        }
    }

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

    fn query_amixer_mute_state() -> Option<bool> {
        let output = Command::new("amixer")
            .args(["sget", "Master"])
            .output()
            .ok()?;

        let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
        if text.contains("[off]") {
            Some(true)
        } else if text.contains("[on]") {
            Some(false)
        } else {
            None
        }
    }

    fn is_wayland() -> bool {
        env::var("XDG_SESSION_TYPE")
            .map(|v| v.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
    }

    fn has_cmd(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
