// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use std::path::Path;
use xtask::{project_root, run_cmd_with_display};

use super::runner::print_group;

const CAPTURE_BIN_REL_PATH: &str = "sidecars/qt-capture/native/build/capture-bin";

pub fn run(list: bool, path: &[String]) -> Result<()> {
    if path.is_empty() {
        if list {
            print_group("sidecars", &["paddle-ocr (coming soon)", "qt-capture"]);
            return Ok(());
        }

        bail!("Missing sidecars suite. Run `cargo xtask test sidecars --list`.");
    }

    if list {
        match path[0].as_str() {
            "paddle-ocr" | "ocr" => {
                print_group("sidecars/paddle-ocr", &["coming soon"]);
                return Ok(());
            }
            "qt-capture" => {
                return list_qt_capture(&path[1..]);
            }
            other => {
                bail!(
                    "Unknown sidecars suite '{}'. Run `cargo xtask test sidecars --list`.",
                    other
                )
            }
        }
    }

    match path[0].as_str() {
        "paddle-ocr" | "ocr" => {
            bail!("`cargo xtask test sidecars paddle-ocr` is coming soon.")
        }
        "qt-capture" => run_qt_capture(&path[1..]),
        other => bail!(
            "Unknown sidecars suite '{}'. Run `cargo xtask test sidecars --list`.",
            other
        ),
    }
}

fn list_qt_capture(path: &[String]) -> Result<()> {
    if path.is_empty() {
        print_group("sidecars/qt-capture", &["capture"]);
        return Ok(());
    }

    match path[0].as_str() {
        "capture" => {
            if path.len() != 1 {
                bail!("Unexpected path for `cargo xtask test sidecars qt-capture capture --list`.");
            }

            print_group(
                "sidecars/qt-capture/capture",
                &["r (rectangle)", "f (freeshape)"],
            );
            Ok(())
        }
        other => bail!(
            "Unknown qt-capture action '{}'. Run `cargo xtask test sidecars qt-capture --list`.",
            other
        ),
    }
}

fn run_qt_capture(path: &[String]) -> Result<()> {
    if path.is_empty() {
        bail!("Missing qt-capture action. Run `cargo xtask test sidecars qt-capture --list`.");
    }

    match path[0].as_str() {
        "capture" => run_qt_capture_capture(&path[1..]),
        other => bail!(
            "Unknown qt-capture action '{}'. Run `cargo xtask test sidecars qt-capture --list`.",
            other
        ),
    }
}

fn run_qt_capture_capture(path: &[String]) -> Result<()> {
    let root = project_root();
    let capture_bin = root.join(CAPTURE_BIN_REL_PATH);
    ensure_capture_bin_exists(&capture_bin)?;

    let args = normalize_capture_args(path)?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let cmd = capture_bin.to_string_lossy().to_string();

    println!("\n[sidecars/qt-capture:capture]");
    run_cmd_with_display(cmd.as_str(), &arg_refs, &arg_refs, &root)
}

fn ensure_capture_bin_exists(path: &Path) -> Result<()> {
    if path.is_file() {
        return Ok(());
    }

    bail!(
        "qt-capture binary not found at '{}'. Build it first.",
        path.display()
    );
}

fn normalize_capture_args(path: &[String]) -> Result<Vec<String>> {
    let mut out = Vec::new();

    for token in path {
        let arg = token.trim();
        let normalized = match arg {
            "r" | "--r" => "--r",
            "f" | "--f" => "--f",
            other => {
                bail!(
                    "Unsupported capture argument '{}'. Use r/f or --r/--f.",
                    other
                )
            }
        };
        out.push(normalized.to_string());
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::normalize_capture_args;

    fn vec_path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn normalize_short_capture_flags() {
        let args = normalize_capture_args(&vec_path(&["r", "f"]))
            .expect("short flags should normalize");
        assert_eq!(args, vec!["--r".to_string(), "--f".to_string()]);
    }

    #[test]
    fn normalize_long_capture_flags() {
        let args = normalize_capture_args(&vec_path(&["--r", "--f"]))
            .expect("long flags should normalize");
        assert_eq!(args, vec!["--r".to_string(), "--f".to_string()]);
    }

    #[test]
    fn reject_unknown_capture_flag() {
        let err = normalize_capture_args(&vec_path(&["x"]))
            .expect_err("unknown flag should error");
        assert!(err
            .to_string()
            .contains("Unsupported capture argument 'x'"));
    }
}
