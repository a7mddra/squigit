// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Usage examples:
//!   cargo xtask build
//!   cargo xtask build all -ocr
//!   cargo xtask build --all --measure-ocr-size
//!   cargo xtask build ocr stt
//!   cargo xtask test auth
//!   cargo xtask report --strict
//!   cargo xtask version 0.2.0
//!   cargo xtask version --bump patch
//!   cargo xtask setup --all

pub mod commands;
pub mod compile;
pub mod console;
pub mod packaging;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use commands::{build as cmd_build, clean, dev, report, setup, test, version};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Build automation and contributor control panel")]
struct Cli {
    /// Run a focused test suite (shortcut for `cargo xtask test <target>`)
    #[arg(long, value_name = "TARGET")]
    test: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Build all targets or selected targets (supports: all, --all, -ocr style exclusions)
    Build {
        /// Include all buildable targets before applying exclusions.
        #[arg(long)]
        all: bool,

        /// Measure OCR payload size report (disabled by default).
        #[arg(long)]
        measure_ocr_size: bool,

        /// Build selectors and exclusions (e.g. ocr stt, all -ocr, capture-qt).
        #[arg(value_name = "TARGET", allow_hyphen_values = true)]
        selectors: Vec<String>,
    },

    /// Clean all build artifacts
    Clean,

    /// Tauri commands (dev, build, etc.) + extra arguments
    Run {
        #[arg(default_value = "dev")]
        cmd: String,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run the app in dev mode (optionally tray/background)
    Dev {
        /// Launch mode: "tray" for background/tray-only
        #[arg(default_value = None)]
        mode: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Run project health report (informational by default)
    Report {
        /// Fail process if any check fails.
        #[arg(long)]
        strict: bool,
    },

    /// Run focused test suites
    Test {
        /// Test target (auth, auth-live, all)
        #[arg(default_value = "all")]
        target: String,
    },

    /// Sync project version across Cargo/JSON/CMake/changelog
    Version {
        /// Explicit target semver (x.y.z).
        version: Option<String>,

        /// Semantic bump mode.
        #[arg(long, value_enum)]
        bump: Option<version::BumpPart>,
    },

    /// Contributor environment setup (safe checks by default)
    Setup {
        /// Attempt admin-level installation steps where possible.
        #[arg(long)]
        all: bool,

        /// Focus Qt/CMake setup checks.
        #[arg(long)]
        qt: bool,

        /// Focus Python setup checks.
        #[arg(long)]
        py: bool,

        /// Focus Cargo/Rust setup checks.
        #[arg(long)]
        cargo: bool,

        /// Focus Node/NPM setup checks.
        #[arg(long)]
        npm: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.test.is_some() && cli.command.is_some() {
        bail!("Use either `cargo xtask test <target>` or `cargo xtask --test <target>`, not both.");
    }

    if let Some(target) = cli.test {
        test::run(test::TestTarget::parse(&target)?)?;
        return Ok(());
    }

    let Some(command) = cli.command else {
        bail!("No command provided. Try `cargo xtask test auth` or `cargo xtask --test auth`.");
    };

    match command {
        Commands::Build {
            all,
            measure_ocr_size,
            selectors,
        } => cmd_build::run(cmd_build::BuildCommandOptions {
            selectors,
            include_all: all,
            measure_ocr_size,
        })?,
        Commands::Clean => clean::all()?,
        Commands::Run { cmd, args } => dev::run(&cmd, false, &args)?,
        Commands::Dev { mode, args } => dev::run("dev", mode.as_deref() == Some("tray"), &args)?,
        Commands::Report { strict } => report::run(report::ReportOptions { strict })?,
        Commands::Test { target } => test::run(test::TestTarget::parse(&target)?)?,
        Commands::Version {
            version: explicit,
            bump,
        } => version::run(version::VersionOptions { explicit, bump })?,
        Commands::Setup {
            all,
            qt,
            py,
            cargo,
            npm,
        } => setup::run(setup::SetupOptions {
            all,
            qt,
            py,
            cargo,
            npm,
        })?,
    }

    Ok(())
}
