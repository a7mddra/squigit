use crate::paths::RuntimePaths;
#[cfg(target_os = "linux")]
use crate::sandbox::audio_guard::AudioGuard;
use crate::sandbox::display_hotplug::DisplayWatcher;
use crate::sandbox::single_instance::InstanceLock;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitCode, Stdio};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct Instance {
    args: Vec<String>,
    output_dir: Option<String>,
}

impl Instance {
    pub fn preflight() -> Self {
        Self {
            args: vec!["--preflight".to_string()],
            output_dir: None,
        }
    }

    pub fn authorize() -> Self {
        Self {
            args: vec!["--authorize".to_string()],
            output_dir: None,
        }
    }

    pub fn capture(mode: String) -> Self {
        Self {
            args: vec![mode],
            output_dir: None,
        }
    }

    pub fn with_output_dir(mut self, output_dir: Option<String>) -> Self {
        self.output_dir = output_dir;
        self
    }

    pub fn run(&mut self) -> Result<ExitCode> {
        let _lock = InstanceLock::try_acquire()
            .context("Failed to acquire instance lock - is another squigit running?")?;

        let mut child = self.spawn_process()?;
        let child_pid = child.id();

        let watcher = DisplayWatcher::start(move || {
            Self::kill_process(child_pid);
        });

        let exit_code = self.handle_ipc(&mut child);

        watcher.stop();
        let _ = child.wait();
        #[cfg(target_os = "linux")]
        AudioGuard::unmute();

        exit_code
    }

    fn spawn_process(&self) -> Result<Child> {
        let paths = RuntimePaths::resolve()?;
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

        if let Some(dir) = &self.output_dir {
            cmd.env("SQUIGIT_OUTPUT_DIR", dir);
        }

        cmd.spawn()
            .context("Failed to spawn the squigit Qt runtime")
    }

    fn handle_ipc(&self, child: &mut Child) -> Result<ExitCode> {
        let stdout = child
            .stdout
            .take()
            .context("The squigit Qt runtime did not expose stdout")?;
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut terminal_exit_code = None;
        let mut capture_authorized = false;
        let mut capture_permission = None;
        let mut capture_saved: Option<String> = None;

        loop {
            line.clear();
            if reader
                .read_line(&mut line)
                .context("Could not read the squigit Qt runtime output")?
                == 0
            {
                break;
            }
            let trimmed = line.trim();
            match trimmed {
                "AUDIO_MUTE" | "REQ_MUTE" => {
                    #[cfg(target_os = "linux")]
                    AudioGuard::mute();
                }
                "AUDIO_UNMUTE" => {
                    #[cfg(target_os = "linux")]
                    AudioGuard::unmute();
                }
                "CAPTURE_AUTHORIZED" => {
                    capture_authorized = true;
                    break;
                }
                "CAPTURE_PERMISSION_GRANTED" => {
                    capture_permission = Some(true);
                    break;
                }
                "CAPTURE_PERMISSION_DENIED" => {
                    capture_permission = Some(false);
                    break;
                }
                "CAPTURE_CANCELLED" => {
                    terminal_exit_code = Some(ExitCode::from(2));
                    break;
                }
                "CAPTURE_FAIL" => {
                    terminal_exit_code = Some(ExitCode::from(1));
                    break;
                }
                _ => {
                    if let Some(path) = trimmed.strip_prefix("SQUIGIT_SAVED:") {
                        capture_saved = Some(path.to_string());
                        break;
                    }
                }
            }
        }

        if let Some(granted) = capture_permission {
            println!(
                "{}",
                if granted {
                    "CAPTURE_PERMISSION_GRANTED"
                } else {
                    "CAPTURE_PERMISSION_DENIED"
                }
            );
            Ok(ExitCode::from(0))
        } else if capture_authorized {
            println!("CAPTURE_AUTHORIZED");
            Ok(ExitCode::from(0))
        } else if let Some(path) = capture_saved {
            println!("{path}");
            Ok(ExitCode::from(0))
        } else {
            if terminal_exit_code.is_none() {
                eprintln!(
                    "[squigit] The Qt runtime exited without terminal protocol output. \
This usually indicates a native crash during startup or capture."
                );
            }
            Ok(terminal_exit_code.unwrap_or(ExitCode::from(1)))
        }
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
