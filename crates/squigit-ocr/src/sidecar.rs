use crate::TextBox;
use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn recognize(image: &Path) -> Result<Vec<TextBox>> {
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
    let output = command
        .output()
        .with_context(|| format!("Could not start {}", binary.display()))?;
    if !output.status.success() {
        let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_default();
        anyhow::bail!(
            "{}",
            payload["error"]
                .as_str()
                .unwrap_or("OCR process failed; see stderr")
        );
    }
    let boxes: Vec<TextBox> =
        serde_json::from_slice(&output.stdout).context("OCR returned invalid JSON")?;
    for item in &boxes {
        anyhow::ensure!(
            item.confidence.is_finite() && item.r#box.iter().flatten().all(|v| v.is_finite()),
            "OCR returned invalid coordinates"
        );
    }
    Ok(boxes)
}
