// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub fn args_request_background<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter()
        .any(|arg| matches!(arg.as_ref(), "--background" | "-b"))
}

pub fn launched_in_background() -> bool {
    args_request_background(std::env::args().skip(1))
}

pub fn get_app_config_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .expect("Could not resolve app config dir")
}

pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use webbrowser::{open_browser, Browser};
        if open_browser(Browser::Chrome, url).is_ok() || open_browser(Browser::Firefox, url).is_ok()
        {
            return Ok(());
        }
    }
    webbrowser::open(url).map_err(|e| e.to_string())
}
