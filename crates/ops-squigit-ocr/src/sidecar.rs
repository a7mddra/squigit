// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use semver::{Version, VersionReq};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_OCR_VERSION_REQUIREMENT: &str = ">=1.2.0";

#[derive(Debug, Error)]
pub enum SidecarError {
    #[error("ERR_MISSING_OCR_PACKAGE")]
    MissingPackage,
    #[error("ERR_OUTDATED_OCR_PACKAGE")]
    OutdatedPackage,
    #[error("Invalid OCR version requirement: {0}")]
    InvalidRequirement(String),
    #[error("Failed to parse OCR sidecar version")]
    VersionParseFailed,
}

pub fn resolve_sidecar_path(resource_dir: &Path) -> (PathBuf, Option<PathBuf>) {
    let name = if cfg!(windows) {
        "squigit-ocr.exe"
    } else {
        "squigit-ocr"
    };

    // 1. PATH (installed via winget/brew/apt/dnf)
    if let Ok(path) = which::which(name) {
        return (path, None);
    }

    // 2. macOS GUI Fallback
    #[cfg(target_os = "macos")]
    {
        let brew_arm = PathBuf::from("/opt/homebrew/bin/squigit-ocr");
        let brew_intel = PathBuf::from("/usr/local/bin/squigit-ocr");
        if brew_arm.exists() {
            return (brew_arm, None);
        }
        if brew_intel.exists() {
            return (brew_intel, None);
        }
    }

    // 3. Windows GUI (Winget) Fallback
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            let winget_path = PathBuf::from(local_app_data)
                .join("Microsoft")
                .join("WindowsApps")
                .join("squigit-ocr.exe");
            if winget_path.exists() {
                return (winget_path, None);
            }
        }
    }

    // 4. Packaged runtime dir (legacy / transition case)
    let host_triple = get_ocr_target_triple();
    let runtime = resource_dir
        .join("binaries")
        .join(format!("paddle-ocr-{}", host_triple));
    let candidate = runtime.join(name);
    if candidate.exists() {
        return (candidate, Some(runtime.clone()));
    }

    // 5. Dev mode fallback
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(target_dir) = current_exe.parent().and_then(|p| p.parent()) {
            let debug_runtime = target_dir
                .join("debug")
                .join("binaries")
                .join(format!("paddle-ocr-{}", host_triple));
            let debug_candidate = debug_runtime.join(name);
            if debug_candidate.exists() {
                return (debug_candidate, Some(debug_runtime));
            }
        }
    }

    (PathBuf::from(name), None)
}

pub fn read_sidecar_version(sidecar_path: &Path) -> Result<String, SidecarError> {
    let output = std::process::Command::new(sidecar_path)
        .arg("--version")
        .output()
        .map_err(|_| SidecarError::MissingPackage)?;

    if !output.status.success() {
        return Err(SidecarError::MissingPackage);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = extract_semver_from_text(&stdout).ok_or(SidecarError::VersionParseFailed)?;
    Ok(version.to_string())
}

pub fn check_ocr_version_requirement(
    sidecar_path: &Path,
    requirement: &str,
) -> Result<String, SidecarError> {
    let req = VersionReq::parse(requirement)
        .map_err(|e| SidecarError::InvalidRequirement(e.to_string()))?;

    let version = read_sidecar_version(sidecar_path)?;
    let parsed = Version::parse(&version).map_err(|_| SidecarError::VersionParseFailed)?;

    if req.matches(&parsed) {
        return Ok(version);
    }

    Err(SidecarError::OutdatedPackage)
}

fn extract_semver_from_text(raw: &str) -> Option<Version> {
    let mut candidates = raw
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '+'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    candidates.reverse();

    for token in candidates {
        if let Ok(version) = Version::parse(token) {
            return Some(version);
        }
    }

    None
}

fn get_ocr_target_triple() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(target_os = "macos")]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(target_os = "linux")]
    {
        "x86_64-unknown-linux-gnu"
    }
}

#[cfg(test)]
mod tests {
    use super::extract_semver_from_text;
    use semver::VersionReq;

    #[test]
    fn extracts_semver_from_noisy_output() {
        let output = "Connectivity check skipped\n0.1.2\n";
        let version = extract_semver_from_text(output).expect("version");
        assert_eq!(version.to_string(), "0.1.2");
    }

    #[test]
    fn requirement_eval_supports_greater_or_equal() {
        let req = VersionReq::parse(">=1.2.0").unwrap();
        let good = extract_semver_from_text("1.2.0").unwrap();
        let bad = extract_semver_from_text("1.1.9").unwrap();

        assert!(req.matches(&good));
        assert!(!req.matches(&bad));
    }
}
