#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

mod paths;
mod qt_app;

use anyhow::Result;
use qt_app::QtApp;
use std::process::ExitCode;
use sys_shutter_suppressor::AudioGuard;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("[qt-capture] Error: {:#}", e);
            AudioGuard::unmute();
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<ExitCode> {
    let mut app = QtApp::new();
    app.run()
}
