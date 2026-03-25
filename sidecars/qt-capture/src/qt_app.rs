// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use std::env;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitCode, Stdio};
use sys_display_hotplug::DisplayWatcher;
use sys_shutter_suppressor::AudioGuard;
use sys_single_instance::InstanceLock;

use ops_chat_storage::{ChatData, ChatMetadata, ChatStorage};
use ops_profile_store::ProfileStore;

use crate::paths::QtPaths;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct QtApp {
    args: Vec<String>,
    input_only: bool,
}

impl QtApp {
    fn is_capture_path_line(line: &str) -> bool {
        if line.starts_with('/') || line.starts_with("\\\\") {
            return true;
        }

        let bytes = line.as_bytes();
        bytes.len() > 2
            && bytes[1] == b':'
            && (bytes[2] == b'/' || bytes[2] == b'\\')
            && bytes[0].is_ascii_alphabetic()
    }

    pub fn new() -> Self {
        let mut args: Vec<String> = env::args().skip(1).collect();
        let input_only = args.contains(&"--input-only".to_string());
        args.retain(|a| a != "--input-only");
        Self { args, input_only }
    }

    pub fn run(&mut self) -> Result<ExitCode> {
        let _lock = InstanceLock::try_acquire("qt-capture")
            .context("Failed to acquire instance lock - is another capture running?")?;

        AudioGuard::mute();

        let mut child = self.spawn_process()?;
        let child_pid = child.id();

        let watcher = DisplayWatcher::start(move || {
            Self::kill_process(child_pid);
        });

        let exit_code = self.handle_ipc(&mut child);

        watcher.stop();
        let _ = child.wait();
        AudioGuard::unmute();

        Ok(exit_code)
    }

    fn spawn_process(&self) -> Result<Child> {
        let paths = QtPaths::resolve()?;
        let mut cmd = Command::new(&paths.bin);

        cmd.args(&self.args)
            .env("GIO_LAUNCHED_DESKTOP_APP_ID", "squigit")
            .env("G_APPLICATION_ID", "squigit")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        for (key, val) in paths.env_vars {
            cmd.env(key, val);
        }

        cmd.spawn().context("Failed to spawn Qt binary")
    }

    fn handle_ipc(&self, child: &mut Child) -> ExitCode {
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut capture_success = false;
            let mut saw_terminal_signal = false;
            let mut capture_path: Option<String> = None;
            let mut image_hash: Option<String> = None;
            let mut display_geo: Option<String> = None;

            for line in reader.lines() {
                match line {
                    Ok(msg) => {
                        let trimmed = msg.trim();
                        if let Some(geo) = trimmed.strip_prefix("DISPLAY_GEO:") {
                            display_geo = Some(geo.to_string());
                            continue;
                        }
                        match trimmed {
                            "REQ_MUTE" => {}
                            "CAPTURE_SUCCESS" => {
                                capture_success = true;
                            }
                            "CAPTURE_FAIL" => {
                                saw_terminal_signal = true;
                                break;
                            }
                            "CAPTURE_DENIED" => {
                                saw_terminal_signal = true;
                                println!("CAPTURE_DENIED");
                                eprintln!("\n============================================================");
                                eprintln!("Screen Recording Permission Denied");
                                eprintln!(
                                    "============================================================"
                                );
                                eprintln!("Your Wayland compositor or Desktop Portal rejected the request.");
                                eprintln!(
                                    "If this happened automatically without a prompt, check your"
                                );
                                eprintln!("screen capture portal configurations (e.g. xdg-desktop-portal-hyprland/wlr).");
                                eprintln!("\nFor help, please report an issue at:");
                                eprintln!("-> https://github.com/a7mddra/squigit/issues/new");
                                eprintln!("============================================================\n");
                                break;
                            }
                            _ => {
                                if capture_success && Self::is_capture_path_line(trimmed) {
                                    if self.input_only {
                                        let (path, hash) = self.process_capture_input_only(trimmed);
                                        if let Some(p) = path {
                                            capture_path = Some(p);
                                            image_hash = hash;
                                        }
                                    } else {
                                        let (path, hash) = self.process_capture(trimmed);
                                        if let Some(p) = path {
                                            capture_path = Some(p);
                                            image_hash = hash;
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                    Err(_) => break,
                }
            }

            if let Some(res) = capture_path {
                if self.input_only {
                    println!("CAS_PATH:{}", res);
                } else {
                    println!("CHAT_ID:{}", res);
                    if let Some(hash) = image_hash {
                        println!("IMAGE_HASH:{}", hash);
                    }
                }
                if let Some(geo) = display_geo {
                    println!("DISPLAY_GEO:{}", geo);
                }
                ExitCode::from(0)
            } else {
                if !saw_terminal_signal {
                    eprintln!(
                        "[qt-capture] Native capture process exited without CAPTURE_* protocol output. \
This usually indicates a native crash during startup/capture."
                    );
                }
                ExitCode::from(1)
            }
        } else {
            ExitCode::from(1)
        }
    }

    fn process_capture(&self, path: &str) -> (Option<String>, Option<String>) {
        ProfileStore::new()
            .ok()
            .and_then(|profile_store| {
                profile_store
                    .get_active_profile_id()
                    .ok()
                    .flatten()
                    .map(|active_id| (profile_store, active_id))
            })
            .and_then(|(profile_store, active_id)| {
                let chats_dir = profile_store.get_chats_dir(&active_id);
                ChatStorage::with_base_dir(chats_dir).ok()
            })
            .and_then(|storage| {
                storage
                    .store_image_from_path(path, None)
                    .ok()
                    .map(|stored| (storage, stored))
            })
            .map(|(storage, stored)| {
                let metadata =
                    ChatMetadata::new("New thread".to_string(), stored.hash.clone(), None);
                let chat = ChatData::new(metadata.clone());
                let _ = storage.save_chat(&chat);
                let _ = std::fs::remove_file(path);
                (Some(metadata.id), Some(stored.hash))
            })
            .unwrap_or_else(|| (Some(path.to_string()), None))
    }

    fn process_capture_input_only(&self, path: &str) -> (Option<String>, Option<String>) {
        ProfileStore::new()
            .ok()
            .and_then(|profile_store| {
                profile_store
                    .get_active_profile_id()
                    .ok()
                    .flatten()
                    .map(|active_id| (profile_store, active_id))
            })
            .and_then(|(profile_store, active_id)| {
                let chats_dir = profile_store.get_chats_dir(&active_id);
                ChatStorage::with_base_dir(chats_dir).ok()
            })
            .and_then(|storage| storage.store_image_from_path(path, None).ok())
            .map(|stored| {
                let _ = std::fs::remove_file(path);
                (Some(stored.path), Some(stored.hash))
            })
            .unwrap_or_else(|| (Some(path.to_string()), None))
    }

    fn kill_process(pid: u32) {
        #[cfg(unix)]
        {
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).output();
        }
        #[cfg(windows)]
        {
            let mut cmd = Command::new("taskkill");
            let _ = cmd
                .creation_flags(CREATE_NO_WINDOW)
                .args(["/F", "/PID", &pid.to_string()])
                .output();
        }
    }
}
