use crate::paths::RuntimePaths;
use crate::qr;
#[cfg(target_os = "linux")]
use crate::sandbox::audio_guard::AudioGuard;
use crate::sandbox::single_instance::InstanceLock;
use anyhow::{Context, Result};
use process_wrap::std::{ChildWrapper, CommandWrap};
use serde_json::{json, Value};
use squigit_ocr::models::ModelManager;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub enum RuntimeEvent {
    MenuTick,
    Saved { action: String, path: PathBuf },
    Exported { path: PathBuf },
    Asked { prompt: String, path: PathBuf },
    Failed(String),
    Stopped,
}

struct Inner {
    input: Mutex<Option<ChildStdin>>,
    child: Mutex<Box<dyn ChildWrapper>>,
    stopping: AtomicBool,
}

#[derive(Clone)]
pub struct Handle {
    inner: Arc<Inner>,
}

impl Handle {
    fn write(&self, message: &Value) -> Result<()> {
        let mut input = self
            .inner
            .input
            .lock()
            .map_err(|_| anyhow::anyhow!("Qt input is unavailable"))?;
        let Some(input) = input.as_mut() else {
            anyhow::ensure!(
                self.inner.stopping.load(Ordering::Acquire),
                "Qt input is closed"
            );
            return Ok(());
        };
        serde_json::to_writer(&mut *input, message)?;
        input.write_all(b"\n")?;
        input.flush()?;
        Ok(())
    }

    fn command(&self, name: &str) -> Result<()> {
        anyhow::ensure!(
            !self.inner.stopping.load(Ordering::Acquire),
            "The Qt runtime is stopping"
        );
        self.write(&json!({"type": name}))
    }

    pub fn show_menu(&self) -> Result<()> {
        self.command("show_menu")
    }

    pub fn hide_menu(&self) -> Result<()> {
        self.command("hide_menu")
    }

    pub fn shutdown(&self) -> Result<()> {
        if self.inner.stopping.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let _ = self.write(&json!({"type": "shutdown"}));
        self.inner
            .input
            .lock()
            .map_err(|_| anyhow::anyhow!("Qt input is unavailable"))?
            .take();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            let mut child = self
                .inner
                .child
                .lock()
                .map_err(|_| anyhow::anyhow!("Qt process is unavailable"))?;
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                child.kill().context("Could not stop the Qt process tree")?;
                child.wait().context("Could not reap the Qt process tree")?;
                return Ok(());
            }
            drop(child);
            thread::sleep(Duration::from_millis(25));
        }
    }
}

pub struct Runtime {
    handle: Handle,
    worker: Option<JoinHandle<()>>,
    _lock: InstanceLock,
}

impl Runtime {
    pub fn start(
        models: ModelManager,
        snapshots: PathBuf,
        blobs: PathBuf,
        on_event: impl Fn(RuntimeEvent) + Send + 'static,
    ) -> Result<Self> {
        let lock = InstanceLock::try_acquire()?;
        let paths = RuntimePaths::resolve()?;
        let mut command = Command::new(&paths.bin);
        command
            .env("GIO_LAUNCHED_DESKTOP_APP_ID", "squigit")
            .env("G_APPLICATION_ID", "squigit")
            .env("SQUIGIT_SNAPSHOT_DIR", snapshots)
            .env("SQUIGIT_BLOB_DIR", blobs)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        for (name, value) in paths.env_vars {
            command.env(name, value);
        }
        let mut command = CommandWrap::from(command);
        #[cfg(unix)]
        command.wrap(process_wrap::std::ProcessGroup::leader());
        #[cfg(windows)]
        command
            .wrap(process_wrap::std::CreationFlags(0x08000000))
            .wrap(process_wrap::std::JobObject);
        let mut child = command.spawn().context("Could not start the Qt runtime")?;
        let input = child.stdin().take().context("Missing Qt stdin")?;
        let output = child.stdout().take().context("Missing Qt stdout")?;
        let handle = Handle {
            inner: Arc::new(Inner {
                input: Mutex::new(Some(input)),
                child: Mutex::new(child),
                stopping: AtomicBool::new(false),
            }),
        };
        let reader_handle = handle.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            if let Err(error) = read_events(output, &reader_handle, models, ready_tx, &on_event) {
                on_event(RuntimeEvent::Failed(format!("{error:#}")));
            }
            #[cfg(target_os = "linux")]
            AudioGuard::unmute();
            if !reader_handle.inner.stopping.load(Ordering::Acquire) {
                on_event(RuntimeEvent::Stopped);
            }
            if let Err(error) = reader_handle.shutdown() {
                eprintln!("[squigit] {error:#}");
            }
        });
        let runtime = Self {
            handle,
            worker: Some(worker),
            _lock: lock,
        };
        ready_rx
            .recv_timeout(Duration::from_secs(15))
            .context("The Qt runtime did not become ready")?;
        Ok(runtime)
    }

    pub fn handle(&self) -> Handle {
        self.handle.clone()
    }

    pub fn shutdown(&mut self) {
        if let Err(error) = self.handle.shutdown() {
            eprintln!("[squigit] {error:#}");
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Job {
    cancelled: Arc<AtomicBool>,
    worker: JoinHandle<Result<()>>,
}

#[derive(Default)]
struct Jobs {
    workers: HashMap<String, Job>,
}

impl Jobs {
    fn cancel(&mut self, id: &str) -> Result<()> {
        if let Some(job) = self.workers.remove(id) {
            job.cancelled.store(true, Ordering::Release);
            job.worker
                .join()
                .map_err(|_| anyhow::anyhow!("Capture worker stopped unexpectedly"))??;
        }
        Ok(())
    }

    fn cancel_all(&mut self) {
        for job in self.workers.values() {
            job.cancelled.store(true, Ordering::Release);
        }
        for (_, job) in self.workers.drain() {
            match job.worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("[squigit] {error:#}"),
                Err(_) => eprintln!("[squigit] Capture worker stopped unexpectedly"),
            }
        }
    }

    fn reap(&mut self) {
        let finished = self
            .workers
            .iter()
            .filter(|(_, job)| job.worker.is_finished())
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in finished {
            if let Err(error) = self.cancel(&id) {
                eprintln!("[squigit] {error:#}");
            }
        }
    }
}

impl Drop for Jobs {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

fn read_events(
    output: ChildStdout,
    handle: &Handle,
    models: ModelManager,
    ready: mpsc::SyncSender<()>,
    on_event: &impl Fn(RuntimeEvent),
) -> Result<()> {
    let mut jobs = Jobs::default();
    for line in BufReader::new(output).lines() {
        let line = line.context("Could not read the Qt runtime output")?;
        let event: Value = serde_json::from_str(&line).context("Invalid Qt event")?;
        jobs.reap();
        match event["type"].as_str().context("Missing Qt event type")? {
            "ready" => {
                let _ = ready.try_send(());
            }
            "menu_tick" => on_event(RuntimeEvent::MenuTick),
            "audio_mute" => {
                #[cfg(target_os = "linux")]
                AudioGuard::mute();
            }
            "audio_unmute" => {
                #[cfg(target_os = "linux")]
                AudioGuard::unmute();
            }
            "ocr_request" | "qr_request" => {
                let id = event["id"]
                    .as_str()
                    .context("Missing capture request id")?
                    .to_owned();
                anyhow::ensure!(
                    !jobs.workers.contains_key(&id),
                    "Duplicate capture request id"
                );
                let image =
                    PathBuf::from(event["image"].as_str().context("Missing capture image")?);
                let ocr = event["type"] == "ocr_request";
                let input = handle.clone();
                let models = models.clone();
                let cancelled = Arc::new(AtomicBool::new(false));
                let cancel_job = cancelled.clone();
                let job_id = id.clone();
                let worker = thread::spawn(move || {
                    let response = if ocr {
                        match squigit_ocr::recognize(&image, &cancelled, &models) {
                            Ok(squigit_ocr::Recognition::Text(boxes)) => {
                                json!({"type": "ocr_result", "id": id, "boxes": boxes})
                            }
                            Ok(squigit_ocr::Recognition::Cancelled) => return Ok(()),
                            Err(error) => {
                                if cancelled.load(Ordering::Acquire) {
                                    return Err(error);
                                }
                                json!({"type": "ocr_result", "id": id, "error": format!("{error:#}")})
                            }
                        }
                    } else {
                        match qr::scan(&image, &cancelled) {
                            Ok(Some(url)) => json!({"type": "qr_result", "id": id, "url": url}),
                            Ok(None) => return Ok(()),
                            Err(error) => {
                                json!({"type": "qr_result", "id": id, "error": format!("{error:#}")})
                            }
                        }
                    };
                    if !cancelled.load(Ordering::Acquire) {
                        input.write(&response)?;
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
            "ocr_cancel" => {
                let id = event["id"]
                    .as_str()
                    .context("Missing OCR cancellation id")?;
                let response = match jobs.cancel(id) {
                    Ok(()) => json!({"type": "ocr_result", "id": id, "cancelled": true}),
                    Err(error) => {
                        json!({"type": "ocr_result", "id": id, "cancelled": false, "error": format!("{error:#}")})
                    }
                };
                handle.write(&response)?;
            }
            "capture_finished" => {
                jobs.cancel_all();
                let session = event["session"]
                    .as_str()
                    .context("Missing capture session")?;
                handle.write(&json!({"type": "capture_released", "session": session}))?;
                match event["status"].as_str() {
                    Some("saved") => on_event(RuntimeEvent::Saved {
                        action: event["action"]
                            .as_str()
                            .context("Missing capture action")?
                            .to_owned(),
                        path: event["path"].as_str().context("Missing saved path")?.into(),
                    }),
                    Some("exported") => on_event(RuntimeEvent::Exported {
                        path: event["path"]
                            .as_str()
                            .context("Missing exported path")?
                            .into(),
                    }),
                    Some("ask_google") => on_event(RuntimeEvent::Asked {
                        prompt: event["prompt"]
                            .as_str()
                            .context("Missing Ask Google prompt")?
                            .to_owned(),
                        path: event["path"]
                            .as_str()
                            .context("Missing Ask Google image path")?
                            .into(),
                    }),
                    Some("qr_open") => {
                        let url = qr::parse_url(event["url"].as_str().context("Missing QR URL")?)?;
                        if let Err(error) = opener::open_browser(url.as_str()) {
                            on_event(RuntimeEvent::Failed(format!(
                                "Could not open the QR URL: {error}"
                            )));
                        }
                    }
                    Some("ocr_open") => {
                        let url =
                            url::Url::parse(event["url"].as_str().context("Missing OCR URL")?)
                                .context("Invalid OCR action URL")?;
                        anyhow::ensure!(
                            url.scheme() == "https"
                                && matches!(
                                    url.host_str(),
                                    Some("www.google.com" | "translate.google.com")
                                ),
                            "Invalid OCR action URL"
                        );
                        if let Err(error) = opener::open_browser(url.as_str()) {
                            on_event(RuntimeEvent::Failed(format!(
                                "Could not open the OCR action: {error}"
                            )));
                        }
                    }
                    Some("failed") => on_event(RuntimeEvent::Failed(
                        event["error"]
                            .as_str()
                            .unwrap_or("Capture failed")
                            .to_owned(),
                    )),
                    Some("cancelled") => {}
                    _ => anyhow::bail!("Unknown capture status"),
                }
            }
            "error" => on_event(RuntimeEvent::Failed(
                event["message"]
                    .as_str()
                    .unwrap_or("Qt runtime error")
                    .to_owned(),
            )),
            kind => anyhow::bail!("Unknown Qt event: {kind}"),
        }
    }
    Ok(())
}
