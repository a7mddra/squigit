#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod about;
mod notifications;
mod settings;

use anyhow::{Context, Result};
use fs2::FileExt;
use rune::{Tray, TrayEvent, TrayIcon, TrayItem, TraySpec, WindowSpec};
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const ICON: &[u8] = include_bytes!("../icons/128x128.png");

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("[squigit] {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let directory =
        std::env::temp_dir().join(format!("squigit-settings-demo-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    Ok(directory.join("config.toml"))
}

fn run() -> Result<ExitCode> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    rune::set_app_id("squigit");
    let _ = rune::set_app_icon(ICON.to_vec());
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("squigit {}", env!("CARGO_PKG_VERSION"));
            return Ok(ExitCode::SUCCESS);
        }
        Some("--help" | "-h") => {
            println!("Squigit\n\nUsage: squigit [OPTION]\n\nNo options: start in the system tray.\n  --squiggle          Open a freeform screenshot overlay\n  --traditional       Open a rectangular screenshot overlay\n  --output-dir <DIR>  Choose where captures are saved\n  --configuration     Open the dummy Rune settings window\n  --about             Open About Squigit\n  --authorize         Request screen capture permission\n  --preflight         Check screen capture permission\n  --version           Print the application version");
            return Ok(ExitCode::SUCCESS);
        }
        Some("--configuration") => {
            settings::app(config_path()?)
                .args(args.into_iter().skip(1))
                .run();
            return Ok(ExitCode::SUCCESS);
        }
        Some("--about") => {
            about::app().args(args.into_iter().skip(1)).run();
            return Ok(ExitCode::SUCCESS);
        }
        Some(_) => return Ok(squigit_qt::run(args).exit_code),
        None => {}
    }

    if let Err(error) = notifications::initialize() {
        eprintln!("[squigit] Could not initialize notifications: {error:#}");
    }

    let lock_dir = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .context("Could not find the runtime directory")?;
    std::fs::create_dir_all(&lock_dir)?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_dir.join("squigit.lock"))?;
    lock.try_lock_exclusive()
        .context("Squigit is already running")?;

    let mut app = settings::app(config_path()?)
        .args(Vec::new())
        .windowless()
        .release_gpu_when_closed()
        .on_quit(|_, _| squigit_qt::cancel_active());
    let settings = app.handle();
    let (about, _) = app.window(WindowSpec::from_app(
        "about",
        about::app().windowless().release_gpu_when_closed(),
    ));
    let icon = image::load_from_memory(ICON)?.into_rgba8();
    let spec = TraySpec::new(
        "squigit",
        "Squigit",
        TrayIcon::rgba(icon.width(), icon.height(), icon.into_raw()),
    )
    .tooltip("Squigit")
    .menu(vec![
        TrayItem::action("capture", "Take screenshot"),
        TrayItem::action("savepath", "Open savepath"),
        TrayItem::action("configuration", "Configuration"),
        TrayItem::action("about", "About"),
        TrayItem::action("quit", "Quit"),
    ]);
    let capturing = Arc::new(AtomicBool::new(false));
    let capture = Arc::new(Mutex::new(None));
    let capture_task = Arc::clone(&capture);
    let tray = Tray::new(spec, move |event| {
        let action = match event {
            TrayEvent::Activate { .. } => "capture".to_owned(),
            TrayEvent::Menu(action) => action,
            _ => return,
        };
        match action.as_str() {
            "capture" => {
                if capturing.swap(true, Ordering::AcqRel) {
                    return;
                }
                let active = Arc::clone(&capturing);
                let task = std::thread::spawn(move || {
                    let outcome = squigit_qt::run(vec!["--squiggle".into()]);
                    active.store(false, Ordering::Release);
                    if let Some(path) = outcome.saved_path {
                        notifications::saved(path);
                    }
                });
                if let Ok(mut capture) = capture_task.lock() {
                    *capture = Some(task);
                }
            }
            "savepath" => {
                std::thread::spawn(|| {
                    if let Err(error) = open_savepath() {
                        eprintln!("[squigit] {error:#}");
                    }
                });
            }
            "configuration" => settings.open(),
            "about" => about.open(),
            "quit" => settings.quit(),
            _ => {}
        }
    })?;
    app.run();
    drop(tray);
    squigit_qt::cancel_active();
    if let Ok(mut capture) = capture.lock() {
        if let Some(task) = capture.take() {
            let _ = task.join();
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn open_savepath() -> Result<()> {
    let path = dirs::picture_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Pictures")))
        .context("Could not find Pictures")?
        .join("squigit");
    std::fs::create_dir_all(&path)?;
    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(&path).status()?;
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(&path).status()?;
    #[cfg(target_os = "windows")]
    let status = Command::new("explorer").arg(&path).status()?;
    anyhow::ensure!(status.success(), "Could not open {}", path.display());
    Ok(())
}
