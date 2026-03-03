// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Spawning & killing the sidecar process.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, ChildStdin, Command};

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Failed to spawn sidecar: {0}")]
    SpawnError(std::io::Error),
    #[error("Failed to capture stdin")]
    StdinError,
    #[error("Failed to capture stdout")]
    StdoutError,
}

pub type Result<T> = std::result::Result<T, ProcessError>;

pub struct SidecarProcess {
    pub child: Child,
    pub stdin: ChildStdin,
}

impl SidecarProcess {
    pub fn spawn(binary_path: &PathBuf) -> Result<(Self, tokio::process::ChildStdout)> {
        let mut cmd = Command::new(binary_path);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit()); // We want to see stderr logs in our console

        // Windows no-window flag
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(ProcessError::SpawnError)?;

        let stdin = child.stdin.take().ok_or(ProcessError::StdinError)?;
        let stdout = child.stdout.take().ok_or(ProcessError::StdoutError)?;

        Ok((Self { child, stdin }, stdout))
    }

    pub async fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill().await
    }
}

impl Drop for SidecarProcess {
    fn drop(&mut self) {
        // Ensure the sidecar doesn't outlive the Rust orchestrator
        let _ = self.child.start_kill();
    }
}
