// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use crate::console::Ansi;
use anyhow::Result;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use which::which;
use xtask::project_root;

#[derive(Debug, Clone, Copy, Default)]
pub struct SetupOptions {
    pub all: bool,
    pub qt: bool,
    pub py: bool,
    pub cargo: bool,
    pub npm: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Component {
    Qt,
    Py,
    Cargo,
    Npm,
}

impl Component {
    fn label(self) -> &'static str {
        match self {
            Self::Qt => "Qt/CMake",
            Self::Py => "Python",
            Self::Cargo => "Rust/Cargo",
            Self::Npm => "Node/NPM",
        }
    }
}

#[derive(Debug, Clone)]
struct ComponentReport {
    ready: bool,
    details: Vec<String>,
}

impl ComponentReport {
    fn missing(details: Vec<String>) -> Self {
        Self {
            ready: false,
            details,
        }
    }

    fn ready(details: Vec<String>) -> Self {
        Self {
            ready: true,
            details,
        }
    }
}

pub fn run(options: SetupOptions) -> Result<()> {
    let ansi = Ansi::detect();

    println!("\n{}", ansi.bold("Squigit xtask setup"));
    println!("  OS: {}", current_os_label());

    let components = selected_components(options);
    let install_mode_all = options.all;
    let safe_mode = !install_mode_all;

    if install_mode_all {
        println!(
            "{}",
            ansi.yellow("  --all enabled: attempting admin-level installs when possible.")
        );
    } else {
        println!(
            "{}",
            ansi.cyan("  default mode: safe checks + user-level attempts + guided remediation.")
        );
    }

    let mut reports = Vec::new();
    for component in components {
        println!(
            "\n{}",
            ansi.bold(&format!("Checking {}...", component.label()))
        );

        let mut report = check_component(component);
        for line in &report.details {
            println!("  - {}", line);
        }

        if !report.ready {
            if safe_mode {
                attempt_safe_install(component, &ansi);
            }

            if install_mode_all {
                attempt_admin_install(component, &ansi);
            }

            report = check_component(component);
        }

        if report.ready {
            println!("  {}", ansi.green("ready"));
        } else {
            println!("  {}", ansi.red("missing dependencies"));
            for tip in guidance(component) {
                println!("  {}", ansi.yellow(&format!("hint: {tip}")));
            }
        }

        reports.push(report);
    }

    let ready_count = reports.iter().filter(|r| r.ready).count();
    let missing_count = reports.len().saturating_sub(ready_count);

    println!("\n{}", ansi.bold("Setup summary"));
    println!("  ready: {ready_count}");
    println!("  missing: {missing_count}");
    println!(
        "  docs: {}",
        project_root()
            .join("docs")
            .join("03-development")
            .join("DEVELOPMENT.md")
            .display()
    );

    if missing_count > 0 {
        println!(
            "{}",
            ansi.yellow(
                "Some dependencies are still missing; see hints above for exact install commands."
            )
        );
    } else {
        println!("{}", ansi.green("Environment looks contributor-ready."));
    }

    Ok(())
}

fn selected_components(options: SetupOptions) -> Vec<Component> {
    let mut selected = Vec::new();

    if options.qt {
        selected.push(Component::Qt);
    }
    if options.py {
        selected.push(Component::Py);
    }
    if options.cargo {
        selected.push(Component::Cargo);
    }
    if options.npm {
        selected.push(Component::Npm);
    }

    if options.all || selected.is_empty() {
        return vec![
            Component::Qt,
            Component::Py,
            Component::Cargo,
            Component::Npm,
        ];
    }

    selected
}

fn check_component(component: Component) -> ComponentReport {
    match component {
        Component::Qt => check_qt(),
        Component::Py => check_python(),
        Component::Cargo => check_cargo(),
        Component::Npm => check_npm(),
    }
}

fn check_qt() -> ComponentReport {
    let cmake = which("cmake").ok();

    #[cfg(target_os = "windows")]
    let qt_tool = which_any(&["windeployqt", "qmake", "qmake6"]);

    #[cfg(target_os = "macos")]
    let qt_tool = which_any(&["macdeployqt", "qmake6", "qmake"]);

    #[cfg(target_os = "linux")]
    let qt_tool = which_any(&["qmake6", "qmake-qt6", "qmake", "linuxdeployqt"]);

    let mut details = Vec::new();
    details.push(match cmake {
        Some(ref path) => format!("cmake: {}", path.display()),
        None => "cmake: missing".to_string(),
    });
    details.push(match qt_tool {
        Some(ref path) => format!("qt tool: {}", path.display()),
        None => "qt tool: missing (qmake6/qmake/macdeployqt/windeployqt)".to_string(),
    });

    let mut ready = cmake.is_some();

    #[cfg(not(target_os = "windows"))]
    {
        ready &= qt_tool.is_some();
    }

    #[cfg(target_os = "windows")]
    {
        let windows_qt = check_windows_qt_modules();
        details.extend(windows_qt.details);
        if qt_tool.is_none() && windows_qt.ready {
            details.push(
                "qt tool note: not on PATH; using windeployqt from detected Qt kit".to_string(),
            );
        }
        ready &= windows_qt.ready;
    }

    if ready {
        ComponentReport::ready(details)
    } else {
        ComponentReport::missing(details)
    }
}

fn check_python() -> ComponentReport {
    let python = which_any(&["python3", "python"]);
    let pip = which_any(&["pip3", "pip"]);

    let details = vec![
        match python {
            Some(ref path) => format!("python: {}", path.display()),
            None => "python: missing".to_string(),
        },
        match pip {
            Some(ref path) => format!("pip: {}", path.display()),
            None => "pip: missing".to_string(),
        },
    ];

    if python.is_some() && pip.is_some() {
        ComponentReport::ready(details)
    } else {
        ComponentReport::missing(details)
    }
}

fn check_cargo() -> ComponentReport {
    let cargo = which("cargo").ok();
    let rustc = which("rustc").ok();
    let rustup = which("rustup").ok();

    let details = vec![
        match cargo {
            Some(ref path) => format!("cargo: {}", path.display()),
            None => "cargo: missing".to_string(),
        },
        match rustc {
            Some(ref path) => format!("rustc: {}", path.display()),
            None => "rustc: missing".to_string(),
        },
        match rustup {
            Some(ref path) => format!("rustup: {}", path.display()),
            None => "rustup: missing".to_string(),
        },
    ];

    if cargo.is_some() && rustc.is_some() {
        ComponentReport::ready(details)
    } else {
        ComponentReport::missing(details)
    }
}

fn check_npm() -> ComponentReport {
    let node = which("node").ok();
    let npm = which("npm").ok();
    let npx = which("npx").ok();

    let details = vec![
        match node {
            Some(ref path) => format!("node: {}", path.display()),
            None => "node: missing".to_string(),
        },
        match npm {
            Some(ref path) => format!("npm: {}", path.display()),
            None => "npm: missing".to_string(),
        },
        match npx {
            Some(ref path) => format!("npx: {}", path.display()),
            None => "npx: missing".to_string(),
        },
    ];

    if node.is_some() && npm.is_some() && npx.is_some() {
        ComponentReport::ready(details)
    } else {
        ComponentReport::missing(details)
    }
}

fn attempt_safe_install(component: Component, ansi: &Ansi) {
    if component != Component::Cargo {
        return;
    }

    if cfg!(windows) {
        return;
    }

    if which("cargo").is_ok() && which("rustc").is_ok() {
        return;
    }

    if which("curl").is_err() || which("sh").is_err() {
        println!(
            "  {}",
            ansi.yellow("safe install skipped: curl/sh not available for rustup bootstrap")
        );
        return;
    }

    println!(
        "  {}",
        ansi.cyan("safe install attempt: rustup (user-level, no sudo)")
    );

    let status = Command::new("sh")
        .arg("-c")
        .arg("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y")
        .status();

    match status {
        Ok(exit) if exit.success() => {
            println!("  {}", ansi.green("rustup installation finished"));
        }
        Ok(exit) => {
            println!(
                "  {}",
                ansi.yellow(&format!(
                    "rustup installer exited with code {:?}",
                    exit.code()
                ))
            );
        }
        Err(err) => {
            println!(
                "  {}",
                ansi.yellow(&format!("rustup installer failed to launch: {err}"))
            );
        }
    }
}

fn attempt_admin_install(component: Component, ansi: &Ansi) {
    println!("  {}", ansi.cyan("admin install attempt (best effort)"));

    #[cfg(target_os = "linux")]
    {
        attempt_admin_install_linux(component, ansi);
    }

    #[cfg(target_os = "macos")]
    {
        attempt_admin_install_macos(component, ansi);
    }

    #[cfg(target_os = "windows")]
    {
        attempt_admin_install_windows(component, ansi);
    }
}

#[cfg(target_os = "linux")]
fn attempt_admin_install_linux(component: Component, ansi: &Ansi) {
    let (manager, base_cmd, base_args): (&str, &str, Vec<&str>) = if which("apt-get").is_ok() {
        ("apt", "apt-get", vec!["install", "-y"])
    } else if which("dnf").is_ok() {
        ("dnf", "dnf", vec!["install", "-y"])
    } else if which("pacman").is_ok() {
        ("pacman", "pacman", vec!["-S", "--needed", "--noconfirm"])
    } else if which("zypper").is_ok() {
        ("zypper", "zypper", vec!["install", "-y"])
    } else {
        println!(
            "  {}",
            ansi.yellow("No supported Linux package manager detected (apt/dnf/pacman/zypper).")
        );
        return;
    };

    if manager == "apt" {
        run_maybe_sudo("apt-get", &["update"], ansi);
    }

    let mut packages: Vec<&str> = Vec::new();
    match component {
        Component::Qt => {
            if manager == "apt" {
                packages.extend([
                    "cmake",
                    "qt6-base-dev",
                    "qt6-declarative-dev",
                    "qt6-tools-dev",
                    "qt6-5compat-dev",
                ]);
            } else if manager == "dnf" {
                packages.extend([
                    "cmake",
                    "qt6-qtbase-devel",
                    "qt6-qtdeclarative-devel",
                    "qt6-qttools-devel",
                ]);
            } else if manager == "pacman" {
                packages.extend(["cmake", "qt6-base", "qt6-declarative", "qt6-5compat"]);
            } else {
                packages.extend(["cmake", "qt6-base-devel", "qt6-declarative-devel"]);
            }
        }
        Component::Py => {
            if manager == "apt" {
                packages.extend(["python3", "python3-venv", "python3-pip"]);
            } else {
                packages.extend(["python3", "python3-pip"]);
            }
        }
        Component::Cargo => {
            if manager == "apt" {
                packages.extend(["cargo", "rustc"]);
            } else if manager == "dnf" {
                packages.extend(["cargo", "rust"]);
            } else {
                packages.extend(["cargo", "rust"]);
            }
        }
        Component::Npm => {
            packages.extend(["nodejs", "npm"]);
        }
    }

    if packages.is_empty() {
        return;
    }

    let mut args = base_args;
    args.extend(packages.iter().copied());
    run_maybe_sudo(base_cmd, &args, ansi);
}

#[cfg(target_os = "macos")]
fn attempt_admin_install_macos(component: Component, ansi: &Ansi) {
    if which("brew").is_err() {
        println!(
            "  {}",
            ansi.yellow("Homebrew is not installed. See https://brew.sh")
        );
        return;
    }

    let mut formulas = Vec::new();
    match component {
        Component::Qt => formulas.extend(["cmake", "qt@6"]),
        Component::Py => formulas.push("python"),
        Component::Cargo => formulas.push("rustup-init"),
        Component::Npm => formulas.push("node"),
    }

    if formulas.is_empty() {
        return;
    }

    let mut args = vec!["install"];
    args.extend(formulas.iter().copied());
    run_command("brew", &args, ansi);
}

#[cfg(target_os = "windows")]
fn attempt_admin_install_windows(component: Component, ansi: &Ansi) {
    if which("winget").is_err() {
        println!(
            "  {}",
            ansi.yellow("winget was not detected. Run setup from an elevated PowerShell with winget available.")
        );
        return;
    }

    let ids = match component {
        Component::Qt => vec!["QtProject.Qt", "Kitware.CMake"],
        Component::Py => vec!["Python.Python.3.12"],
        Component::Cargo => vec!["Rustlang.Rustup"],
        Component::Npm => vec!["OpenJS.NodeJS.LTS"],
    };

    for id in ids {
        run_command(
            "winget",
            &[
                "install",
                "-e",
                "--id",
                id,
                "--accept-package-agreements",
                "--accept-source-agreements",
            ],
            ansi,
        );
    }

    if component == Component::Qt {
        println!(
            "  {}",
            ansi.yellow(
                "winget Qt package can miss optional modules; install qt5compat + qtshadertools if build/deploy fails."
            )
        );
    }
}

fn run_command(cmd: &str, args: &[&str], ansi: &Ansi) {
    println!("  $ {} {}", cmd, args.join(" "));
    match Command::new(cmd).args(args).status() {
        Ok(status) if status.success() => println!("  {}", ansi.green("ok")),
        Ok(status) => println!(
            "  {}",
            ansi.yellow(&format!("command exited with {:?}", status.code()))
        ),
        Err(err) => println!("  {}", ansi.yellow(&format!("command failed: {err}"))),
    }
}

fn guidance(component: Component) -> Vec<String> {
    let docs = project_root()
        .join("docs")
        .join("03-development")
        .join("DEVELOPMENT.md")
        .display()
        .to_string();

    let mut tips = Vec::new();
    match component {
        Component::Qt => {
            #[cfg(target_os = "linux")]
            tips.push(
                "Ubuntu/Debian: sudo apt-get install -y cmake qt6-base-dev qt6-declarative-dev qt6-tools-dev qt6-5compat-dev".to_string(),
            );
            #[cfg(target_os = "macos")]
            tips.push("macOS: brew install cmake qt@6".to_string());
            #[cfg(target_os = "windows")]
            tips.push(
                "Windows: install Qt 6 + CMake + VS 2022 Build Tools (Desktop C++)".to_string(),
            );
            #[cfg(target_os = "windows")]
            tips.push(
                "Required Qt modules for capture sidecar: qt5compat and qtshadertools.".to_string(),
            );
            #[cfg(target_os = "windows")]
            tips.push(
                "If using aqt: aqt install-qt windows desktop 6.6.0 win64_msvc2019_64 -O %USERPROFILE% -m qt5compat qtshadertools".to_string(),
            );
            tips.push(
                "If Qt is installed but not detected, set PATH/Qt6_DIR to your Qt kit root."
                    .to_string(),
            );
        }
        Component::Py => {
            #[cfg(target_os = "linux")]
            tips.push(
                "Linux: sudo apt-get install -y python3 python3-venv python3-pip".to_string(),
            );
            #[cfg(target_os = "macos")]
            tips.push("macOS: brew install python".to_string());
            #[cfg(target_os = "windows")]
            tips.push("Windows: install Python 3.12+ and ensure it is on PATH.".to_string());
        }
        Component::Cargo => {
            tips.push("Install Rust toolchain: https://rustup.rs".to_string());
            tips.push("Then verify: rustc --version && cargo --version".to_string());
        }
        Component::Npm => {
            #[cfg(target_os = "linux")]
            tips.push("Linux: sudo apt-get install -y nodejs npm".to_string());
            #[cfg(target_os = "macos")]
            tips.push("macOS: brew install node".to_string());
            #[cfg(target_os = "windows")]
            tips.push(
                "Windows: install Node.js LTS and ensure node/npm/npx are on PATH.".to_string(),
            );
        }
    }

    tips.push(format!("For full contributor setup notes see: {docs}"));
    tips
}

fn which_any(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().find_map(|name| which(name).ok())
}

fn current_os_label() -> &'static str {
    if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
struct WindowsQtCheck {
    ready: bool,
    details: Vec<String>,
}

#[cfg(target_os = "windows")]
fn check_windows_qt_modules() -> WindowsQtCheck {
    let candidates = find_windows_qt_candidates();
    if candidates.is_empty() {
        return WindowsQtCheck {
            ready: false,
            details: vec![
                "qt kit: not found (set Qt6_DIR/QTDIR/CMAKE_PREFIX_PATH or install Qt 6 kit)"
                    .to_string(),
            ],
        };
    }

    let mut first_failure = None;
    for prefix in candidates {
        let check = evaluate_windows_qt_prefix(&prefix);
        if check.ready {
            return check;
        }
        if first_failure.is_none() {
            first_failure = Some(check);
        }
    }

    first_failure.unwrap_or_else(|| WindowsQtCheck {
        ready: false,
        details: vec!["qt kit: no usable Qt installation found".to_string()],
    })
}

#[cfg(target_os = "windows")]
fn evaluate_windows_qt_prefix(prefix: &Path) -> WindowsQtCheck {
    let required = [
        ("windeployqt", prefix.join(r"bin\windeployqt.exe")),
        (
            "Qt6 CMake package",
            prefix.join(r"lib\cmake\Qt6\Qt6Config.cmake"),
        ),
        (
            "Qt Quick CMake package",
            prefix.join(r"lib\cmake\Qt6Quick\Qt6QuickConfig.cmake"),
        ),
        (
            "Qt Qml CMake package",
            prefix.join(r"lib\cmake\Qt6Qml\Qt6QmlConfig.cmake"),
        ),
        (
            "Qt5Compat.GraphicalEffects",
            prefix.join(r"qml\Qt5Compat\GraphicalEffects\qmldir"),
        ),
        (
            "Qt6ShaderTools runtime",
            prefix.join(r"bin\Qt6ShaderTools.dll"),
        ),
    ];

    let mut ready = true;
    let mut details = vec![format!("qt kit: {}", prefix.display())];

    for (label, path) in required {
        if path.exists() {
            details.push(format!("{label}: ok"));
        } else {
            ready = false;
            details.push(format!("{label}: missing ({})", path.display()));
        }
    }

    WindowsQtCheck { ready, details }
}

#[cfg(target_os = "windows")]
fn find_windows_qt_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(qt6_dir) = std::env::var("Qt6_DIR") {
        candidates.push(normalize_windows_qt_prefix(PathBuf::from(qt6_dir)));
    }
    if let Ok(qtdir) = std::env::var("QTDIR") {
        candidates.push(PathBuf::from(qtdir));
    }
    if let Ok(prefix_path) = std::env::var("CMAKE_PREFIX_PATH") {
        for path in std::env::split_paths(&prefix_path) {
            candidates.push(normalize_windows_qt_prefix(path));
        }
    }

    for qmake in ["qmake", "qmake6"] {
        if let Ok(output) = Command::new(qmake)
            .args(["-query", "QT_INSTALL_PREFIX"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() {
                    candidates.push(PathBuf::from(path));
                    break;
                }
            }
        }
    }

    for candidate in [
        r"C:\Qt\6.8.0\msvc2022_64",
        r"C:\Qt\6.8.0\msvc2019_64",
        r"C:\Qt\6.7.0\msvc2022_64",
        r"C:\Qt\6.7.0\msvc2019_64",
        r"C:\Qt\6.6.0\msvc2022_64",
        r"C:\Qt\6.6.0\msvc2019_64",
    ] {
        candidates.push(PathBuf::from(candidate));
    }

    if let Some(home) = dirs::home_dir() {
        for version in ["6.8.0", "6.7.0", "6.6.0"] {
            candidates.push(home.join(version).join("msvc2022_64"));
            candidates.push(home.join(version).join("msvc2019_64"));
            candidates.push(home.join("Qt").join(version).join("msvc2022_64"));
            candidates.push(home.join("Qt").join(version).join("msvc2019_64"));
        }
    }

    dedupe_existing_windows_paths(candidates)
}

#[cfg(target_os = "windows")]
fn normalize_windows_qt_prefix(path: PathBuf) -> PathBuf {
    let qt6_config = path.join("Qt6Config.cmake");
    let is_qt6_cmake_dir = path
        .file_name()
        .map(|name| name.to_string_lossy().eq_ignore_ascii_case("Qt6"))
        .unwrap_or(false)
        && qt6_config.exists();

    if is_qt6_cmake_dir {
        if let Some(prefix) = path.parent().and_then(Path::parent).and_then(Path::parent) {
            return prefix.to_path_buf();
        }
    }

    path
}

#[cfg(target_os = "windows")]
fn dedupe_existing_windows_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if !path.exists() {
            continue;
        }
        if out.iter().any(|existing| existing == &path) {
            continue;
        }
        out.push(path);
    }
    out
}
