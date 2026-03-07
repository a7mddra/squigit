// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use chrono::Local;
use clap::ValueEnum;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut};
use walkdir::WalkDir;
use xtask::project_root;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BumpPart {
    Patch,
    Minor,
    Major,
}

#[derive(Debug, Clone, Default)]
pub struct VersionOptions {
    pub explicit: Option<String>,
    pub bump: Option<BumpPart>,
}

pub fn run(options: VersionOptions) -> Result<()> {
    if options.explicit.is_some() == options.bump.is_some() {
        anyhow::bail!(
            "Provide exactly one of: explicit version argument OR --bump patch|minor|major"
        );
    }

    let root = project_root();
    let version_path = root.join("VERSION");

    let target_version = if let Some(explicit) = options.explicit {
        parse_semver(&explicit)?;
        explicit
    } else {
        let current = read_canonical_version(&root, &version_path)?;
        bump_semver(&current, options.bump.expect("validated above"))?
    };

    parse_semver(&target_version)?;

    fs::write(&version_path, format!("{}\n", target_version))?;

    let mut changed_files = vec![version_path.clone()];

    let cargo_files = find_cargo_manifests(&root)?;
    for cargo_path in cargo_files {
        if update_cargo_manifest(&cargo_path, &target_version)? {
            changed_files.push(cargo_path);
        }
    }

    let tauri_json = root.join("apps").join("desktop").join("tauri.conf.json");
    if update_json_version_line(&tauri_json, &target_version)? {
        changed_files.push(tauri_json);
    }

    let renderer_json = root
        .join("apps")
        .join("desktop")
        .join("renderer")
        .join("package.json");
    if update_json_version_line(&renderer_json, &target_version)? {
        changed_files.push(renderer_json);
    }

    let qt_cmake = root
        .join("sidecars")
        .join("qt-capture")
        .join("native")
        .join("CMakeLists.txt");
    if update_cmake_project_versions_file(&qt_cmake, &target_version)? {
        changed_files.push(qt_cmake);
    }

    let whisper_cmake = root
        .join("sidecars")
        .join("whisper-stt")
        .join("CMakeLists.txt");
    if update_cmake_project_versions_file(&whisper_cmake, &target_version)? {
        changed_files.push(whisper_cmake);
    }

    let changelog = root.join("CHANGELOG.md");
    if ensure_changelog_version_section(&changelog, &target_version)? {
        changed_files.push(changelog);
    }

    println!("\nUpdated version -> {}", target_version);
    println!("Changed files:");
    for path in changed_files {
        println!(
            "  - {}",
            path.strip_prefix(&root).unwrap_or(path.as_path()).display()
        );
    }

    Ok(())
}

fn read_canonical_version(root: &Path, version_file: &Path) -> Result<String> {
    if version_file.exists() {
        let value = fs::read_to_string(version_file)?;
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            parse_semver(trimmed)?;
            return Ok(trimmed.to_string());
        }
    }

    let workspace_cargo = root.join("Cargo.toml");
    let content = fs::read_to_string(&workspace_cargo).with_context(|| {
        format!(
            "Failed to read workspace cargo file: {}",
            workspace_cargo.display()
        )
    })?;
    let doc = content.parse::<DocumentMut>().with_context(|| {
        format!(
            "Failed to parse workspace Cargo.toml: {}",
            workspace_cargo.display()
        )
    })?;

    let fallback = doc
        .get("workspace")
        .and_then(|item| item.get("package"))
        .and_then(|item| item.get("version"))
        .and_then(|item| item.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not derive canonical version from VERSION or workspace.package.version"
            )
        })?
        .to_string();

    parse_semver(&fallback)?;
    Ok(fallback)
}

fn parse_semver(version: &str) -> Result<(u64, u64, u64)> {
    let re = Regex::new(r"^([0-9]+)\.([0-9]+)\.([0-9]+)$").expect("valid semver regex");
    let caps = re
        .captures(version.trim())
        .ok_or_else(|| anyhow::anyhow!("Invalid version '{}'. Expected semver x.y.z", version))?;

    let major = caps[1].parse::<u64>()?;
    let minor = caps[2].parse::<u64>()?;
    let patch = caps[3].parse::<u64>()?;

    Ok((major, minor, patch))
}

fn bump_semver(current: &str, bump: BumpPart) -> Result<String> {
    let (mut major, mut minor, mut patch) = parse_semver(current)?;
    match bump {
        BumpPart::Patch => patch += 1,
        BumpPart::Minor => {
            minor += 1;
            patch = 0;
        }
        BumpPart::Major => {
            major += 1;
            minor = 0;
            patch = 0;
        }
    }

    Ok(format!("{major}.{minor}.{patch}"))
}

fn find_cargo_manifests(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_ignored_path(entry.path()))
    {
        let entry = entry?;
        if entry.file_type().is_file() && entry.file_name() == "Cargo.toml" {
            paths.push(entry.path().to_path_buf());
        }
    }

    Ok(paths)
}

fn update_cargo_manifest(path: &Path, version: &str) -> Result<bool> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read Cargo manifest: {}", path.display()))?;
    let mut doc = content
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse Cargo manifest: {}", path.display()))?;

    let mut changed = false;

    let has_workspace_version = doc
        .get("workspace")
        .and_then(|item| item.get("package"))
        .and_then(|item| item.get("version"))
        .and_then(|item| item.as_str())
        .is_some();

    if has_workspace_version {
        doc["workspace"]["package"]["version"] = value(version);
        changed = true;
    }

    let has_package_version = doc
        .get("package")
        .and_then(|item| item.get("version"))
        .and_then(|item| item.as_str())
        .is_some();

    if has_package_version {
        doc["package"]["version"] = value(version);
        changed = true;
    }

    if changed {
        fs::write(path, doc.to_string())?;
    }

    Ok(changed)
}

fn update_json_version_line(path: &Path, version: &str) -> Result<bool> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read JSON file: {}", path.display()))?;

    let re = Regex::new(r#"(?m)^(\s*"version"\s*:\s*")([^"]+)("\s*,?\s*)$"#)
        .expect("valid version line regex");

    let updated = re.replacen(&content, 1, format!("${{1}}{}${{3}}", version));
    if updated == content {
        anyhow::bail!(
            "Could not locate top-level version line in {}",
            path.display()
        );
    }

    fs::write(path, updated.as_ref())?;
    Ok(true)
}

fn update_cmake_project_versions_file(path: &Path, version: &str) -> Result<bool> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read CMake file: {}", path.display()))?;

    let (updated, changed) = update_cmake_project_versions_text(&content, version);
    if changed {
        fs::write(path, updated)?;
    }

    Ok(changed)
}

fn update_cmake_project_versions_text(content: &str, version: &str) -> (String, bool) {
    let project_re =
        Regex::new(r"(?i)^(\s*project\(\s*[^\s\)]+)([^\)]*)\)\s*$").expect("valid project regex");
    let version_re = Regex::new(r"(?i)\s+VERSION\s+[^\s\)]+").expect("valid cmake version regex");

    let mut changed = false;
    let mut out = String::new();

    for line in content.lines() {
        if let Some(caps) = project_re.captures(line) {
            let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let rest = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let rest_no_version = version_re.replace_all(rest, "");
            let new_line = format!("{} VERSION {}{})", prefix, version, rest_no_version);
            if new_line != line {
                changed = true;
            }
            out.push_str(&new_line);
            out.push('\n');
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    if !content.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }

    (out, changed)
}

fn ensure_changelog_version_section(path: &Path, version: &str) -> Result<bool> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read changelog: {}", path.display()))?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let (updated, changed) = insert_changelog_section(&content, version, &today);
    if changed {
        fs::write(path, updated)?;
    }

    Ok(changed)
}

fn insert_changelog_section(content: &str, version: &str, date: &str) -> (String, bool) {
    let version_heading_re = Regex::new(&format!(r"(?m)^## \[{}\]", regex::escape(version)))
        .expect("valid heading regex");

    if version_heading_re.is_match(content) {
        return (content.to_string(), false);
    }

    let first_section_re = Regex::new(r"(?m)^## \[").expect("valid first section regex");
    let insert_at = first_section_re
        .find(content)
        .map(|m| m.start())
        .unwrap_or(content.len());

    let section = format!(
        "## [{version}] - {date}\n\n### Added\n\n- TBD\n\n### Changed\n\n- TBD\n\n### Fixed\n\n- TBD\n\n"
    );

    let (head, tail) = content.split_at(insert_at);

    let mut out = String::new();
    out.push_str(head.trim_end());
    out.push_str("\n\n");
    out.push_str(&section);
    out.push_str(tail.trim_start_matches('\n'));

    (out, true)
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

#[cfg(test)]
mod tests {
    use super::{
        bump_semver, insert_changelog_section, parse_semver, update_cmake_project_versions_text,
        BumpPart,
    };

    #[test]
    fn parse_semver_and_bump() {
        assert_eq!(parse_semver("1.2.3").expect("valid semver"), (1, 2, 3));
        assert_eq!(
            bump_semver("1.2.3", BumpPart::Patch).expect("patch"),
            "1.2.4"
        );
        assert_eq!(
            bump_semver("1.2.3", BumpPart::Minor).expect("minor"),
            "1.3.0"
        );
        assert_eq!(
            bump_semver("1.2.3", BumpPart::Major).expect("major"),
            "2.0.0"
        );
        assert!(parse_semver("1.2").is_err());
    }

    #[test]
    fn changelog_section_is_inserted_once() {
        let base =
            "# Changelog\n\nAll notable changes...\n\n## [0.1.0] - 2025-10-02\n\n### Added\n";
        let (updated, changed) = insert_changelog_section(base, "0.2.0", "2026-03-07");
        assert!(changed);
        assert!(updated.contains("## [0.2.0] - 2026-03-07"));

        let (updated_again, changed_again) =
            insert_changelog_section(&updated, "0.2.0", "2026-03-07");
        assert!(!changed_again);
        assert_eq!(updated_again, updated);
    }

    #[test]
    fn cmake_project_versions_are_enforced() {
        let input = r#"
project(squigit-stt)
project(CAPTURE LANGUAGES CXX)
project(CAPTURE VERSION 1.0.0 LANGUAGES CXX OBJCXX)
"#;

        let (updated, changed) = update_cmake_project_versions_text(input, "2.3.4");
        assert!(changed);
        assert!(updated.contains("project(squigit-stt VERSION 2.3.4)"));
        assert!(updated.contains("project(CAPTURE VERSION 2.3.4 LANGUAGES CXX)"));
        assert!(updated.contains("project(CAPTURE VERSION 2.3.4 LANGUAGES CXX OBJCXX)"));
    }
}
