// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use xtask::{run_cmd_with_node_bin, run_cmd_with_node_bin_and_env, tauri_dir, ui_dir};
use xtask::run_cmd;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildTarget {
    Cli,
    Ocr,
    Stt,
    Capture,
    CaptureQt,
    Desktop,
}

impl BuildTarget {
    fn key(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Ocr => "ocr",
            Self::Stt => "stt",
            Self::Capture => "capture",
            Self::CaptureQt => "capture-qt",
            Self::Desktop => "desktop",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cli => "CLI",
            Self::Ocr => "PaddleOCR",
            Self::Stt => "Whisper STT",
            Self::Capture => "Capture Engine",
            Self::CaptureQt => "Capture Qt",
            Self::Desktop => "Desktop (Tauri)",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuildCommandOptions {
    pub selectors: Vec<String>,
    pub include_all: bool,
    pub measure_ocr_size: bool,
}

#[derive(Debug)]
struct BuildRunResult {
    target: BuildTarget,
    elapsed: Duration,
    ok: bool,
    error: Option<String>,
}

pub fn run(options: BuildCommandOptions) -> Result<()> {
    let mut selectors = options.selectors.clone();
    let inline_flags = extract_inline_flags(&mut selectors);

    let targets = resolve_targets(&selectors, options.include_all)?;
    if targets.is_empty() {
        anyhow::bail!("No build targets selected.");
    }

    let measure_ocr_size = options.measure_ocr_size || inline_flags.measure_ocr_size;

    println!("\nBuild plan:");
    for target in &targets {
        println!("  - {} ({})", target.label(), target.key());
    }
    if measure_ocr_size {
        println!("  - OCR size measurement: enabled");
    } else {
        println!("  - OCR size measurement: disabled (use --measure-ocr-size)");
    }

    let mut results = Vec::with_capacity(targets.len());
    for target in targets {
        let started = Instant::now();
        let result = match target {
            BuildTarget::Cli => cli_placeholder(),
            BuildTarget::Ocr => crate::compile::paddle_ocr::build(crate::compile::paddle_ocr::OcrBuildOptions {
                measure_payload_size: measure_ocr_size,
            }),
            BuildTarget::Stt => crate::compile::whisper_stt::build(),
            BuildTarget::Capture => crate::compile::qt_capture::build_all(),
            BuildTarget::CaptureQt => crate::compile::qt_capture::qt_only(),
            BuildTarget::Desktop => desktop(),
        };

        let elapsed = started.elapsed();
        let run = match result {
            Ok(()) => BuildRunResult {
                target,
                elapsed,
                ok: true,
                error: None,
            },
            Err(err) => BuildRunResult {
                target,
                elapsed,
                ok: false,
                error: Some(format!("{err:#}")),
            },
        };
        results.push(run);
    }

    print_build_summary(&results);

    if results.iter().any(|r| !r.ok) {
        anyhow::bail!("One or more build targets failed.");
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, Default)]
struct InlineBuildFlags {
    measure_ocr_size: bool,
}

fn extract_inline_flags(selectors: &mut Vec<String>) -> InlineBuildFlags {
    let mut flags = InlineBuildFlags::default();

    selectors.retain(|token| {
        let lowered = token.trim().to_ascii_lowercase();
        if lowered == "--measure-ocr-size" || lowered == "-measure-ocr-size" {
            flags.measure_ocr_size = true;
            return false;
        }
        true
    });

    flags
}

pub fn resolve_targets(selectors: &[String], include_all_flag: bool) -> Result<Vec<BuildTarget>> {
    let mut include_all = include_all_flag;
    let mut includes = Vec::new();
    let mut excludes = Vec::new();

    for raw in selectors {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }

        let lowered = token.to_ascii_lowercase();
        if lowered == "all" || lowered == "--all" {
            include_all = true;
            continue;
        }

        if let Some(excluded) = lowered.strip_prefix('-') {
            if excluded.is_empty() {
                anyhow::bail!("Invalid exclusion token '{token}'. Expected format like '-ocr'.");
            }
            excludes.push(parse_target(excluded)?);
            continue;
        }

        includes.push(parse_target(&lowered)?);
    }

    let mut selected = Vec::new();
    if include_all || (!excludes.is_empty() && includes.is_empty()) || selectors.is_empty() {
        selected.extend(default_targets());
    }

    if !includes.is_empty() {
        selected.extend(includes);
    }

    dedupe_targets(&mut selected);

    if !excludes.is_empty() {
        let excluded_set: HashSet<BuildTarget> = excludes.into_iter().collect();
        selected.retain(|target| !excluded_set.contains(target));
    }

    if selected.is_empty() {
        anyhow::bail!("Build target selection resolved to empty set.");
    }

    Ok(selected)
}

fn print_build_summary(results: &[BuildRunResult]) {
    println!("\nBuild Summary:");

    let mut passed = 0usize;
    let mut failed = 0usize;

    for result in results {
        let status = if result.ok {
            passed += 1;
            "PASS"
        } else {
            failed += 1;
            "FAIL"
        };

        println!(
            "  [{}] {:<16} {:>6.2}s",
            status,
            result.target.key(),
            result.elapsed.as_secs_f64()
        );

        if let Some(err) = &result.error {
            println!("       {err}");
        }
    }

    println!("\n  Passed: {passed}");
    println!("  Failed: {failed}");
}

fn parse_target(token: &str) -> Result<BuildTarget> {
    match token {
        "cli" => Ok(BuildTarget::Cli),
        "ocr" => Ok(BuildTarget::Ocr),
        "stt" => Ok(BuildTarget::Stt),
        "capture" => Ok(BuildTarget::Capture),
        "capture-qt" | "captureqt" | "qt" => Ok(BuildTarget::CaptureQt),
        "desktop" | "tauri" | "app" => Ok(BuildTarget::Desktop),
        _ => anyhow::bail!(
            "Unknown build target '{token}'. Supported targets: ocr, stt, capture, capture-qt, desktop, tauri, app, cli"
        ),
    }
}

fn default_targets() -> Vec<BuildTarget> {
    vec![
        BuildTarget::Ocr,
        BuildTarget::Stt,
        BuildTarget::Capture,
        BuildTarget::Desktop,
    ]
}

fn dedupe_targets(targets: &mut Vec<BuildTarget>) {
    let mut seen = HashSet::new();
    targets.retain(|target| seen.insert(*target));
}

fn parse_bool_env(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub fn desktop() -> Result<()> {
    println!("\nBuilding Tauri desktop app...");
    let ui = ui_dir();
    let app = tauri_dir();
    let tauri_debug = parse_bool_env("SQUIGIT_TAURI_DEBUG") || parse_bool_env("CI");
    let node_bin = ui.join("node_modules").join(".bin");
    if !ui.join("node_modules").exists() {
        println!("\nInstalling npm dependencies...");
        run_cmd("npm", &["install"], &ui)?;
    }

    let mut tauri_args = vec!["build"];
    if cfg!(target_os = "linux") {
        tauri_args.push("--bundles");
        tauri_args.push("appimage");
    } else if cfg!(target_os = "windows") {
        tauri_args.push("--bundles");
        tauri_args.push("nsis");
    } else if cfg!(target_os = "macos") {
        tauri_args.push("--bundles");
        tauri_args.push("dmg");
    }

    let mut env_vars = Vec::new();
    let mut set_env = |key: &str, value: &str| {
        if let Some((_, existing_value)) = env_vars.iter_mut().find(|(k, _)| k == key) {
            *existing_value = value.to_string();
        } else {
            env_vars.push((key.to_string(), value.to_string()));
        }
    };

    if cfg!(target_os = "linux") {
        set_env("APPIMAGE_EXTRACT_AND_RUN", "1");
        set_env("NO_STRIP", "true");
    }

    if tauri_debug {
        set_env("RUST_BACKTRACE", "full");
        set_env("RUST_LOG", "tauri_cli=trace,tauri_bundler=trace");
        set_env("TAURI_LOG_LEVEL", "debug");
        set_env("TAURI_BUNDLER_DEBUG", "1");

        println!("\nTauri debug mode: enabled (SQUIGIT_TAURI_DEBUG=1 or CI=true)");
    } else {
        println!("\nTauri debug mode: disabled");
    }

    println!("\nTauri command: tauri {}", tauri_args.join(" "));
    if env_vars.is_empty() {
        run_cmd_with_node_bin("tauri", &tauri_args, &app, &node_bin)?;
    } else {
        run_cmd_with_node_bin_and_env(
            "tauri",
            &tauri_args,
            &app,
            &node_bin,
            &env_vars,
        )?;
    }

    println!("\nDesktop app build complete!");
    Ok(())
}

fn cli_placeholder() -> Result<()> {
    anyhow::bail!(
        "CLI target is reserved for future work. `apps/cli` is currently not implemented yet."
    )
}

#[cfg(test)]
mod tests {
    use super::{resolve_targets, BuildTarget};

    fn selectors(list: &[&str]) -> Vec<String> {
        list.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn default_build_targets_when_no_args() {
        let actual = resolve_targets(&selectors(&[]), false).expect("resolve default targets");
        assert_eq!(
            actual,
            vec![
                BuildTarget::Ocr,
                BuildTarget::Stt,
                BuildTarget::Capture,
                BuildTarget::Desktop,
            ]
        );
    }

    #[test]
    fn build_all_excluding_ocr() {
        let actual =
            resolve_targets(&selectors(&["all", "-ocr"]), false).expect("resolve all -ocr");
        assert_eq!(
            actual,
            vec![
                BuildTarget::Stt,
                BuildTarget::Capture,
                BuildTarget::Desktop,
            ]
        );
    }
}
