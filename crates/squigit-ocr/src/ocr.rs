use crate::{Recognition, TextBox};
use anyhow::{Context, Result};
use process_wrap::std::{ChildWrapper, CommandWrap};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub fn recognize(
    image: &Path,
    cancelled: &AtomicBool,
    models: &crate::models::ModelManager,
) -> Result<Recognition> {
    if cancelled.load(Ordering::Acquire) {
        return Ok(Recognition::Cancelled);
    }
    let binary = crate::sidecar::resolve_sidecar_path()?;
    let selected = models.selected_model()?;
    let mut command = Command::new(&binary);
    if selected != crate::models::DEFAULT_OCR_MODEL_ID {
        command
            .arg("--rec-model-dir")
            .arg(models.get_model_dir(&selected));
    }
    if cancelled.load(Ordering::Acquire) {
        return Ok(Recognition::Cancelled);
    }
    command
        .arg(image)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    command
        .env("OMP_NUM_THREADS", "1")
        .env("OPENBLAS_NUM_THREADS", "1")
        .env("MKL_NUM_THREADS", "1")
        .env("NUMEXPR_NUM_THREADS", "1")
        .env("OMP_WAIT_POLICY", "PASSIVE");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            command.pre_exec(|| {
                libc::nice(10);
                #[cfg(target_os = "linux")]
                libc::syscall(libc::SYS_ioprio_set, 1, 0, (2 << 13) | 7);
                Ok(())
            });
        }
    }
    let mut command = CommandWrap::from(command);
    #[cfg(unix)]
    command.wrap(process_wrap::std::ProcessGroup::leader());
    #[cfg(windows)]
    command
        .wrap(process_wrap::std::CreationFlags(
            windows::Win32::System::Threading::CREATE_NO_WINDOW
                | windows::Win32::System::Threading::BELOW_NORMAL_PRIORITY_CLASS,
        ))
        .wrap(process_wrap::std::JobObject);
    let mut child = command
        .spawn()
        .with_context(|| format!("Could not start the OCR runtime at {}", binary.display()))?;
    let Some(mut stdout) = child.stdout().take() else {
        stop(child.as_mut())?;
        anyhow::bail!("Missing OCR stdout");
    };
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let mut exited = None;
    let status = loop {
        let cancel_requested = cancelled.load(Ordering::Acquire);
        if cancel_requested || started.elapsed() > Duration::from_secs(120) {
            stop(child.as_mut())?;
            let _ = reader.join();
            if cancel_requested {
                return Ok(Recognition::Cancelled);
            }
            anyhow::bail!("OCR exceeded two minutes");
        }
        if exited.is_none() {
            match child.try_wait() {
                Ok(status) => exited = status,
                Err(error) => {
                    stop(child.as_mut())?;
                    let _ = reader.join();
                    return Err(error.into());
                }
            }
        }
        if let Some(status) = exited {
            if reader.is_finished() {
                break status;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let output = reader
        .join()
        .map_err(|_| anyhow::anyhow!("OCR reader stopped"))??;
    if !status.success() {
        let payload: serde_json::Value = serde_json::from_slice(&output).unwrap_or_default();
        anyhow::bail!(
            "{}",
            payload["error"]
                .as_str()
                .unwrap_or("OCR process failed; see stderr")
        );
    }
    let boxes: Vec<TextBox> =
        serde_json::from_slice(&output).context("OCR returned invalid JSON")?;
    for item in &boxes {
        anyhow::ensure!(
            item.confidence.is_finite() && item.r#box.iter().flatten().all(|v| v.is_finite()),
            "OCR returned invalid coordinates"
        );
    }
    Ok(Recognition::Text(boxes))
}

fn stop(child: &mut dyn ChildWrapper) -> Result<()> {
    if let Err(error) = child.kill() {
        if child.try_wait()?.is_none() {
            return Err(error).context("Could not stop the OCR process tree");
        }
    }
    child
        .wait()
        .context("Could not reap the OCR process tree")?;
    Ok(())
}
