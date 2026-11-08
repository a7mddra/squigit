mod native;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const USAGE: &str = "Usage:
  cargo xtask build [--qt]
  cargo xtask run [-- <ARGS>]
  cargo xtask doctor
  cargo xtask fmt [--all]
  cargo xtask clean
  cargo xtask help";

fn main() -> ExitCode {
    match dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch() -> Result<(), String> {
    let root = workspace_root()?;
    let mut arguments = env::args().skip(1);
    let Some(command) = arguments.next() else {
        println!("{USAGE}");
        return Ok(());
    };
    let arguments = arguments.collect::<Vec<_>>();

    match command.as_str() {
        "build" => match arguments.as_slice() {
            [] => native::run(&root, false),
            [flag] if flag == "--qt" => native::run(&root, true),
            _ => Err(format!(
                "unknown argument for build: {}\n\n{USAGE}",
                arguments.join(" ")
            )),
        },
        "run" => run_binary(&root, &arguments),
        "doctor" => {
            reject_arguments("doctor", &arguments)?;
            doctor(&root)
        }
        "fmt" => match arguments.as_slice() {
            [] => fmt(&root, false),
            [flag] if flag == "--all" => fmt(&root, true),
            _ => Err(format!(
                "unknown argument for fmt: {}\n\n{USAGE}",
                arguments.join(" ")
            )),
        },
        "clean" => {
            reject_arguments("clean", &arguments)?;
            clean(&root)
        }
        "help" | "--help" | "-h" => {
            reject_arguments("help", &arguments)?;
            println!("{USAGE}");
            Ok(())
        }
        _ => Err(format!("unknown command '{command}'\n\n{USAGE}")),
    }
}

fn workspace_root() -> Result<PathBuf, String> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "xtask has no workspace parent".to_string())?;
    let expected = workspace
        .canonicalize()
        .map_err(|error| format!("could not resolve the workspace root: {error}"))?;
    let current = env::current_dir()
        .and_then(|directory| directory.canonicalize())
        .map_err(|error| format!("could not resolve the current directory: {error}"))?;

    if current != expected {
        return Err(format!(
            "cargo xtask only runs from the workspace root: {}",
            expected.display()
        ));
    }

    Ok(workspace.to_path_buf())
}

fn reject_arguments(command: &str, arguments: &[String]) -> Result<(), String> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(format!("{command} does not accept arguments\n\n{USAGE}"))
    }
}

fn run_binary(root: &Path, arguments: &[String]) -> Result<(), String> {
    let forwarding = match arguments.first().map(String::as_str) {
        Some("--") => arguments.iter().skip(1).cloned().collect::<Vec<_>>(),
        _ => arguments.to_vec(),
    };

    let host = native::host_target_triple().map_err(|error| error.to_string())?;
    let binary_name = if cfg!(windows) {
        "squigit.exe"
    } else {
        "squigit"
    };
    let binary = root.join("binaries").join(host).join(binary_name);

    if !binary.is_file() {
        return Err(format!(
            "the packaged binary is missing at {}\nrun 'cargo xtask build' first",
            binary.display()
        ));
    }

    let mut command = Command::new(&binary);
    if !forwarding.is_empty() {
        command.args(&forwarding);
    }
    run_command(
        &mut command,
        &format!("{} {}", binary.display(), forwarding.join(" ")),
    )
}

fn doctor(root: &Path) -> Result<(), String> {
    let manifest = root.join("Cargo.toml");
    let mut cargo = cargo();
    cargo
        .args(["check", "--workspace", "--manifest-path"])
        .arg(&manifest)
        .current_dir(root);
    run_command(
        &mut cargo,
        &format!(
            "cargo check --workspace --manifest-path {}",
            manifest.display()
        ),
    )?;

    let native_dir = root.join("native");
    let build_dir = native_dir.join("build");
    let mut cmake = Command::new("cmake");
    cmake
        .arg("-S")
        .arg(&native_dir)
        .arg("-B")
        .arg(&build_dir)
        .current_dir(root);
    if let Ok(qt6_dir) = env::var("Qt6_DIR") {
        cmake.arg(format!("-DCMAKE_PREFIX_PATH={qt6_dir}"));
    } else if let Ok(prefix) = env::var("CMAKE_PREFIX_PATH") {
        cmake.arg(format!("-DCMAKE_PREFIX_PATH={prefix}"));
    }
    run_command(&mut cmake, "cmake configure (native)")
}

fn clean(root: &Path) -> Result<(), String> {
    let manifest = root.join("Cargo.toml");
    let mut cargo = cargo();
    cargo
        .args(["clean", "--manifest-path"])
        .arg(&manifest)
        .current_dir(root);
    run_command(
        &mut cargo,
        &format!("cargo clean --manifest-path {}", manifest.display()),
    )?;

    remove_directory(&root.join("native/build"))?;
    remove_directory(&root.join("native/_internal"))?;
    remove_directory(&root.join("binaries"))
}

fn git_changed_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=ACMR", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run git diff: {e}"))?;

    let mut files: Vec<PathBuf> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| root.join(line))
        .collect();

    let untracked = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run git ls-files: {e}"))?;

    files.extend(
        String::from_utf8_lossy(&untracked.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| root.join(line)),
    );

    let staged = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=ACMR", "--cached"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("could not run git diff --cached: {e}"))?;

    files.extend(
        String::from_utf8_lossy(&staged.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| root.join(line)),
    );

    files.sort();
    files.dedup();

    Ok(files)
}

fn fmt(root: &Path, format_all: bool) -> Result<(), String> {
    if format_all {
        return format_all_files(root);
    }

    let changed = git_changed_files(root)?;
    if changed.is_empty() {
        println!("  No changed files to format.");
        return Ok(());
    }

    let rust_files: Vec<&Path> = changed
        .iter()
        .filter(|f| f.extension() == Some(OsStr::new("rs")))
        .map(|f| f.as_path())
        .collect();

    let prettier_files: Vec<&Path> = changed
        .iter()
        .filter(|f| {
            matches!(
                f.extension().and_then(OsStr::to_str),
                Some("ts" | "tsx" | "js" | "jsx" | "css" | "json" | "md" | "html")
            )
        })
        .map(|f| f.as_path())
        .collect();

    let cpp_files: Vec<&Path> = changed
        .iter()
        .filter(|f| {
            matches!(
                f.extension().and_then(OsStr::to_str),
                Some("cpp" | "h" | "hpp" | "mm" | "c" | "cc")
            )
        })
        .map(|f| f.as_path())
        .collect();

    if !rust_files.is_empty() {
        let mut cmd = Command::new("rustfmt");
        cmd.arg("--edition").arg("2021");
        for file in &rust_files {
            cmd.arg(file);
        }
        run_command(
            &mut cmd,
            &format!(
                "rustfmt ({} file{})",
                rust_files.len(),
                if rust_files.len() == 1 { "" } else { "s" }
            ),
        )?;
    }

    if !prettier_files.is_empty() {
        let mut cmd = Command::new(npx());
        cmd.args(["prettier", "--write"]).current_dir(root);
        for file in &prettier_files {
            cmd.arg(file);
        }
        run_command(
            &mut cmd,
            &format!(
                "prettier --write ({} file{})",
                prettier_files.len(),
                if prettier_files.len() == 1 { "" } else { "s" }
            ),
        )?;
    }

    if !cpp_files.is_empty() && which::which("clang-format").is_ok() {
        let mut cmd = Command::new("clang-format");
        cmd.arg("-i");
        for file in &cpp_files {
            cmd.arg(file);
        }
        run_command(
            &mut cmd,
            &format!(
                "clang-format -i ({} file{})",
                cpp_files.len(),
                if cpp_files.len() == 1 { "" } else { "s" }
            ),
        )?;
    }

    Ok(())
}

fn format_all_files(root: &Path) -> Result<(), String> {
    let manifest = root.join("Cargo.toml");
    let mut cargo = cargo();
    cargo
        .args(["fmt", "--all", "--manifest-path"])
        .arg(&manifest)
        .current_dir(root);
    run_command(
        &mut cargo,
        &format!("cargo fmt --all --manifest-path {}", manifest.display()),
    )?;

    let mut prettier = Command::new(npx());
    prettier
        .args(["prettier", "--write", "."])
        .current_dir(root);
    run_command(&mut prettier, "npx prettier --write .")?;

    if which::which("clang-format").is_ok() {
        format_cpp_files(&root.join("native/src"))?;
    }

    Ok(())
}

fn format_cpp_files(dir: &Path) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let mut files = Vec::new();
    collect_cpp_files(dir, &mut files)?;
    if !files.is_empty() {
        let mut clang = Command::new("clang-format");
        clang.arg("-i");
        for file in &files {
            clang.arg(file);
        }
        run_command(&mut clang, "clang-format -i native/src/**/*")?;
    }
    Ok(())
}

fn collect_cpp_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("could not read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("could not read dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_cpp_files(&path, files)?;
        } else if let Some(ext) = path.extension().and_then(OsStr::to_str) {
            if matches!(ext, "cpp" | "h" | "hpp" | "mm" | "c" | "cc") {
                files.push(path);
            }
        }
    }
    Ok(())
}

fn remove_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    println!("  Removing {}", path.display());
    fs::remove_dir_all(path)
        .map_err(|error| format!("could not remove {}: {error}", path.display()))
}

fn cargo() -> Command {
    Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}

fn npx() -> &'static str {
    if cfg!(windows) {
        "npx.cmd"
    } else {
        "npx"
    }
}

fn run_command(command: &mut Command, display: &str) -> Result<(), String> {
    println!("  $ {display}");
    let status = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("could not start '{display}': {error}"))?;

    if status.success() {
        Ok(())
    } else if let Some(code) = status.code() {
        Err(format!("'{display}' exited with status {code}"))
    } else {
        Err(format!("'{display}' was terminated by a signal"))
    }
}
