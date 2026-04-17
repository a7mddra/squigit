// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

pub fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        use webbrowser::{open_browser, Browser};
        if open_browser(Browser::Chrome, url).is_ok() || open_browser(Browser::Firefox, url).is_ok()
        {
            return Ok(());
        }
    }

    webbrowser::open(url).map_err(|e| e.to_string())?;
    Ok(())
}
