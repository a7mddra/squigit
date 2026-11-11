// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::fs;
#[cfg(not(windows))]
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub type BuildResult<T = ()> = Result<T, String>;

pub fn run(repo_root: &Path, measure_payload: bool) -> BuildResult {
    println!("\nBuilding the native Squigit OCR executable...");
    let native = repo_root.join("native/ocr");
    let venv = native.join("venv");
    let deps_marker = venv.join(".squigit-ocr-deps-v3");
    let force_recreate = std::env::var("SQUIGIT_OCR_RECREATE_VENV")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if venv.exists() && (force_recreate || !deps_marker.exists()) {
        println!("\nRefreshing OCR venv to match dependency baseline...");
        remove_directory(&venv)?;
    }

    if !venv.exists() {
        println!("\nCreating virtual environment...");
        let mut created = false;
        for (command, arguments) in [
            ("python3", &["-m", "venv", "venv"][..]),
            ("python", &["-m", "venv", "venv"][..]),
            ("py", &["-3", "-m", "venv", "venv"][..]),
        ] {
            if run_command(command, arguments, &native).is_ok() {
                created = true;
                break;
            }
        }
        if !created {
            return Err(
                "Failed to create OCR venv. Ensure Python 3 is available via `python3`, `python`, or `py -3`."
                    .to_string(),
            );
        }
    }

    println!("\nInstalling dependencies...");
    let python = venv_python(&native);
    run_command(
        &python,
        &["-m", "pip", "install", "-r", "requirements-build.txt"],
        &native,
    )?;
    run_command(
        &python,
        &[
            "-m",
            "pip",
            "install",
            "--no-deps",
            "-r",
            "requirements-core.txt",
        ],
        &native,
    )?;
    run_command(
        &python,
        &["-m", "pip", "install", "-r", "requirements-runtime.txt"],
        &native,
    )?;

    #[cfg(target_os = "macos")]
    {
        println!("\nApplying macOS NumPy compatibility pin...");
        run_command(
            &python,
            &["-m", "pip", "install", "--force-reinstall", "numpy==1.26.4"],
            &native,
        )?;
    }

    println!("\nApplying patches...");
    run_command(&python, &["patches/paddle_core.py"], &native)?;
    run_command(&python, &["patches/paddlex_official_models.py"], &native)?;
    run_command(&python, &["patches/paddlex_deps.py"], &native)?;
    run_command(
        &python,
        &["patches/paddlex_image_batch_sampler.py"],
        &native,
    )?;

    run_command(
        &python,
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
        &native,
    )?;
    run_command(
        &python,
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
        &native,
    )?;
    fs::write(&deps_marker, "v3\n").map_err(|error| {
        format!(
            "Could not write OCR dependency marker {}: {error}",
            deps_marker.display()
        )
    })?;

    println!("\nDownloading models...");
    #[cfg(windows)]
    run_command(&python, &["scripts/download_models.py"], &native)?;
    #[cfg(not(windows))]
    run_command(
        &python,
        &["scripts/download_models.py", "--clean-stale"],
        &native,
    )?;

    println!("\nRunning OCR runtime smoke check...");
    run_command(&python, &["scripts/smoke_runtime.py"], &native)?;

    println!("\nBuilding executable...");
    run_command(
        &python,
        &["-m", "PyInstaller", "--clean", "-y", "ocr-engine.spec"],
        &native,
    )?;

    #[cfg(not(windows))]
    {
        println!("\nChecking the built OCR executable...");
        let dist_executable_onedir = native.join("dist/squigit-ocr/squigit-ocr");
        let dist_executable_onefile = native.join("dist/squigit-ocr");
        let dist_executable = if dist_executable_onedir.exists() {
            dist_executable_onedir
        } else {
            dist_executable_onefile
        };
        smoke_packaged_ocr(&python, &native, &dist_executable)?;
    }

    let host_triple = host_target_triple()?;
    let packaged_runtime = package(repo_root, &native, &host_triple)?;

    #[cfg(not(windows))]
    {
        println!("\nChecking the packaged OCR executable...");
        smoke_packaged_ocr(&python, &native, &packaged_runtime.join("squigit-ocr"))?;
    }

    if measure_payload || parse_bool_env("SQUIGIT_OCR_MEASURE_SIZE") {
        measure_payload_size(repo_root, &native, &python, &packaged_runtime, &host_triple)?;
    } else {
        println!("\nSkipping OCR payload size measurement (disabled by default).");
    }

    println!("\nNative Squigit OCR build complete!");
    Ok(())
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

fn package(repo_root: &Path, native: &Path, host_triple: &str) -> BuildResult<PathBuf> {
    println!("\nPackaging the native OCR runtime for distribution...");

    let dist_dir = native.join("dist");
    let package_binaries = repo_root.join("binaries");
    fs::create_dir_all(&package_binaries).map_err(|error| {
        format!(
            "Could not create packaging directory {}: {error}",
            package_binaries.display()
        )
    })?;

    let binary_name = format!("squigit-ocr{}", if cfg!(windows) { ".exe" } else { "" });
    let source_binary = dist_dir.join(&binary_name);
    let source_runtime = dist_dir.join("squigit-ocr");
    let destination = paddle_package_destination(repo_root, host_triple);

    if source_runtime.is_dir() {
        println!("  Copying OCR runtime dir to {}", destination.display());
        remove_existing_path(&destination)?;
        copy_runtime_directory(&source_runtime, &destination)?;
        return Ok(destination);
    }

    if source_binary.exists() {
        remove_existing_path(&destination)?;
        fs::create_dir_all(&destination).map_err(|error| {
            format!(
                "Could not create OCR package directory {}: {error}",
                destination.display()
            )
        })?;
        let destination_binary = destination.join(&binary_name);
        println!(
            "  Copying legacy OCR binary to {}",
            destination_binary.display()
        );
        fs::copy(&source_binary, &destination_binary).map_err(|error| {
            format!(
                "Could not copy OCR binary {} to {}: {error}",
                source_binary.display(),
                destination_binary.display()
            )
        })?;
        return Ok(destination);
    }

    Err(format!(
        "OCR artifacts not found. Expected runtime dir {} or binary {}",
        source_runtime.display(),
        source_binary.display()
    ))
}

fn paddle_package_destination(repo_root: &Path, host_triple: &str) -> PathBuf {
    repo_root
        .join("binaries")
        .join(format!("paddle-ocr-{host_triple}"))
}

fn measure_payload_size(
    repo_root: &Path,
    native: &Path,
    python: &Path,
    packaged_runtime: &Path,
    host_triple: &str,
) -> BuildResult {
    println!("\nMeasuring OCR payload size...");
    if !packaged_runtime.exists() {
        println!(
            "  [warn] OCR payload path not found for size report: {}",
            packaged_runtime.display()
        );
        return Ok(());
    }

    let reports_dir = repo_root.join("target/ocr-size");
    fs::create_dir_all(&reports_dir).map_err(|error| {
        format!(
            "Could not create OCR size report directory {}: {error}",
            reports_dir.display()
        )
    })?;
    let report_path = reports_dir.join(format!("ocr-size-{host_triple}.json"));
    let input = packaged_runtime.to_string_lossy().into_owned();
    let output = report_path.to_string_lossy().into_owned();

    #[cfg(windows)]
    let arguments = vec![
        "scripts/measure_runtime_size.py",
        "--input",
        input.as_str(),
        "--output",
        output.as_str(),
    ];
    #[cfg(not(windows))]
    let arguments = vec![
        "scripts/measure_runtime_size.py",
        "--input",
        input.as_str(),
        "--output",
        output.as_str(),
        "--preserve-symlinks",
    ];

    run_command(python, &arguments, native)
}

#[cfg(not(windows))]
fn smoke_packaged_ocr(python: &Path, native: &Path, executable: &Path) -> BuildResult {
    let executable = executable.to_string_lossy().into_owned();
    run_command(
        python,
        &["scripts/smoke_executable.py", "--executable", &executable],
        native,
    )
}

fn venv_python(native: &Path) -> PathBuf {
    if cfg!(windows) {
        native.join("venv/Scripts/python.exe")
    } else {
        native.join("venv/bin/python")
    }
}

fn host_target_triple() -> BuildResult<String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("Could not inspect the Rust host target: {error}"))?;
    if !output.status.success() {
        return Err("Could not inspect the Rust host target with 'rustc -vV'.".to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("host: ").map(str::trim))
        .filter(|host| !host.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "Rust did not report a host target in 'rustc -vV'.".to_string())
}

fn run_command(command: impl AsRef<Path>, arguments: &[&str], cwd: &Path) -> BuildResult {
    let command = command.as_ref();
    println!("  $ {} {}", command.display(), arguments.join(" "));
    let status = Command::new(command)
        .args(arguments)
        .current_dir(cwd)
        .status()
        .map_err(|error| {
            format!(
                "Failed to run {} {:?}: {error}",
                command.display(),
                arguments
            )
        })?;

    if status.success() {
        Ok(())
    } else if let Some(code) = status.code() {
        Err(format!(
            "Command {} failed with exit code {code}.",
            command.display()
        ))
    } else {
        Err(format!(
            "Command {} was terminated by a signal.",
            command.display()
        ))
    }
}

fn remove_directory(path: &Path) -> BuildResult {
    fs::remove_dir_all(path)
        .map_err(|error| format!("Could not remove directory {}: {error}", path.display()))
}

fn remove_existing_path(path: &Path) -> BuildResult {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Could not inspect {}: {error}", path.display())),
    };

    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("Could not remove directory {}: {error}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|error| format!("Could not remove file {}: {error}", path.display()))
    }
}

#[cfg(windows)]
fn copy_runtime_directory(source: &Path, destination: &Path) -> BuildResult {
    copy_directory(source, destination)
}

#[cfg(not(windows))]
fn copy_runtime_directory(source: &Path, destination: &Path) -> BuildResult {
    copy_directory_preserving_symlinks(source, destination)?;
    verify_symlink_integrity(source, destination)
}

#[cfg(windows)]
fn copy_directory(source: &Path, destination: &Path) -> BuildResult {
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "Could not create directory {}: {error}",
            destination.display()
        )
    })?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("Could not read directory {}: {error}", source.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Could not read an entry in {}: {error}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect {}: {error}", source_path.display()))?;
        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "Could not copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn copy_directory_preserving_symlinks(source: &Path, destination: &Path) -> BuildResult {
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "Could not create directory {}: {error}",
            destination.display()
        )
    })?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("Could not read directory {}: {error}", source.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Could not read an entry in {}: {error}", source.display()))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect {}: {error}", source_path.display()))?;

        if file_type.is_dir() {
            copy_directory_preserving_symlinks(&source_path, &destination_path)?;
        } else if file_type.is_symlink() {
            remove_existing_path(&destination_path)?;
            let target = fs::read_link(&source_path).map_err(|error| {
                format!("Could not read symlink {}: {error}", source_path.display())
            })?;
            unix_fs::symlink(&target, &destination_path).map_err(|error| {
                format!(
                    "Could not create symlink {} -> {}: {error}",
                    destination_path.display(),
                    target.display()
                )
            })?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "Could not copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn verify_symlink_integrity(source: &Path, destination: &Path) -> BuildResult {
    let source_count = count_symlinks(source)?;
    let destination_count = count_symlinks(destination)?;
    if source_count == destination_count {
        Ok(())
    } else {
        Err(format!(
            "OCR runtime symlink integrity failed: src={} dst={} ({} -> {})",
            source_count,
            destination_count,
            source.display(),
            destination.display()
        ))
    }
}

#[cfg(not(windows))]
fn count_symlinks(path: &Path) -> BuildResult<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(path)
        .map_err(|error| format!("Could not read directory {}: {error}", path.display()))?
    {
        let entry = entry
            .map_err(|error| format!("Could not read an entry in {}: {error}", path.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Could not inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            count += 1;
        } else if file_type.is_dir() {
            count += count_symlinks(&entry.path())?;
        }
    }
    Ok(count)
}
