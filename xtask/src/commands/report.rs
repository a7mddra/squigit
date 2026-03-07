// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::console::Ansi;
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::DocumentMut;
use walkdir::WalkDir;
use xtask::{get_host_target_triple, project_root, tauri_dir, ui_dir};

#[derive(Debug, Clone, Copy, Default)]
pub struct ReportOptions {
    pub strict: bool,
}

#[derive(Debug, Clone)]
struct CheckResult {
    name: String,
    passed: bool,
    details: String,
}

impl CheckResult {
    fn pass(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            details: details.into(),
        }
    }

    fn fail(name: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            details: details.into(),
        }
    }
}

pub fn run(options: ReportOptions) -> Result<()> {
    let ansi = Ansi::detect();

    println!("\n{}", ansi.bold("Squigit xtask report"));
    println!(
        "  strict mode: {}",
        if options.strict { "on" } else { "off" }
    );

    let mut checks = Vec::new();
    checks.extend(tool_checks());
    checks.push(command_check(
        "cargo check",
        "cargo",
        &["check"],
        &project_root(),
    ));
    checks.push(command_check(
        "renderer typecheck",
        "npx",
        &["tsc", "--noEmit"],
        &ui_dir(),
    ));

    checks.extend(sidecar_checks()?);
    checks.push(version_consistency_check()?);

    println!("\n{}", ansi.bold("Results"));

    let mut failed = 0usize;
    let mut passed = 0usize;

    for check in &checks {
        let status = if check.passed {
            passed += 1;
            ansi.green("PASS")
        } else {
            failed += 1;
            ansi.red("FAIL")
        };

        println!("  [{}] {}", status, check.name);
        if !check.details.trim().is_empty() {
            for line in check.details.lines() {
                println!("       {}", line);
            }
        }
    }

    println!("\n{}", ansi.bold("Summary"));
    println!("  passed: {passed}");
    println!("  failed: {failed}");

    if options.strict && failed > 0 {
        anyhow::bail!("Report failed in strict mode.");
    }

    if failed > 0 {
        println!(
            "{}",
            ansi.yellow("Report completed with warnings (non-strict mode keeps exit code 0).")
        );
    } else {
        println!("{}", ansi.green("Report checks passed."));
    }

    Ok(())
}

fn tool_checks() -> Vec<CheckResult> {
    let mut checks = Vec::new();

    for tool in ["cargo", "rustc", "python3", "npm", "npx", "cmake"] {
        checks.push(tool_check(tool, tool));
    }

    let ui = ui_dir();
    let local_tauri = ui.join("node_modules").join(".bin").join(if cfg!(windows) {
        "tauri.cmd"
    } else {
        "tauri"
    });

    let tauri_from_path = which::which("tauri").is_ok() || which::which("tauri.cmd").is_ok();
    if tauri_from_path || local_tauri.exists() {
        let details = if tauri_from_path {
            "tauri found on PATH".to_string()
        } else {
            format!("tauri found at {}", local_tauri.display())
        };
        checks.push(CheckResult::pass("tool: tauri", details));
    } else {
        checks.push(CheckResult::fail(
            "tool: tauri",
            format!(
                "tauri not found on PATH or local node_modules. Expected local path: {}",
                local_tauri.display()
            ),
        ));
    }

    checks
}

fn tool_check(name: &str, command: &str) -> CheckResult {
    match which::which(command) {
        Ok(path) => CheckResult::pass(format!("tool: {name}"), format!("{}", path.display())),
        Err(_) => CheckResult::fail(format!("tool: {name}"), "not found on PATH"),
    }
}

fn command_check(name: &str, cmd: &str, args: &[&str], cwd: &Path) -> CheckResult {
    let output = Command::new(cmd).args(args).current_dir(cwd).output();

    match output {
        Ok(out) => {
            if out.status.success() {
                CheckResult::pass(name, format!("command succeeded in {}", cwd.display()))
            } else {
                CheckResult::fail(
                    name,
                    format!(
                        "exit code {:?}\n{}",
                        out.status.code(),
                        compact_output(&out.stdout, &out.stderr)
                    ),
                )
            }
        }
        Err(err) => CheckResult::fail(name, format!("failed to execute: {err}")),
    }
}

fn compact_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut merged = String::new();

    let stdout_text = String::from_utf8_lossy(stdout).trim().to_string();
    if !stdout_text.is_empty() {
        merged.push_str("stdout:\n");
        merged.push_str(&truncate_text(&stdout_text, 12, 1200));
        merged.push('\n');
    }

    let stderr_text = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr_text.is_empty() {
        merged.push_str("stderr:\n");
        merged.push_str(&truncate_text(&stderr_text, 12, 1200));
    }

    if merged.trim().is_empty() {
        "(no output)".to_string()
    } else {
        merged.trim_end().to_string()
    }
}

fn truncate_text(text: &str, max_lines: usize, max_chars: usize) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    if lines.len() > max_lines {
        lines.truncate(max_lines);
    }
    let mut out = lines.join("\n");
    if out.len() > max_chars {
        out.truncate(max_chars);
        out.push_str("\n... (truncated)");
    }
    out
}

fn sidecar_checks() -> Result<Vec<CheckResult>> {
    let mut checks = Vec::new();

    let host = get_host_target_triple().context("Failed to resolve host target triple")?;
    let app_binaries = tauri_dir().join("binaries");
    let debug_binaries = project_root().join("target").join("debug").join("binaries");

    let expected = expected_sidecar_artifacts(&host);

    for base in [&app_binaries, &debug_binaries] {
        for (label, rel_path) in &expected {
            let path = base.join(rel_path);
            let name = format!("artifact: {} ({})", label, base.display());
            if path.exists() {
                let size = path_size_bytes(&path)?;
                checks.push(CheckResult::pass(
                    name,
                    format!("found at {} ({})", path.display(), format_bytes(size)),
                ));
            } else {
                checks.push(CheckResult::fail(
                    name,
                    format!("missing at {}", path.display()),
                ));
            }
        }
    }

    let total_app_size = if app_binaries.exists() {
        let mut total = 0u64;
        for entry in fs::read_dir(&app_binaries)? {
            let entry = entry?;
            total = total.saturating_add(path_size_bytes(&entry.path())?);
        }
        total
    } else {
        0
    };

    checks.push(CheckResult::pass(
        "binaries total size (apps/desktop/binaries)",
        format!("{}", format_bytes(total_app_size)),
    ));

    Ok(checks)
}

fn expected_sidecar_artifacts(host: &str) -> Vec<(String, PathBuf)> {
    let whisper_name = if cfg!(windows) {
        format!("whisper-stt-{host}.exe")
    } else {
        format!("whisper-stt-{host}")
    };

    vec![
        (
            "OCR runtime".to_string(),
            PathBuf::from(format!("paddle-ocr-{host}")),
        ),
        (
            "Capture runtime".to_string(),
            PathBuf::from(format!("qt-capture-{host}")),
        ),
        ("Whisper binary".to_string(), PathBuf::from(whisper_name)),
    ]
}

fn version_consistency_check() -> Result<CheckResult> {
    let root = project_root();
    let version_file = root.join("VERSION");

    let canonical = match fs::read_to_string(&version_file) {
        Ok(content) => content.trim().to_string(),
        Err(_) => {
            return Ok(CheckResult::fail(
                "version consistency",
                format!(
                    "VERSION file missing at {}. Use `cargo xtask version <x.y.z>` to create/sync it.",
                    version_file.display()
                ),
            ));
        }
    };

    if canonical.is_empty() {
        return Ok(CheckResult::fail(
            "version consistency",
            "VERSION file is empty",
        ));
    }

    let mut mismatches = Vec::new();

    let workspace_version = read_workspace_version(&root.join("Cargo.toml"))?;
    if workspace_version.as_deref() != Some(canonical.as_str()) {
        mismatches.push(format!(
            "Cargo workspace version mismatch: expected {canonical}, got {}",
            workspace_version.unwrap_or_else(|| "<missing>".to_string())
        ));
    }

    for (path, value) in collect_package_versions(&root)? {
        if value != canonical {
            mismatches.push(format!(
                "{} -> {}",
                path.strip_prefix(&root).unwrap_or(path.as_path()).display(),
                value
            ));
        }
    }

    for (path, value) in [
        (
            root.join("apps").join("desktop").join("tauri.conf.json"),
            read_json_version(&root.join("apps").join("desktop").join("tauri.conf.json"))?,
        ),
        (
            root.join("apps")
                .join("desktop")
                .join("renderer")
                .join("package.json"),
            read_json_version(
                &root
                    .join("apps")
                    .join("desktop")
                    .join("renderer")
                    .join("package.json"),
            )?,
        ),
    ] {
        if value.as_deref() != Some(canonical.as_str()) {
            mismatches.push(format!(
                "{} -> {}",
                path.strip_prefix(&root).unwrap_or(path.as_path()).display(),
                value.unwrap_or_else(|| "<missing>".to_string())
            ));
        }
    }

    for cmake_path in [
        root.join("sidecars")
            .join("qt-capture")
            .join("native")
            .join("CMakeLists.txt"),
        root.join("sidecars")
            .join("whisper-stt")
            .join("CMakeLists.txt"),
    ] {
        let versions = read_cmake_project_versions(&cmake_path)?;
        if versions.is_empty() {
            mismatches.push(format!(
                "{} -> missing project VERSION",
                cmake_path
                    .strip_prefix(&root)
                    .unwrap_or(cmake_path.as_path())
                    .display()
            ));
        } else if versions.iter().any(|value| value != &canonical) {
            mismatches.push(format!(
                "{} -> {}",
                cmake_path
                    .strip_prefix(&root)
                    .unwrap_or(cmake_path.as_path())
                    .display(),
                versions.join(", ")
            ));
        }
    }

    if mismatches.is_empty() {
        Ok(CheckResult::pass(
            "version consistency",
            format!("all tracked versions match canonical {canonical}"),
        ))
    } else {
        Ok(CheckResult::fail(
            "version consistency",
            format!("canonical VERSION={canonical}\n{}", mismatches.join("\n")),
        ))
    }
}

fn read_workspace_version(path: &Path) -> Result<Option<String>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read workspace Cargo file: {}", path.display()))?;
    let doc = text
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse TOML at {}", path.display()))?;

    let version = doc
        .get("workspace")
        .and_then(|item| item.as_table_like())
        .and_then(|workspace| workspace.get("package"))
        .and_then(|item| item.as_table_like())
        .and_then(|pkg| pkg.get("version"))
        .and_then(|item| item.as_str())
        .map(ToString::to_string);

    Ok(version)
}

fn collect_package_versions(root: &Path) -> Result<Vec<(PathBuf, String)>> {
    let mut out = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() || entry.file_name() != "Cargo.toml" {
            continue;
        }

        let path = entry.path().to_path_buf();
        let text = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read Cargo file: {}", path.display()))?;
        let doc = match text.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(_) => continue,
        };

        if let Some(version) = doc
            .get("package")
            .and_then(|item| item.as_table_like())
            .and_then(|pkg| pkg.get("version"))
            .and_then(|item| item.as_str())
        {
            out.push((path, version.to_string()));
        }
    }

    Ok(out)
}

fn read_json_version(path: &Path) -> Result<Option<String>> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read JSON version file: {}", path.display()))?;
    let json: Value = serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse JSON file: {}", path.display()))?;

    Ok(json
        .get("version")
        .and_then(|item| item.as_str())
        .map(ToString::to_string))
}

fn read_cmake_project_versions(path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read CMake file: {}", path.display()))?;

    let re = Regex::new(r"(?i)project\([^\)]*\bVERSION\s+([0-9]+\.[0-9]+\.[0-9]+)")
        .expect("valid regex");

    Ok(re
        .captures_iter(&content)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect())
}

fn path_size_bytes(path: &Path) -> Result<u64> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("Failed to inspect path: {}", path.display()))?;

    if metadata.is_file() {
        return Ok(metadata.len());
    }

    if metadata.is_dir() {
        let mut total = 0u64;
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                total = total.saturating_add(entry.metadata()?.len());
            }
        }
        return Ok(total);
    }

    Ok(0)
}

fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.2} KB", b / KB)
    } else {
        format!("{} B", bytes)
    }
}

fn is_ignored_path(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            ".git" | "target" | "node_modules" | "venv" | "build" | "dist"
        )
    })
}
