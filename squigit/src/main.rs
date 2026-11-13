#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod about;
mod auth;
mod image_search;
mod notifications;
mod settings;
mod storage;
mod tick;

use anyhow::{Context, Result};
use fs2::FileExt;
use rune::{Tray, TrayEvent, TrayIcon, TrayItem, TraySpec, WindowSpec};
use std::process::{Command, ExitCode};

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
            println!("Squigit\n\nUsage: squigit [OPTION]\n\nNo options: start the tray and persistent Qt menu, initially hidden.\n  --configuration     Open the Rune settings window\n  --about             Open About Squigit\n  --version           Print the application version");
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    let data = storage::AppData::open()?;
    match args.first().map(String::as_str) {
        Some("--configuration") => {
            settings::app().args(args.into_iter().skip(1)).run();
            return Ok(ExitCode::SUCCESS);
        }
        Some("--about") => {
            about::app().args(args.into_iter().skip(1)).run();
            return Ok(ExitCode::SUCCESS);
        }
        Some(argument) => anyhow::bail!("Unknown application argument: {argument}"),
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

    let models = squigit_ocr::models::ModelManager::new(data.clone());
    let mut app = settings::app()
        .args(Vec::new())
        .windowless()
        .release_gpu_when_closed();
    let settings = app.handle();
    let search_settings = settings.clone();
    let tick = tick::Player::start();
    let mut qt = squigit_qt::Runtime::start(
        models,
        storage::snapshots_dir()?,
        data.blobs_dir(),
        move |event| match event {
            squigit_qt::RuntimeEvent::MenuTick => tick.play(),
            squigit_qt::RuntimeEvent::Saved { action, path } if action == "snapshot" => {
                notifications::saved(path);
            }
            squigit_qt::RuntimeEvent::Saved { action, path } if action == "circle-to-search" => {
                image_search::submit(
                    data.clone(),
                    path,
                    image_search::Target::Lens,
                    search_settings.clone(),
                );
            }
            squigit_qt::RuntimeEvent::Saved { .. } => {}
            squigit_qt::RuntimeEvent::Exported { path } => notifications::saved(path),
            squigit_qt::RuntimeEvent::Asked { prompt, path } => {
                image_search::submit(
                    data.clone(),
                    path,
                    image_search::Target::Ai(prompt),
                    search_settings.clone(),
                );
            }
            squigit_qt::RuntimeEvent::Failed(error) => eprintln!("[squigit] {error}"),
            squigit_qt::RuntimeEvent::Stopped => {
                eprintln!("[squigit] The Qt runtime stopped unexpectedly");
            }
        },
    )?;
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
        TrayItem::action("show-menu", "Show menu"),
        TrayItem::action("hide-menu", "Hide menu"),
        TrayItem::action("savepath", "Open savepath"),
        TrayItem::action("configuration", "Configuration"),
        TrayItem::action("about", "About"),
        TrayItem::action("quit", "Quit"),
    ]);
    let menu = qt.handle();
    let tray = Tray::new(spec, move |event| {
        let action = match event {
            TrayEvent::Activate { .. } => "show-menu".to_owned(),
            TrayEvent::Menu(action) => action,
            _ => return,
        };
        match action.as_str() {
            "show-menu" | "hide-menu" => {
                let result = if action == "show-menu" {
                    menu.show_menu()
                } else {
                    menu.hide_menu()
                };
                if let Err(error) = result {
                    eprintln!("[squigit] {error:#}");
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
    qt.shutdown();
    Ok(ExitCode::SUCCESS)
}

fn open_savepath() -> Result<()> {
    let path = storage::snapshots_dir()?;
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
