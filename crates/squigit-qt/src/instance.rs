use crate::paths::RuntimePaths;
use crate::qr;
#[cfg(target_os = "linux")]
use crate::sandbox::audio_guard::AudioGuard;
use crate::sandbox::display_hotplug::DisplayWatcher;
use crate::sandbox::single_instance::InstanceLock;
use crate::CaptureOutcome;
use anyhow::{Context, Result};
use std::collections::HashMap;
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

struct Job {
    cancelled: Arc<AtomicBool>,
    worker: JoinHandle<Result<(), String>>,
}

#[derive(Default)]
struct Jobs {
    workers: HashMap<String, Job>,
}

impl Jobs {
    fn cancel(&mut self, id: &str) -> Result<(), String> {
        if let Some(job) = self.workers.remove(id) {
            job.cancelled.store(true, Ordering::Release);
            job.worker
                .join()
                .map_err(|_| "Capture worker stopped unexpectedly".to_owned())??;
        }
        Ok(())
    }
}

impl Drop for Jobs {
    fn drop(&mut self) {
        for job in self.workers.values() {
            job.cancelled.store(true, Ordering::Release);
        }
        for (_, job) in self.workers.drain() {
            let _ = job.worker.join();
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
        let status = child.wait();
        ACTIVE_PID.store(0, Ordering::Release);
        #[cfg(target_os = "linux")]
        AudioGuard::unmute();

        let (outcome, qr_url) = exit_code?;
        status.context("Could not wait for the Qt capture overlay to close")?;
        if let Some(url) = qr_url {
            opener::open_browser(url.as_str())
                .context("Could not open the QR URL in the default browser")?;
        }
        Ok(outcome)
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

    fn handle_ipc(&self, child: &mut Child) -> Result<(CaptureOutcome, Option<url::Url>)> {
        let stdout = child
            .stdout
            .take()
            .context("The squigit Qt runtime did not expose stdout")?;
        let stdin = Arc::new(Mutex::new(child.stdin.take().context("Missing Qt stdin")?));
        let mut jobs = Jobs::default();
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut terminal_exit_code = None;
        let mut capture_authorized = false;
        let mut capture_permission = None;
        let mut capture_saved: Option<String> = None;
        let mut qr_url = None;

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
                        let cancelled = Arc::new(AtomicBool::new(false));
                        let cancel_job = Arc::clone(&cancelled);
                        let job_id = id.clone();
                        jobs.workers.retain(|_, job| !job.worker.is_finished());
                        let worker = std::thread::spawn(move || {
                            let response = match squigit_ocr::recognize(&image, &cancelled) {
                                Ok(squigit_ocr::Recognition::Text(boxes)) => {
                                    serde_json::json!({"id": id, "boxes": boxes})
                                }
                                Ok(squigit_ocr::Recognition::Cancelled) => return Ok(()),
                                Err(error) => {
                                    if cancelled.load(Ordering::Acquire) {
                                        return Err(format!("{error:#}"));
                                    }
                                    serde_json::json!({"id": id, "error": format!("{error:#}")})
                                }
                            };
                            if let Ok(mut input) = input.lock() {
                                let _ = writeln!(input, "{response}");
                            }
                            Ok(())
                        });
                        jobs.workers.insert(
                            job_id,
                            Job {
                                cancelled: cancel_job,
                                worker,
                            },
                        );
                    }
                    if let Some(payload) = trimmed.strip_prefix("QR_REQUEST:") {
                        let request: serde_json::Value =
                            serde_json::from_str(payload).context("Invalid Qt QR request")?;
                        let id = request["id"]
                            .as_str()
                            .context("Missing QR request id")?
                            .to_owned();
                        let image = std::path::PathBuf::from(
                            request["image"].as_str().context("Missing QR image")?,
                        );
                        let input = Arc::clone(&stdin);
                        let cancelled = Arc::new(AtomicBool::new(false));
                        let cancel_job = Arc::clone(&cancelled);
                        let job_id = id.clone();
                        jobs.workers.retain(|_, job| !job.worker.is_finished());
                        let worker = std::thread::spawn(move || {
                            let response = match qr::scan(&image, &cancelled) {
                                Ok(Some(url)) => serde_json::json!({"id": id, "url": url}),
                                Ok(None) => return Ok(()),
                                Err(error) => {
                                    serde_json::json!({"id": id, "error": format!("{error:#}")})
                                }
                            };
                            if !cancelled.load(Ordering::Acquire) {
                                if let Ok(mut input) = input.lock() {
                                    let _ = writeln!(input, "{response}");
                                }
                            }
                            Ok(())
                        });
                        jobs.workers.insert(
                            job_id,
                            Job {
                                cancelled: cancel_job,
                                worker,
                            },
                        );
                    }
                    if let Some(payload) = trimmed.strip_prefix("QR_OPEN:") {
                        let request: serde_json::Value =
                            serde_json::from_str(payload).context("Invalid Qt QR URL")?;
                        qr_url = Some(qr::parse_url(
                            request["url"].as_str().context("Missing QR URL")?,
                        )?);
                        terminal_exit_code = Some(ExitCode::SUCCESS);
                        break;
                    }
                    if let Some(payload) = trimmed.strip_prefix("OCR_CANCEL:") {
                        let request: serde_json::Value =
                            serde_json::from_str(payload).context("Invalid Qt OCR cancellation")?;
                        let id = request["id"]
                            .as_str()
                            .context("Missing OCR cancellation id")?;
                        let response = match jobs.cancel(id) {
                            Ok(()) => serde_json::json!({"id": id, "cancelled": true}),
                            Err(error) => {
                                serde_json::json!({"id": id, "cancelled": false, "error": error})
                            }
                        };
                        if let Ok(mut input) = stdin.lock() {
                            let _ = writeln!(input, "{response}");
                        }
                    }
                    if let Some(path) = trimmed.strip_prefix("SQUIGIT_SAVED:") {
                        capture_saved = Some(path.to_string());
                        break;
                    }
                }
            }
        }

        let outcome = if let Some(granted) = capture_permission {
            println!(
                "{}",
                if granted {
                    "CAPTURE_PERMISSION_GRANTED"
                } else {
                    "CAPTURE_PERMISSION_DENIED"
                }
            );
            CaptureOutcome::exit(ExitCode::SUCCESS)
        } else if capture_authorized {
            println!("CAPTURE_AUTHORIZED");
            CaptureOutcome::exit(ExitCode::SUCCESS)
        } else if let Some(path) = capture_saved {
            println!("{path}");
            CaptureOutcome {
                exit_code: ExitCode::SUCCESS,
                saved_path: Some(path.into()),
            }
        } else {
            if terminal_exit_code.is_none() {
                eprintln!(
                    "[squigit] The Qt runtime exited without terminal protocol output. \
This usually indicates a native crash during startup or capture."
                );
            }
            CaptureOutcome::exit(terminal_exit_code.unwrap_or(ExitCode::FAILURE))
        };
        Ok((outcome, qr_url))
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
