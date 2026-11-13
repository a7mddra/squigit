use anyhow::Result;
use notify_rust::{Notification, NotificationResponse};
use std::path::{Path, PathBuf};

#[cfg(any(windows, target_os = "macos"))]
const APP_ID: &str = "io.github.a7mddra.squigit";

pub fn initialize() -> Result<()> {
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_ID}"))?;
        key.set_value("DisplayName", &"Squigit")?;
        let icon = std::env::current_exe()?.with_file_name("squigit.png");
        key.set_value("IconUri", &icon.to_string_lossy().into_owned())?;
        let app_id = APP_ID.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let status = unsafe {
            windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(app_id.as_ptr())
        };
        anyhow::ensure!(
            status >= 0,
            "Could not set the Squigit Windows app ID: {status:#x}"
        );
    }
    #[cfg(target_os = "macos")]
    notify_rust::set_application(APP_ID)?;
    Ok(())
}

pub fn saved(path: PathBuf) {
    std::thread::spawn(move || {
        if let Err(error) = show_saved(&path) {
            eprintln!("[squigit] Image saved, but its notification failed: {error:#}");
        }
    });
}

pub fn missing_image_hosting_key(settings: rune::AppHandle<crate::settings::App>) {
    image_search_error(
        "Add your ImgBB API key in Image hosting to use Circle to Search, Ask Google, and online QR scanning.".into(),
        true,
        settings,
    );
}

pub fn image_search_error(
    message: String,
    missing_key: bool,
    settings: rune::AppHandle<crate::settings::App>,
) {
    std::thread::spawn(move || {
        if let Err(error) = show_image_search_error(&message, missing_key, settings) {
            eprintln!("[squigit] {message}; notification failed: {error:#}");
        }
    });
}

fn show_image_search_error(
    message: &str,
    missing_key: bool,
    settings: rune::AppHandle<crate::settings::App>,
) -> Result<()> {
    let event_id = format!("{:032x}", rand::random::<u128>());
    let mut notification = Notification::new();
    notification
        .summary(if missing_key {
            "Set up image hosting"
        } else {
            "Image search failed"
        })
        .body(message);
    #[cfg(all(unix, not(target_os = "macos")))]
    notification
        .appname("Squigit")
        .hint(notify_rust::Hint::DesktopEntry("squigit".into()))
        .hint(notify_rust::Hint::Custom(
            "x-squigit-event-id".into(),
            event_id.clone(),
        ));
    #[cfg(windows)]
    notification.app_id(APP_ID);
    if missing_key {
        let settings_action = format!("set-api-{event_id}");
        notification
            .timeout(notify_rust::Timeout::Never)
            .action(&settings_action, "Set API");
        #[cfg(all(unix, not(target_os = "macos")))]
        notification
            .hint(notify_rust::Hint::Resident(true))
            .action("default", "Set API");
        let handle = notification.show()?;
        handle.wait_for_response(|response: &NotificationResponse| {
            if matches!(response, NotificationResponse::Default)
                || matches!(response, NotificationResponse::Action(action) if action == &settings_action)
            {
                settings.update(|state, _| state.open_image_hosting());
                settings.open();
            }
        })?;
    } else {
        notification.show()?;
    }
    Ok(())
}

fn show_saved(path: &Path) -> Result<()> {
    let filename = path
        .file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy();
    let mut notification = Notification::new();
    notification
        .summary("Image saved")
        .body(&filename)
        .image_path(&path.to_string_lossy())
        .action("open-folder", "Open in folder");

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let icon = std::env::current_exe()?.with_file_name("squigit.png");
        notification
            .appname("Squigit")
            .icon(&icon.to_string_lossy())
            .hint(notify_rust::Hint::DesktopEntry("squigit".into()))
            .action("default", "Open in folder");
    }
    #[cfg(windows)]
    notification.app_id(APP_ID);

    notification.show()?.wait_for_response(|response: &NotificationResponse| {
        if matches!(response, NotificationResponse::Default)
            || matches!(response, NotificationResponse::Action(action) if action == "open-folder")
        {
            if let Err(error) = opener::reveal(path) {
                eprintln!("[squigit] Could not reveal the saved image: {error}");
            }
        }
    })?;
    Ok(())
}
