mod instance;
mod paths;
mod sandbox;

use anyhow::{Context, Result};
pub use instance::cancel_active;
use instance::Instance;
use paths::RuntimePaths;
#[cfg(target_os = "linux")]
use sandbox::audio_guard::AudioGuard;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

pub struct CaptureOutcome {
    pub exit_code: ExitCode,
    pub saved_path: Option<PathBuf>,
}

impl CaptureOutcome {
    fn exit(exit_code: ExitCode) -> Self {
        Self {
            exit_code,
            saved_path: None,
        }
    }
}

const HELP: &str = "Squigit

Usage:
  squigit [MODE] [--auto-ocr] [--output-dir <DIR>]
  squigit --authorize
  squigit --preflight
  squigit --help
  squigit --version

Capture modes:
  --squiggle     Capture a freeform squiggle selection (default)
  --traditional  Capture a rectangular selection

Options:
  --auto-ocr          Recognize the frozen display after selecting an area
  --output-dir <DIR>  Save the captured image into DIR (default: Pictures/squigit)
  --authorize         Request screenshot access without opening the selection overlay
  --preflight         Check screenshot access without requesting permission
  -h, --help          Print this help
  -V, --version       Print the squigit version";

enum CaptureCommand {
    Preflight,
    Authorize,
    Capture {
        mode: String,
        auto_ocr: bool,
        output_dir: Option<String>,
    },
}

pub fn run(arguments: Vec<String>) -> CaptureOutcome {
    match dispatch(arguments) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("[squigit] Error: {:#}", e);
            #[cfg(target_os = "linux")]
            AudioGuard::unmute();
            CaptureOutcome::exit(ExitCode::FAILURE)
        }
    }
}

fn dispatch(arguments: Vec<String>) -> Result<CaptureOutcome> {
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("{HELP}");
        return Ok(CaptureOutcome::exit(ExitCode::SUCCESS));
    }
    if arguments
        .iter()
        .any(|argument| argument == "--version" || argument == "-V")
    {
        verify_runtime()?;
        println!("squigit {}", env!("CARGO_PKG_VERSION"));
        return Ok(CaptureOutcome::exit(ExitCode::SUCCESS));
    }

    let command = parse_arguments(arguments)?;
    let mut instance = match command {
        CaptureCommand::Preflight => Instance::preflight(),
        CaptureCommand::Authorize => Instance::authorize(),
        CaptureCommand::Capture {
            mode,
            auto_ocr,
            output_dir,
        } => Instance::capture(mode, auto_ocr).with_output_dir(output_dir),
    };
    instance.run()
}

fn parse_arguments(arguments: Vec<String>) -> Result<CaptureCommand> {
    let mut mode = None;
    let mut output_dir = None;
    let mut auto_ocr = false;
    let mut authorize = false;
    let mut preflight = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--auto-ocr" => {
                if auto_ocr {
                    anyhow::bail!("--auto-ocr may only be provided once");
                }
                auto_ocr = true;
            }
            "--squiggle" | "--traditional" => {
                if mode.is_some() {
                    anyhow::bail!("Capture modes are mutually exclusive");
                }
                mode = Some(arguments[index].clone());
            }
            "--output-dir" => {
                if output_dir.is_some() {
                    anyhow::bail!("--output-dir may only be provided once");
                }
                let Some(value) = arguments.get(index + 1) else {
                    anyhow::bail!("--output-dir requires a directory");
                };
                output_dir = Some(resolve_output_dir(value)?);
                index += 1;
            }
            "--authorize" => {
                if authorize {
                    anyhow::bail!("--authorize may only be provided once");
                }
                authorize = true;
            }
            "--preflight" => {
                if preflight {
                    anyhow::bail!("--preflight may only be provided once");
                }
                preflight = true;
            }
            argument => anyhow::bail!("Unknown capture argument: {argument}"),
        }
        index += 1;
    }

    if authorize || preflight {
        if mode.is_some() || output_dir.is_some() || auto_ocr {
            anyhow::bail!("Permission commands cannot be combined with capture options");
        }
        if authorize && preflight {
            anyhow::bail!("--authorize and --preflight are mutually exclusive");
        }
        return Ok(if authorize {
            CaptureCommand::Authorize
        } else {
            CaptureCommand::Preflight
        });
    }

    Ok(CaptureCommand::Capture {
        mode: mode.unwrap_or_else(|| "--squiggle".to_string()),
        auto_ocr,
        output_dir,
    })
}

fn resolve_output_dir(value: &str) -> Result<String> {
    let candidate = if value == "~" {
        home()?
    } else if let Some(stripped) = value.strip_prefix("~/") {
        home()?.join(stripped)
    } else if Path::new(value).is_absolute() {
        PathBuf::from(value)
    } else {
        std::env::current_dir()
            .context("Could not resolve the current directory")?
            .join(value)
    };
    Ok(candidate.to_string_lossy().into_owned())
}

fn home() -> Result<PathBuf> {
    dirs::home_dir().context("Could not resolve the home directory")
}

fn verify_runtime() -> Result<()> {
    let RuntimePaths { bin, env_vars } = RuntimePaths::resolve()?;
    let mut command = Command::new(&bin);
    command
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    for (key, value) in env_vars {
        command.env(key, value);
    }
    let status = command.status().with_context(|| {
        format!(
            "Could not start the packaged Qt runtime at {}",
            bin.display()
        )
    })?;
    if !status.success() {
        anyhow::bail!(
            "The packaged Qt runtime failed its version check with exit code {:?}",
            status.code()
        );
    }
    Ok(())
}
