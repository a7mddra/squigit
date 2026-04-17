// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use std::fs;
use std::process::Command;
use xtask::{project_root, run_cmd};

use super::runner::print_group;

pub fn run(list: bool, all: bool, path: &[String]) -> Result<()> {
    let crates = discover_workspace_crates()?;

    if list {
        if all || !path.is_empty() {
            bail!("`cargo xtask test crates --list` does not accept extra arguments.");
        }

        let entries: Vec<&str> = crates.iter().map(|target| target.alias.as_str()).collect();
        print_group("crates", &entries);
        return Ok(());
    }

    if all && !path.is_empty() {
        bail!("Use either `cargo xtask test crates --all` or a single crate name.");
    }

    if path.len() > 1 {
        bail!("`cargo xtask test crates` accepts at most one crate name.");
    }

    let selected: Vec<&CrateTarget> = if all {
        crates.iter().collect()
    } else if let Some(token) = path.first() {
        let target = crates
            .iter()
            .find(|candidate| candidate.alias == *token || candidate.package == *token)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unknown crate '{}'. Run `cargo xtask test crates --list`.",
                    token
                )
            })?;
        vec![target]
    } else {
        bail!("Missing crate target. Use `cargo xtask test crates --list` or `cargo xtask test crates --all`.");
    };

    if all {
        return run_all_with_report(&selected);
    }

    let mut args = vec!["test".to_string()];
    for target in &selected {
        args.push("-p".to_string());
        args.push(target.package.clone());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_cmd("cargo", &arg_refs, &project_root())
}

#[derive(Debug, Clone)]
struct CrateRunResult {
    alias: String,
    package: String,
    ok: bool,
}

fn run_all_with_report(selected: &[&CrateTarget]) -> Result<()> {
    let root = project_root();
    let mut results = Vec::with_capacity(selected.len());

    for target in selected {
        println!(
            "\n[crates/{}] Running cargo test -p {}",
            target.alias, target.package
        );

        let status = Command::new("cargo")
            .args(["test", "-p", target.package.as_str()])
            .current_dir(&root)
            .status()?;

        results.push(CrateRunResult {
            alias: target.alias.clone(),
            package: target.package.clone(),
            ok: status.success(),
        });
    }

    print_all_report(&results);

    if results.iter().any(|result| !result.ok) {
        bail!("One or more crate test targets failed.");
    }

    Ok(())
}

fn print_all_report(results: &[CrateRunResult]) {
    let passed = results.iter().filter(|result| result.ok).count();
    let failed = results.len().saturating_sub(passed);

    println!("\n============================================================");
    println!("CRATES TEST REPORT (--all)");
    println!("------------------------------------------------------------");
    for result in results {
        let status = if result.ok { "PASS" } else { "FAIL" };
        println!(
            "[{}] {:<18} ({})",
            status, result.alias, result.package
        );
    }
    println!("------------------------------------------------------------");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total : {}", results.len());
    println!("============================================================");
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CrateTarget {
    alias: String,
    package: String,
}

fn discover_workspace_crates() -> Result<Vec<CrateTarget>> {
    let root = project_root().join("crates");
    let mut crates = Vec::new();

    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let Some(dir_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        let cargo_toml = path.join("Cargo.toml");
        if !cargo_toml.is_file() {
            continue;
        }

        crates.push(CrateTarget {
            alias: alias_from_dir_name(dir_name),
            package: dir_name.to_string(),
        });
    }

    crates.sort_by(|a, b| a.alias.cmp(&b.alias));
    Ok(crates)
}

fn alias_from_dir_name(dir_name: &str) -> String {
    for prefix in ["ops-", "svc-", "sys-"] {
        if let Some(rest) = dir_name.strip_prefix(prefix) {
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }

    dir_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::alias_from_dir_name;

    #[test]
    fn strips_known_prefixes_for_alias() {
        assert_eq!(alias_from_dir_name("ops-profile-store"), "profile-store");
        assert_eq!(alias_from_dir_name("svc-speech-engine"), "speech-engine");
        assert_eq!(alias_from_dir_name("sys-single-instance"), "single-instance");
    }

    #[test]
    fn keeps_unprefixed_dir_name() {
        assert_eq!(alias_from_dir_name("my-crate"), "my-crate");
    }
}
