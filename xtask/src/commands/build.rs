// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use xtask::{capture_sidecar_dir, qt_native_dir};
use xtask::{get_host_target_triple, ocr_sidecar_dir, venv_python};
use xtask::{project_root, run_cmd, run_cmd_with_node_bin, run_cmd_with_node_bin_and_env};
use xtask::{tauri_dir, ui_dir};

use crate::commands::pkg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildTarget {
    Ocr,
    Whisper,
    Capture,
    CaptureQt,
    Desktop,
    Cli,
}

impl BuildTarget {
    fn key(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::Whisper => "whisper",
            Self::Capture => "capture",
            Self::CaptureQt => "capture-qt",
            Self::Desktop => "desktop",
            Self::Cli => "cli",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Ocr => "PaddleOCR",
            Self::Whisper => "Whisper STT",
            Self::Capture => "Capture Engine",
            Self::CaptureQt => "Capture Qt",
            Self::Desktop => "Desktop (Tauri)",
            Self::Cli => "CLI (reserved)",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BuildCommandOptions {
    pub selectors: Vec<String>,
    pub include_all: bool,
    pub measure_ocr_size: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OcrBuildOptions {
    pub measure_payload_size: bool,
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

    let measure_ocr_size =
        should_measure_ocr_size(options.measure_ocr_size || inline_flags.measure_ocr_size);

    println!("\nBuild plan:");
    for target in &targets {
        println!("  - {} ({})", target.label(), target.key());
    }
    if measure_ocr_size {
        println!("  - OCR size measurement: enabled");
    } else {
        println!("  - OCR size measurement: disabled (use --measure-ocr-size or SQUIGIT_OCR_MEASURE_SIZE=1)");
    }

    let mut results = Vec::with_capacity(targets.len());
    for target in targets {
        let started = Instant::now();
        let result = match target {
            BuildTarget::Ocr => ocr_with_options(OcrBuildOptions {
                measure_payload_size: measure_ocr_size,
            }),
            BuildTarget::Whisper => whisper(),
            BuildTarget::Capture => capture(),
            BuildTarget::CaptureQt => capture_qt_only(),
            BuildTarget::Desktop => desktop(),
            BuildTarget::Cli => cli_placeholder(),
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
        "ocr" => Ok(BuildTarget::Ocr),
        "whisper" => Ok(BuildTarget::Whisper),
        "capture" => Ok(BuildTarget::Capture),
        "capture-qt" | "captureqt" | "qt" => Ok(BuildTarget::CaptureQt),
        "desktop" | "tauri" | "app" => Ok(BuildTarget::Desktop),
        "cli" => Ok(BuildTarget::Cli),
        _ => anyhow::bail!(
            "Unknown build target '{token}'. Supported targets: ocr, whisper, capture, capture-qt, desktop, tauri, app, cli"
        ),
    }
}

fn default_targets() -> Vec<BuildTarget> {
    vec![
        BuildTarget::Ocr,
        BuildTarget::Whisper,
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

fn should_measure_ocr_size(cli_flag: bool) -> bool {
    cli_flag || parse_bool_env("SQUIGIT_OCR_MEASURE_SIZE")
}

pub fn capture() -> Result<()> {
    println!("\nBuilding Capture Engine...");
    build_qt_native()?;
    println!("\nDeploying Qt runtime...");
    deploy_qt_native()?;
    #[cfg(target_os = "macos")]
    {
        println!("\nSigning macOS bundle...");
        crate::platforms::macos::sign(&qt_native_dir())?;
    }
    build_capture_rust_wrapper()?;
    pkg::capture()?;
    println!("\nCapture Engine build complete!");
    Ok(())
}

pub fn capture_qt_only() -> Result<()> {
    println!("\nBuilding Qt native binary (CMake only)...");
    build_qt_native()?;
    println!("\nQt build complete!");
    Ok(())
}

fn build_qt_native() -> Result<()> {
    println!("\nRunning Qt CMake build...");
    let native_dir = qt_native_dir();
    #[cfg(target_os = "linux")]
    crate::platforms::linux::build(&native_dir)?;
    #[cfg(target_os = "macos")]
    crate::platforms::macos::build(&native_dir)?;
    #[cfg(target_os = "windows")]
    crate::platforms::win::build(&native_dir)?;
    Ok(())
}

fn deploy_qt_native() -> Result<()> {
    let native_dir = qt_native_dir();
    #[cfg(target_os = "linux")]
    crate::platforms::linux::deploy(&native_dir)?;
    #[cfg(target_os = "macos")]
    crate::platforms::macos::deploy(&native_dir)?;
    #[cfg(target_os = "windows")]
    crate::platforms::win::deploy(&native_dir)?;
    Ok(())
}

fn build_capture_rust_wrapper() -> Result<()> {
    println!("\nBuilding Rust wrapper...");
    let _sidecar = capture_sidecar_dir();
    run_cmd(
        "cargo",
        &["build", "--release", "-p", "capture-engine"],
        &project_root(),
    )?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn smoke_packaged_sidecar(py: &str, sidecar_dir: &Path, sidecar_path: &Path) -> Result<()> {
    let sidecar_str = sidecar_path.to_string_lossy().to_string();
    run_cmd(
        py,
        &["scripts/smoke_sidecar.py", "--sidecar", &sidecar_str],
        sidecar_dir,
    )
}

pub fn ocr_with_options(options: OcrBuildOptions) -> Result<()> {
    println!("\nBuilding PaddleOCR sidecar...");
    let sidecar = ocr_sidecar_dir();
    let venv = sidecar.join("venv");
    let deps_marker = venv.join(".squigit-ocr-deps-v3");
    let force_recreate = std::env::var("SQUIGIT_OCR_RECREATE_VENV")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if venv.exists() && (force_recreate || !deps_marker.exists()) {
        println!("\nRefreshing OCR venv to match dependency baseline...");
        fs::remove_dir_all(&venv)?;
    }

    if !venv.exists() {
        println!("\nCreating virtual environment...");
        let mut created = false;

        for (cmd, args) in [
            ("python3", vec!["-m", "venv", "venv"]),
            ("python", vec!["-m", "venv", "venv"]),
            ("py", vec!["-3", "-m", "venv", "venv"]),
        ] {
            if run_cmd(cmd, &args, &sidecar).is_ok() {
                created = true;
                break;
            }
        }

        if !created {
            anyhow::bail!(
                "Failed to create OCR venv. Ensure Python 3 is available via `python3`, `python`, or `py -3`."
            );
        }
    }
    println!("\nInstalling dependencies...");
    let python = venv_python();
    let py = python.to_str().unwrap();
    run_cmd(
        py,
        &["-m", "pip", "install", "-r", "requirements-build.txt"],
        &sidecar,
    )?;
    run_cmd(
        py,
        &[
            "-m",
            "pip",
            "install",
            "--no-deps",
            "-r",
            "requirements-core.txt",
        ],
        &sidecar,
    )?;
    run_cmd(
        py,
        &["-m", "pip", "install", "-r", "requirements-runtime.txt"],
        &sidecar,
    )?;
    #[cfg(target_os = "macos")]
    {
        println!("\nApplying macOS NumPy compatibility pin...");
        run_cmd(
            py,
            &["-m", "pip", "install", "--force-reinstall", "numpy==1.26.4"],
            &sidecar,
        )?;
    }

    println!("\nApplying patches...");
    run_cmd(py, &["patches/paddle_core.py"], &sidecar)?;
    run_cmd(py, &["patches/paddlex_official_models.py"], &sidecar)?;
    run_cmd(py, &["patches/paddlex_deps.py"], &sidecar)?;
    run_cmd(py, &["patches/paddlex_image_batch_sampler.py"], &sidecar)?;

    run_cmd(
        py,
        &[
            "-m",
            "pip",
            "uninstall",
            "-y",
            "modelscope",
            "huggingface-hub",
            "hf-xet",
            "pypdfium2",
            "pypdfium2-raw",
            "opencv-contrib-python",
            "rich",
            "typer",
            "markdown-it-py",
            "mdurl",
        ],
        &sidecar,
    )?;
    run_cmd(
        py,
        &[
            "-c",
            r###"import pathlib
import re
import sys
from importlib import metadata

req = {}
for req_file in ("requirements-core.txt", "requirements-build.txt", "requirements-runtime.txt"):
    for line in pathlib.Path(req_file).read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        line = line.split("#", 1)[0].strip()
        if "==" not in line:
            continue
        name, version = line.split("==", 1)
        name = re.split(r"[;\s]", name.strip(), maxsplit=1)[0]
        req[name.lower().replace("_", "-")] = version.strip()

errors = []
for package, expected in req.items():
    try:
        actual = metadata.version(package)
    except Exception as exc:
        errors.append(f"{package}: metadata lookup failed: {exc}")
        continue

    if actual != expected:
        errors.append(f"{package}: expected {expected}, got {actual}")

if errors:
    print("OCR dependency verification failed:")
    print("\n".join(errors))
    sys.exit(1)

print("OCR dependency verification passed.")"###,
        ],
        &sidecar,
    )?;
    fs::write(&deps_marker, "v3\n")?;

    println!("\nDownloading models...");
    #[cfg(target_os = "windows")]
    run_cmd(py, &["download_models.py"], &sidecar)?;
    #[cfg(not(target_os = "windows"))]
    run_cmd(py, &["download_models.py", "--clean-stale"], &sidecar)?;
    println!("\nRunning OCR runtime smoke check...");
    run_cmd(py, &["scripts/smoke_runtime.py"], &sidecar)?;

    println!("\nBuilding executable...");
    run_cmd(
        py,
        &["-m", "PyInstaller", "--clean", "-y", "ocr-engine.spec"],
        &sidecar,
    )?;

    #[cfg(not(target_os = "windows"))]
    {
        println!("\nRunning dist sidecar smoke checks...");
        let dist_sidecar = sidecar.join("dist").join("ocr-engine").join("ocr-engine");
        smoke_packaged_sidecar(py, &sidecar, &dist_sidecar)?;
    }

    pkg::ocr()?;

    let host_triple = get_host_target_triple()?;
    let app_binaries = project_root().join("apps").join("desktop").join("binaries");

    #[cfg(not(target_os = "windows"))]
    {
        println!("\nRunning packaged sidecar smoke checks...");
        let packaged_sidecar = app_binaries
            .join(format!("paddle-ocr-{}", host_triple))
            .join("ocr-engine");
        smoke_packaged_sidecar(py, &sidecar, &packaged_sidecar)?;
    }

    let measure_payload_size = should_measure_ocr_size(options.measure_payload_size);
    if measure_payload_size {
        println!("\nMeasuring OCR payload size...");
        let runtime_dir = app_binaries.join(format!("paddle-ocr-{}", host_triple));
        let legacy_bin = app_binaries.join(format!(
            "ocr-engine-{}{}",
            host_triple,
            if cfg!(windows) { ".exe" } else { "" }
        ));
        let size_input = if runtime_dir.exists() {
            runtime_dir
        } else {
            legacy_bin
        };
        if size_input.exists() {
            let reports_dir = project_root().join("target").join("ocr-size");
            fs::create_dir_all(&reports_dir)?;
            let report_path = reports_dir.join(format!("ocr-size-{}.json", host_triple));
            let size_input_str = size_input.to_string_lossy().to_string();
            let report_path_str = report_path.to_string_lossy().to_string();
            #[cfg(target_os = "windows")]
            let measure_args = vec![
                "scripts/measure_runtime_size.py".to_string(),
                "--input".to_string(),
                size_input_str,
                "--output".to_string(),
                report_path_str,
            ];
            #[cfg(not(target_os = "windows"))]
            let mut measure_args = vec![
                "scripts/measure_runtime_size.py".to_string(),
                "--input".to_string(),
                size_input_str,
                "--output".to_string(),
                report_path_str,
            ];
            #[cfg(not(target_os = "windows"))]
            measure_args.push("--preserve-symlinks".to_string());

            let measure_arg_refs: Vec<&str> = measure_args.iter().map(String::as_str).collect();
            run_cmd(py, &measure_arg_refs, &sidecar)?;
        } else {
            println!(
                "  [warn] OCR payload path not found for size report: {}",
                size_input.display()
            );
        }
    } else {
        println!("\nSkipping OCR payload size measurement (disabled by default).");
    }

    println!("\nSidecar build complete!");
    Ok(())
}

pub fn desktop() -> Result<()> {
    println!("\nBuilding Tauri desktop app...");
    let ui = ui_dir();
    let app = tauri_dir();
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
    if cfg!(target_os = "linux") {
        let host_triple = get_host_target_triple()?;
        let ocr_sidecar_path = project_root()
            .join("apps")
            .join("desktop")
            .join("binaries")
            .join(format!("paddle-ocr-{}", host_triple))
            .join("_internal");

        if ocr_sidecar_path.exists() {
            println!("  [info] Applying LD_LIBRARY_PATH for paddle-ocr sidecar");
            env_vars.push((
                "LD_LIBRARY_PATH".to_string(),
                ocr_sidecar_path.to_string_lossy().to_string(),
            ));
        }
    }

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

pub fn whisper() -> Result<()> {
    println!("\nBuilding Whisper STT sidecar...");
    let sidecar = xtask::whisper_sidecar_dir();
    let build_dir = sidecar.join("build");

    refresh_whisper_cmake_cache_if_stale(&sidecar, &build_dir)?;
    fs::create_dir_all(&build_dir)?;

    let source_dir = sidecar.to_string_lossy().to_string();
    let build_dir_str = build_dir.to_string_lossy().to_string();

    println!("\nRunning CMake config...");
    run_cmd(
        "cmake",
        &[
            "-S",
            &source_dir,
            "-B",
            &build_dir_str,
            "-DCMAKE_BUILD_TYPE=Release",
        ],
        &project_root(),
    )?;

    println!("\nRunning CMake build...");
    run_cmd(
        "cmake",
        &["--build", &build_dir_str, "--config", "Release"],
        &project_root(),
    )?;

    println!("\nSidecar build complete!");
    crate::commands::pkg::whisper()?;
    Ok(())
}

fn refresh_whisper_cmake_cache_if_stale(sidecar: &Path, build_dir: &Path) -> Result<()> {
    let cache_path = build_dir.join("CMakeCache.txt");
    if !cache_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&cache_path).with_context(|| {
        format!(
            "Failed reading Whisper cache file for validation: {}",
            cache_path.display()
        )
    })?;

    let mut cached_home_dir = None;
    let mut cached_cache_dir = None;
    for line in content.lines() {
        if let Some(value) = line.strip_prefix("CMAKE_HOME_DIRECTORY:INTERNAL=") {
            cached_home_dir = Some(value.trim().to_string());
        }
        if let Some(value) = line.strip_prefix("CMAKE_CACHEFILE_DIR:INTERNAL=") {
            cached_cache_dir = Some(value.trim().to_string());
        }
    }

    let expected_home = normalize_path(
        &sidecar
            .canonicalize()
            .unwrap_or_else(|_| sidecar.to_path_buf()),
    );
    let expected_cache = normalize_path(
        &build_dir
            .canonicalize()
            .unwrap_or_else(|_| build_dir.to_path_buf()),
    );

    let home_mismatch = cached_home_dir
        .as_deref()
        .map(normalize_path_str)
        .map(|value| value != expected_home)
        .unwrap_or(false);

    let cache_mismatch = cached_cache_dir
        .as_deref()
        .map(normalize_path_str)
        .map(|value| value != expected_cache)
        .unwrap_or(false);

    if home_mismatch || cache_mismatch {
        println!(
            "  Detected stale Whisper CMake cache from different source/build path; recreating build directory..."
        );
        fs::remove_dir_all(build_dir)?;
    }

    Ok(())
}

fn normalize_path(path: &Path) -> String {
    normalize_path_str(path.to_string_lossy().as_ref())
}

fn normalize_path_str(value: &str) -> String {
    let normalized = value.replace('\\', "/").trim_end_matches('/').to_string();

    if cfg!(target_os = "windows") {
        normalized.to_ascii_lowercase()
    } else {
        normalized
    }
}

fn cli_placeholder() -> Result<()> {
    anyhow::bail!(
        "CLI target is reserved for future work. `apps/cli` is currently not implemented yet."
    )
}

#[cfg(test)]
mod tests {
    use super::{extract_inline_flags, resolve_targets, BuildTarget};

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
                BuildTarget::Whisper,
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
                BuildTarget::Whisper,
                BuildTarget::Capture,
                BuildTarget::Desktop,
            ]
        );
    }

    #[test]
    fn aliases_and_dedupe_are_supported() {
        let actual = resolve_targets(
            &selectors(&["desktop", "tauri", "app", "whisper", "whisper"]),
            false,
        )
        .expect("resolve aliases");
        assert_eq!(actual, vec![BuildTarget::Desktop, BuildTarget::Whisper]);
    }

    #[test]
    fn explicit_targets_extend_all_selection() {
        let actual = resolve_targets(&selectors(&["all", "capture-qt", "-ocr"]), false)
            .expect("resolve mixed");
        assert_eq!(
            actual,
            vec![
                BuildTarget::Whisper,
                BuildTarget::Capture,
                BuildTarget::Desktop,
                BuildTarget::CaptureQt,
            ]
        );
    }

    #[test]
    fn invalid_target_returns_error() {
        let err = resolve_targets(&selectors(&["unknown"]), false)
            .expect_err("unknown selector should fail")
            .to_string();
        assert!(err.contains("Unknown build target"));
    }

    #[test]
    fn inline_measure_flag_is_extracted() {
        let mut items = selectors(&["cli", "--measure-ocr-size"]);
        let flags = extract_inline_flags(&mut items);
        assert!(flags.measure_ocr_size);
        assert_eq!(items, vec!["cli".to_string()]);
    }
}
