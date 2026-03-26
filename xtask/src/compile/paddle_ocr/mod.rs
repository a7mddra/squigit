// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use std::fs;
#[cfg(not(target_os = "windows"))]
use std::path::Path;
use xtask::{get_host_target_triple, ocr_sidecar_dir, project_root, run_cmd, venv_python};

#[derive(Debug, Clone, Copy, Default)]
pub struct OcrBuildOptions {
    pub measure_payload_size: bool,
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

pub fn build(options: OcrBuildOptions) -> Result<()> {
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
        let dist_sidecar_onedir = sidecar.join("dist").join("squigit-ocr").join("squigit-ocr");
        let dist_sidecar_onefile = sidecar.join("dist").join("squigit-ocr");
        let dist_sidecar = if dist_sidecar_onedir.exists() {
            dist_sidecar_onedir
        } else {
            dist_sidecar_onefile
        };
        smoke_packaged_sidecar(py, &sidecar, &dist_sidecar)?;
    }

    crate::packaging::paddle_ocr::ocr()?;

    let host_triple = get_host_target_triple()?;
    let pkg_binaries = project_root().join("packaging").join("binaries");

    #[cfg(not(target_os = "windows"))]
    {
        println!("\nRunning packaged sidecar smoke checks...");
        let packaged_sidecar = pkg_binaries
            .join(format!("paddle-ocr-{}", host_triple))
            .join("squigit-ocr");
        smoke_packaged_sidecar(py, &sidecar, &packaged_sidecar)?;
    }

    let measure_payload_size = should_measure_ocr_size(options.measure_payload_size);
    if measure_payload_size {
        println!("\nMeasuring OCR payload size...");
        let runtime_dir = pkg_binaries.join(format!("paddle-ocr-{}", host_triple));
        let size_input = runtime_dir.clone();
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
