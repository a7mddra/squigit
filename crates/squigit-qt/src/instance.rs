use crate::paths::RuntimePaths;
#[cfg(target_os = "linux")]
use crate::sandbox::audio_guard::AudioGuard;
use crate::sandbox::display_hotplug::DisplayWatcher;
use crate::sandbox::single_instance::InstanceLock;
use crate::CaptureOutcome;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

static ACTIVE_PID: AtomicU32 = AtomicU32::new(0);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub fn cancel_active() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    let pid = ACTIVE_PID.load(Ordering::Acquire);
    if pid != 0 {
        Instance::kill_process(pid);
    }
}

#[derive(Default)]
struct OcrJobs {
    cancelled: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl Drop for OcrJobs {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

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

    pub fn capture(mode: String, auto_ocr: bool) -> Self {
        let mut args = vec![mode];
        if auto_ocr {
            args.push("--auto-ocr".into());
        }
        Self {
            args,
            output_dir: None,
        }
    }

    pub fn with_output_dir(mut self, output_dir: Option<String>) -> Self {
        self.output_dir = output_dir;
        self
    }

    pub fn run(&mut self) -> Result<CaptureOutcome> {
        if SHUTTING_DOWN.load(Ordering::Acquire) {
            return Ok(CaptureOutcome::exit(ExitCode::from(2)));
        }
        let _lock = InstanceLock::try_acquire()
            .context("Failed to acquire instance lock - is another squigit running?")?;

        let mut child = self.spawn_process()?;
        let child_pid = child.id();
        ACTIVE_PID.store(child_pid, Ordering::Release);
        if SHUTTING_DOWN.load(Ordering::Acquire) {
            let _ = child.kill();
        }

        let watcher = DisplayWatcher::start(move || {
            Self::kill_process(child_pid);
        });

        let exit_code = self.handle_ipc(&mut child);

        if exit_code.is_err() {
            let _ = child.kill();
        }
        watcher.stop();
        let _ = child.wait();
        ACTIVE_PID.store(0, Ordering::Release);
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
            .stdin(Stdio::piped())
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

    fn handle_ipc(&self, child: &mut Child) -> Result<CaptureOutcome> {
        let stdout = child
            .stdout
            .take()
            .context("The squigit Qt runtime did not expose stdout")?;
        let stdin = Arc::new(Mutex::new(child.stdin.take().context("Missing Qt stdin")?));
        let mut jobs = OcrJobs::default();
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
                    if let Some(payload) = trimmed.strip_prefix("OCR_REQUEST:") {
                        let request: serde_json::Value =
                            serde_json::from_str(payload).context("Invalid Qt OCR request")?;
                        let id = request["id"]
                            .as_str()
                            .context("Missing OCR request id")?
                            .to_owned();
                        let image = std::path::PathBuf::from(
                            request["image"].as_str().context("Missing OCR image")?,
                        );
                        let input = Arc::clone(&stdin);
                        let cancelled = Arc::clone(&jobs.cancelled);
                        jobs.workers.push(std::thread::spawn(move || {
                            let response = match squigit_ocr::recognize(&image, &cancelled) {
                                Ok(boxes) => serde_json::json!({"id": id, "boxes": boxes}),
                                Err(error) => {
                                    serde_json::json!({"id": id, "error": format!("{error:#}")})
                                }
                            };
                            if let Ok(mut input) = input.lock() {
                                let _ = writeln!(input, "{response}");
                            }
                        }));
                    }
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
            Ok(CaptureOutcome::exit(ExitCode::SUCCESS))
        } else if capture_authorized {
            println!("CAPTURE_AUTHORIZED");
            Ok(CaptureOutcome::exit(ExitCode::SUCCESS))
        } else if let Some(path) = capture_saved {
            println!("{path}");
            Ok(CaptureOutcome {
                exit_code: ExitCode::SUCCESS,
                saved_path: Some(path.into()),
            })
        } else {
            if terminal_exit_code.is_none() {
                eprintln!(
                    "[squigit] The Qt runtime exited without terminal protocol output. \
This usually indicates a native crash during startup or capture."
                );
            }
            Ok(CaptureOutcome::exit(
                terminal_exit_code.unwrap_or(ExitCode::FAILURE),
            ))
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
