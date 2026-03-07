// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy)]
pub struct Ansi {
    enabled: bool,
}

impl Ansi {
    pub fn detect() -> Self {
        if env::var_os("NO_COLOR").is_some() {
            return Self { enabled: false };
        }

        if env::var("CLICOLOR_FORCE")
            .map(|value| value.trim() == "1")
            .unwrap_or(false)
        {
            return Self { enabled: true };
        }

        Self {
            enabled: io::stdout().is_terminal(),
        }
    }

    pub fn green(self, text: &str) -> String {
        self.wrap("32", text)
    }

    pub fn red(self, text: &str) -> String {
        self.wrap("31", text)
    }

    pub fn yellow(self, text: &str) -> String {
        self.wrap("33", text)
    }

    pub fn cyan(self, text: &str) -> String {
        self.wrap("36", text)
    }

    pub fn bold(self, text: &str) -> String {
        self.wrap("1", text)
    }

    fn wrap(self, code: &str, text: &str) -> String {
        if !self.enabled {
            return text.to_string();
        }
        format!("\x1b[{code}m{text}\x1b[0m")
    }
}
