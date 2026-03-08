// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::ShortcutConfig;
use std::ffi::OsString;
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use zbus::interface;

const DBUS_DESTINATION: &str = "com.squigit.app";
const DBUS_PATH: &str = "/com/squigit/app";
const DBUS_METHOD_FULL: &str = "com.squigit.app.Capture";
const DBUS_INTERFACE: &str = "com.squigit.app";
const DBUS_METHOD_NAME: &str = "Capture";
const HOTKEY_SCRIPT_NAME: &str = "hotkey-trigger.sh";

pub(crate) struct LinuxHandle {
    shutdown: Arc<AtomicBool>,

    _thread: std::thread::JoinHandle<()>,
}

pub fn trigger_linux_ipc() -> bool {
    try_dbus_send() || try_busctl() || try_gdbus()
}

pub fn install_linux_shortcut(_bin_path: &str, trigger: &str, name: &str) -> Result<(), String> {
    let de = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    let trigger_gnome = format!("<Super><Shift>{}", trigger.split('+').last().unwrap_or("a"));
    let trigger_kde = format!(
        "Meta+Shift+{}",
        trigger.split('+').last().unwrap_or("A").to_uppercase()
    );

    let app_lower = "Squigit".to_lowercase();
    let command_str = build_shortcut_command()?;
    let command = command_str.as_str();

    if de.contains("gnome")
        || de.contains("ubuntu")
        || de.contains("unity")
        || de.contains("budgie")
    {
        let binding_path_str = format!(
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/{}-binding/",
            app_lower
        );
        let binding_path = binding_path_str.as_str();
        let schema = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

        let get_bindings = Command::new("gsettings")
            .arg("get")
            .arg("org.gnome.settings-daemon.plugins.media-keys")
            .arg("custom-keybindings")
            .output();

        if let Ok(output) = get_bindings {
            let mut list = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if list == "@as []" {
                list = "[]".to_string();
            }
            if !list.contains(binding_path) {
                let new_list = if list == "[]" {
                    format!("['{}']", binding_path)
                } else {
                    format!("{}, '{}']", list.trim_end_matches(']'), binding_path)
                };
                let _ = Command::new("gsettings")
                    .args(&[
                        "set",
                        "org.gnome.settings-daemon.plugins.media-keys",
                        "custom-keybindings",
                        &new_list,
                    ])
                    .output();
            }

            let _ = Command::new("gsettings")
                .args(&["set", &format!("{}:{}", schema, binding_path), "name", name])
                .output();
            let _ = Command::new("gsettings")
                .args(&[
                    "set",
                    &format!("{}:{}", schema, binding_path),
                    "command",
                    command,
                ])
                .output();
            let _ = Command::new("gsettings")
                .args(&[
                    "set",
                    &format!("{}:{}", schema, binding_path),
                    "binding",
                    &trigger_gnome,
                ])
                .output();
            return Ok(());
        }
        return Err("Failed to setup GNOME bindings via gsettings".into());
    } else if de.contains("kde") || de.contains("plasma") {
        let uuid_str = format!("12345678-1234-5678-1234-{:012x}", std::process::id());
        let import_file = format!("/tmp/{}-binding.khotkeys", app_lower);
        let content = format!(
            "[Data]
DataCount=1
[Data_1]
Comment={name}
Enabled=true
Name={name}
Type=SIMPLE_ACTION_DATA
[Data_1Actions]
ActionsCount=1
[Data_1Actions0]
CommandURL={command}
Type=COMMAND_URL
[Data_1Triggers]
TriggersCount=1
[Data_1Triggers0]
Key={trigger_kde}
Type=SHORTCUT
Uuid={{{uuid_str}}}"
        );
        std::fs::write(&import_file, content).map_err(|e| e.to_string())?;

        let _ = Command::new("qdbus")
            .args(&[
                "org.kde.kded5",
                "/modules/khotkeys",
                "org.kde.khotkeys.import_shortcuts_list",
                &import_file,
            ])
            .output();
        let _ = Command::new("qdbus")
            .args(&[
                "org.kde.kglobalaccel",
                "/kglobalaccel",
                "org.kde.kglobalaccel.Component.importLegacyShortcuts",
                &import_file,
            ])
            .output();
        return Ok(());
    } else if de.contains("xfce") {
        let _ = Command::new("xfconf-query")
            .args(&[
                "--channel",
                "xfce4-keyboard-shortcuts",
                "--property",
                &format!("/commands/custom/{}", trigger_gnome),
                "--create",
                "--type",
                "string",
                "--set",
                command,
            ])
            .output();
        return Ok(());
    }

    Err(format!(
        "Unsupported Desktop Environment: {}, please configure shortcut manually",
        de
    )
    .into())
}

fn build_shortcut_command() -> Result<String, String> {
    let script_path = ensure_hotkey_trigger_script()?;
    Ok(format!("/bin/sh {}", script_path))
}

fn ensure_hotkey_trigger_script() -> Result<String, String> {
    let path = hotkey_script_path();

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Failed to create shortcut helper directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let script = format!(
        r#"#!/bin/sh
# Auto-generated by Squigit. Triggered from desktop shortcut daemons.
if [ -z "${{DBUS_SESSION_BUS_ADDRESS:-}}" ] && [ -n "${{XDG_RUNTIME_DIR:-}}" ] && [ -S "${{XDG_RUNTIME_DIR}}/bus" ]; then
  export DBUS_SESSION_BUS_ADDRESS="unix:path=${{XDG_RUNTIME_DIR}}/bus"
fi

if command -v dbus-send >/dev/null 2>&1; then
  dbus-send --session --type=method_call --dest={dest} {path} {method_full} >/dev/null 2>&1 && exit 0
fi

if command -v busctl >/dev/null 2>&1; then
  busctl --user call {dest} {path} {iface} {method} >/dev/null 2>&1 && exit 0
fi

if command -v gdbus >/dev/null 2>&1; then
  gdbus call --session --dest {dest} --object-path {path} --method {method_full} >/dev/null 2>&1 && exit 0
fi

exit 1
"#,
        dest = DBUS_DESTINATION,
        path = DBUS_PATH,
        method_full = DBUS_METHOD_FULL,
        iface = DBUS_INTERFACE,
        method = DBUS_METHOD_NAME,
    );

    std::fs::write(&path, script).map_err(|e| {
        format!(
            "Failed to write shortcut helper script {}: {}",
            path.display(),
            e
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)
            .map_err(|e| format!("Failed to stat shortcut helper script: {}", e))?
            .permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(&path, perms)
            .map_err(|e| format!("Failed to chmod shortcut helper script: {}", e))?;
    }

    Ok(path.to_string_lossy().to_string())
}

fn hotkey_script_path() -> std::path::PathBuf {
    let base = xdg_config_home().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    base.join("squigit").join(HOTKEY_SCRIPT_NAME)
}

fn xdg_config_home() -> Option<std::path::PathBuf> {
    let xdg_home = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from);
    if xdg_home.is_some() {
        return xdg_home;
    }
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(|home| std::path::PathBuf::from(home).join(".config"))
}

fn try_dbus_send() -> bool {
    run_capture_command(
        "dbus-send",
        &[
            "--session",
            "--type=method_call",
            &format!("--dest={}", DBUS_DESTINATION),
            DBUS_PATH,
            DBUS_METHOD_FULL,
        ],
    )
}

fn try_busctl() -> bool {
    run_capture_command(
        "busctl",
        &[
            "--user",
            "call",
            DBUS_DESTINATION,
            DBUS_PATH,
            DBUS_INTERFACE,
            DBUS_METHOD_NAME,
        ],
    )
}

fn try_gdbus() -> bool {
    run_capture_command(
        "gdbus",
        &[
            "call",
            "--session",
            "--dest",
            DBUS_DESTINATION,
            "--object-path",
            DBUS_PATH,
            "--method",
            DBUS_METHOD_FULL,
        ],
    )
}

fn run_capture_command(program: &str, args: &[&str]) -> bool {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none() {
        if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()) {
            let mut addr = OsString::from("unix:path=");
            addr.push(runtime_dir);
            addr.push("/bus");
            cmd.env("DBUS_SESSION_BUS_ADDRESS", addr);
        }
    }
    cmd.output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

struct AppDbus {
    callback: Arc<dyn Fn() + Send + Sync + 'static>,
}

#[interface(name = "com.squigit.app")]
impl AppDbus {
    async fn capture(&self) {
        (self.callback)();
    }
}

impl LinuxHandle {
    pub fn register<F>(_config: ShortcutConfig, callback: F) -> Result<Self, String>
    where
        F: Fn() + Send + Sync + 'static,
    {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();
        let cb: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);

        let thread = std::thread::Builder::new()
            .name("dbus-listener".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(_) => return,
                };

                rt.block_on(async {
                    let dbus_service = AppDbus { callback: cb };
                    let app_lower = "Squigit".to_lowercase();

                    let _conn = match zbus::connection::Builder::session() {
                        Ok(builder) => match builder
                            .name(format!("com.{}.app", app_lower))
                            .expect("Invalid D-Bus name")
                            .serve_at(format!("/com/{}/app", app_lower), dbus_service)
                            .expect("Failed to serve dbus interface")
                            .build()
                            .await
                        {
                            Ok(c) => c,
                            Err(_) => return,
                        },
                        Err(_) => return,
                    };

                    loop {
                        if shutdown_clone.load(Ordering::SeqCst) {
                            break;
                        }

                        let _ = tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                    }
                });
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            shutdown,
            _thread: thread,
        })
    }

    pub fn unregister(self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}
