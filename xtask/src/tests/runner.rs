// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};

use crate::commands::test::TestCommandOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedInvocation {
    ListRoot,
    Category {
        category: String,
        tail: Vec<String>,
        list: bool,
        all: bool,
    },
}

pub fn run(options: TestCommandOptions) -> Result<()> {
    match parse_invocation(options)? {
        ParsedInvocation::ListRoot => {
            print_group("tests", &["apps", "crates", "sidecars"]);
            Ok(())
        }
        ParsedInvocation::Category {
            category,
            tail,
            list,
            all,
        } => match category.as_str() {
            "apps" => super::apps::run(list, &tail),
            "crates" => super::crates::run(list, all, &tail),
            "sidecars" => super::sidecars::run(list, &tail),
            other => bail!(
                "Unknown test category '{}'. Run `cargo xtask test --list`.",
                other
            ),
        },
    }
}

pub fn print_group(title: &str, entries: &[&str]) {
    println!("[{}]", title);
    for entry in entries {
        println!("- {}", entry);
    }
}

fn parse_invocation(options: TestCommandOptions) -> Result<ParsedInvocation> {
    if options.path.is_empty() {
        if options.list {
            return Ok(ParsedInvocation::ListRoot);
        }

        bail!(
            "Missing test path. Run `cargo xtask test --list` or `cargo xtask test apps auth --list`."
        );
    }

    Ok(ParsedInvocation::Category {
        category: options.path[0].clone(),
        tail: options.path[1..].to_vec(),
        list: options.list,
        all: options.all,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_invocation, run, ParsedInvocation};
    use crate::commands::test::TestCommandOptions;

    fn options(list: bool, path: &[&str]) -> TestCommandOptions {
        TestCommandOptions {
            list,
            all: false,
            path: path.iter().map(|token| token.to_string()).collect(),
        }
    }

    #[test]
    fn parses_root_list() {
        let parsed = parse_invocation(options(true, &[])).expect("parse root list");
        assert_eq!(parsed, ParsedInvocation::ListRoot);
    }

    #[test]
    fn rejects_empty_non_list_invocation() {
        let err = parse_invocation(options(false, &[])).expect_err("expected parse failure");
        assert!(err.to_string().contains("Missing test path"));
    }

    #[test]
    fn parses_apps_auth_login_route() {
        let parsed = parse_invocation(options(false, &["apps", "auth", "login"]))
            .expect("parse apps auth login");
        assert_eq!(
            parsed,
            ParsedInvocation::Category {
                category: "apps".to_string(),
                tail: vec!["auth".to_string(), "login".to_string()],
                list: false,
                all: false,
            }
        );
    }

    #[test]
    fn rejects_legacy_flag_style_category_tokens() {
        let err = run(options(false, &["--apps", "--auth", "login"]))
            .expect_err("expected unknown category");
        assert!(err.to_string().contains("Unknown test category '--apps'"));
    }
}
