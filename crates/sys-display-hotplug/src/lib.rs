// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Display hotplug monitor for screen capture
//!
//! Monitors for HDMI/VGA cable plug/unplug events during capture.
//! When topology changes, triggers a callback to kill the capture process.
//! This prevents ghost freezes and jumps to primary screen.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub struct DisplayWatcher {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl DisplayWatcher {
    pub fn start<F>(on_change: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let handle = thread::spawn(move || {
            let mut monitor = DisplayMonitor::new();

            while running_clone.load(Ordering::Relaxed) {
                if monitor.check() {
                    on_change();
                    break;
                }
                thread::sleep(Duration::from_millis(300));
            }
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for DisplayWatcher {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

pub struct DisplayMonitor {
    last_count: i32,
    last_check: Instant,
}

impl DisplayMonitor {
    pub fn new() -> Self {
        Self {
            last_count: Self::get_monitor_count(),
            last_check: Instant::now(),
        }
    }

    pub fn check(&mut self) -> bool {
        if self.last_check.elapsed() < Duration::from_millis(250) {
            return false;
        }
        self.last_check = Instant::now();

        let current = Self::get_monitor_count();
        if current != self.last_count {
            thread::sleep(Duration::from_millis(500));
            let confirmed = Self::get_monitor_count();

            if confirmed != self.last_count {
                self.last_count = confirmed;
                return true;
            }
        }
        false
    }

    #[cfg(target_os = "linux")]
    fn get_monitor_count() -> i32 {
        if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
            let count = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if !name.starts_with("card") || !name.contains('-') {
                        return false;
                    }
                    let status_path = e.path().join("status");
                    std::fs::read_to_string(status_path)
                        .map(|s| s.trim() == "connected")
                        .unwrap_or(false)
                })
                .count();
            if count > 0 {
                return count as i32;
            }
        }
        1
    }

    #[cfg(target_os = "macos")]
    fn get_monitor_count() -> i32 {
        use std::process::Command;
        let out = Command::new("system_profiler")
            .arg("SPDisplaysDataType")
            .output();
        if let Ok(o) = out {
            String::from_utf8_lossy(&o.stdout)
                .matches("Resolution:")
                .count() as i32
        } else {
            1
        }
    }

    #[cfg(target_os = "windows")]
    fn get_monitor_count() -> i32 {
        use windows_sys::core::BOOL;
        use windows_sys::Win32::Foundation::{LPARAM, RECT};
        use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

        unsafe extern "system" fn count_monitor_cb(
            _hmonitor: HMONITOR,
            _hdc: HDC,
            _lprc_monitor: *mut RECT,
            lparam: LPARAM,
        ) -> BOOL {
            let count_ptr = lparam as *mut i32;
            if !count_ptr.is_null() {
                // SAFETY: The caller passes a valid pointer to `count`.
                unsafe {
                    *count_ptr += 1;
                }
            }
            1
        }

        let mut count: i32 = 0;
        // SAFETY: Null HDC/clip is valid to enumerate all displays; callback and lparam are valid.
        let ok = unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(count_monitor_cb),
                (&mut count as *mut i32) as isize,
            )
        };

        if ok != 0 && count > 0 {
            count
        } else {
            1
        }
    }
}

impl Default for DisplayMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor_count_nonzero() {
        let count = DisplayMonitor::get_monitor_count();
        assert!(count >= 1, "Should detect at least one display");
    }

    #[test]
    fn test_watcher_can_stop() {
        let watcher = DisplayWatcher::start(|| {});
        watcher.stop();
    }
}
