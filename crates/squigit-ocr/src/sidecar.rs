use crate::TextBox;
use anyhow::{Context, Result};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub fn recognize(image: &Path, cancelled: &AtomicBool) -> Result<Vec<TextBox>> {
    let root = std::env::current_exe()?
        .parent()
        .context("Missing application directory")?
        .to_path_buf();
    let binary = root.join("_internal/ocr").join(if cfg!(windows) {
        "squigit-ocr.exe"
    } else {
        "squigit-ocr"
    });
    let mut command = Command::new(&binary);
    command
        .arg(image)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("Could not start {}", binary.display()))?;
    let mut stdout = child.stdout.take().context("Missing OCR stdout")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if cancelled.load(Ordering::Acquire) || started.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            anyhow::bail!("OCR cancelled or exceeded two minutes");
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(error.into());
            }
        }
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
    Ok(boxes)
}
