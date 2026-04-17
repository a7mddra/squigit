// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub struct TestCommandOptions {
    pub list: bool,
    pub all: bool,
    pub path: Vec<String>,
}

pub fn run(options: TestCommandOptions) -> Result<()> {
    crate::tests::runner::run(options)
}
