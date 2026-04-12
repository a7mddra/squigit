// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use xtask::{project_root, run_cmd};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTarget {
    Auth,
    AuthLive,
    All,
}

impl TestTarget {
    pub fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "all" => Ok(Self::All),
            "auth" => Ok(Self::Auth),
            "auth-live" | "auth_live" | "live-auth" => Ok(Self::AuthLive),
            other => anyhow::bail!(
                "Unknown test target '{}'. Supported targets: auth, auth-live, all",
                other
            ),
        }
    }
}

pub fn run(target: TestTarget) -> Result<()> {
    match target {
        TestTarget::Auth => auth(),
        TestTarget::AuthLive => auth_live(),
        TestTarget::All => {
            auth()?;
            Ok(())
        }
    }
}

fn auth() -> Result<()> {
    println!("\nRunning auth test suite...");
    run_cmd(
        "cargo",
        &["test", "-p", "ops-profile-store", "--test", "auth_and_security"],
        &project_root(),
    )
}

fn auth_live() -> Result<()> {
    println!("\nRunning auth live-store CLI tests...");
    run_cmd(
        "npm",
        &["--prefix", "apps/cli", "run", "test:live"],
        &project_root(),
    )
}
