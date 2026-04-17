// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};
use xtask::{project_root, run_cmd, run_cmd_with_display};

use super::runner::print_group;

pub fn run(list: bool, path: &[String]) -> Result<()> {
    if path.is_empty() {
        if list {
            print_group("apps", &["auth", "apis", "brain"]);
            return Ok(());
        }

        bail!("Missing apps suite. Run `cargo xtask test apps --list`.");
    }

    match path[0].as_str() {
        "auth" => run_auth(list, &path[1..]),
        "apis" => run_apis(list, &path[1..]),
        "brain" => run_brain(list, &path[1..]),
        other => bail!(
            "Unknown apps suite '{}'. Run `cargo xtask test apps --list`.",
            other
        ),
    }
}

fn run_brain(list: bool, path: &[String]) -> Result<()> {
    if list {
        if path.is_empty() {
            print_group(
                "apps/brain",
                &[
                    "analyze <image_path> [user_message...]",
                    "prompt <chat_id> <message...>",
                    "chats",
                ],
            );
            return Ok(());
        }

        bail!("Unexpected path for `cargo xtask test apps brain --list`.");
    }

    if path.is_empty() {
        bail!("Missing brain action. Run `cargo xtask test apps brain --list`.");
    }

    let action = path[0].as_str();
    let mut action_args: Vec<&str> = Vec::new();

    match action {
        "analyze" => {
            if path.len() < 2 {
                bail!("Action 'analyze' requires `<image_path>`.");
            }

            if path[1].trim().is_empty() {
                bail!("Action 'analyze' requires a non-empty `<image_path>`.");
            }

            action_args.push("analyze");
            action_args.push(path[1].as_str());
            for arg in &path[2..] {
                action_args.push(arg.as_str());
            }
        }
        "prompt" => {
            if path.len() < 3 {
                bail!("Action 'prompt' requires `<chat_id> <message...>`.");
            }

            if path[1].trim().is_empty() {
                bail!("Action 'prompt' requires a non-empty `<chat_id>`.");
            }

            let message = path[2..].join(" ").trim().to_string();
            if message.is_empty() {
                bail!("Action 'prompt' requires a non-empty message.");
            }

            action_args.push("prompt");
            action_args.push(path[1].as_str());
            for arg in &path[2..] {
                action_args.push(arg.as_str());
            }
        }
        "chats" => {
            if path.len() != 1 {
                bail!("Action 'chats' does not accept arguments.");
            }

            action_args.push("chats");
        }
        other => {
            bail!(
                "Unknown brain action '{}'. Run `cargo xtask test apps brain --list`.",
                other
            )
        }
    }

    run_cli_command(
        "apps/brain",
        action,
        "starting brain flow",
        "brain",
        &action_args,
    )
}

fn run_apis(list: bool, path: &[String]) -> Result<()> {
    if list {
        if path.is_empty() {
            print_group(
                "apps/apis",
                &[
                    "add <provider> <key>",
                    "remove <provider>",
                    "show <provider>",
                ],
            );
            return Ok(());
        }

        bail!("Unexpected path for `cargo xtask test apps apis --list`.");
    }

    if path.is_empty() {
        bail!("Missing apis action. Run `cargo xtask test apps apis --list`.");
    }

    let action = path[0].as_str();
    let mut action_args: Vec<&str> = Vec::new();

    match action {
        "add" => {
            match path.len() {
                1 => bail!("{}", add_key_hint(None)?),
                2 => bail!("{}", add_key_hint(path.get(1).map(String::as_str))?),
                3 => {}
                _ => bail!("Action 'add' requires `<provider> <key>`."),
            }
            assert_supported_provider(path[1].as_str())?;
            if path[2].trim().is_empty() {
                bail!("Action 'add' requires a non-empty key.");
            }
            action_args.extend([action, path[1].as_str(), path[2].as_str()]);
        }
        "remove" | "show" => {
            if path.len() != 2 {
                bail!("Action '{}' requires `<provider>`.", action);
            }
            assert_supported_provider(path[1].as_str())?;
            action_args.extend([action, path[1].as_str()]);
        }
        other => {
            bail!(
                "Unknown apis action '{}'. Run `cargo xtask test apps apis --list`.",
                other
            )
        }
    }

    run_cli_command(
        "apps/apis",
        action,
        "starting API flow",
        "api",
        &action_args,
    )
}

fn run_auth(list: bool, path: &[String]) -> Result<()> {
    if list {
        if path.is_empty() {
            print_group(
                "apps/auth",
                &[
                    "login",
                    "signup",
                    "logout",
                    "profiles",
                    "remove <id_or_email>",
                ],
            );
            return Ok(());
        }

        bail!("Unexpected path for `cargo xtask test apps auth --list`.");
    }

    if path.is_empty() {
        bail!("Missing auth action. Run `cargo xtask test apps auth --list`.");
    }

    let action = path[0].as_str();
    let mut action_args: Vec<&str> = Vec::new();

    match action {
        "login" | "signup" | "logout" | "profiles" => {
            if path.len() != 1 {
                bail!("Action '{}' does not accept arguments.", action);
            }
            action_args.push(action);
        }
        "remove" => {
            if path.len() != 2 || path[1].trim().is_empty() {
                bail!("Action 'remove' requires `<id_or_email>`.");
            }
            action_args.push("remove");
            action_args.push(path[1].as_str());
        }
        other => {
            bail!(
                "Unknown auth action '{}'. Run `cargo xtask test apps auth --list`.",
                other
            )
        }
    }

    run_cli_command(
        "apps/auth",
        action,
        "starting auth flow",
        "auth",
        &action_args,
    )
}

fn assert_supported_provider(provider: &str) -> Result<()> {
    match provider {
        "google-ai-studio" | "imgbb" => Ok(()),
        other => bail!(
            "Unsupported provider '{}'. Use 'google-ai-studio' or 'imgbb'.",
            other
        ),
    }
}

fn add_key_hint(provider: Option<&str>) -> Result<String> {
    let google_label = terminal_link("Google AI Studio", "https://aistudio.google.com/app/apikey");
    let imgbb_label = terminal_link("ImgBB API", "https://api.imgbb.com/");

    match provider {
        Some("google-ai-studio") => Ok(format!(
            "you can add in your {} <provider> <key>",
            google_label
        )),
        Some("imgbb") => Ok(format!(
            "you can add in your {} <provider> <key>",
            imgbb_label
        )),
        Some(other) => {
            assert_supported_provider(other)?;
            Ok("Action 'add' requires `<provider> <key>`.".to_string())
        }
        None => Ok(format!(
            "Action 'add' requires `<provider> <key>`.\n\
you can add in your {} <provider> <key>\n\
you can add in your {} <provider> <key>",
            google_label, imgbb_label
        )),
    }
}

fn terminal_link(label: &str, url: &str) -> String {
    if should_emit_osc8_links() {
        // OSC 8 hyperlink format:
        // ESC ] 8 ;; <url> ST <label> ESC ] 8 ;; ST
        format!("\x1b]8;;{url}\x1b\\{label}\x1b]8;;\x1b\\")
    } else {
        format!("{label}: {url}")
    }
}

fn should_emit_osc8_links() -> bool {
    if let Some(value) = std::env::var("CARGO_TERM_HYPERLINKS").ok().as_deref() {
        let normalized = value.trim().to_ascii_lowercase();
        if matches!(normalized.as_str(), "1" | "true" | "yes" | "on") {
            return true;
        }
        if matches!(normalized.as_str(), "0" | "false" | "no" | "off") {
            return false;
        }
    }

    io::stderr().is_terminal()
}

fn run_cli_command(
    scope: &str,
    action: &str,
    step_two_label: &str,
    category: &str,
    action_args: &[&str],
) -> Result<()> {
    let root = project_root();
    println!("\n[{}:{}]", scope, action);

    let build_elapsed = if should_use_cached_cli_build(&root)? {
        println!(
            "[{}] Step 1/2: building apps/cli... (cached, {})",
            scope,
            format_elapsed(Duration::ZERO)
        );
        Duration::ZERO
    } else {
        println!("[{}] Step 1/2: building apps/cli...", scope);
        let started = Instant::now();
        run_cmd("npm", &["--prefix", "apps/cli", "run", "build"], &root)?;
        let elapsed = started.elapsed();
        println!("[{}] Step 1/2: done ({})", scope, format_elapsed(elapsed));
        elapsed
    };

    println!("[{}] Step 2/2: {}...", scope, step_two_label);
    let mut node_args = vec!["apps/cli/dist/src/index.js", category];
    node_args.extend_from_slice(action_args);

    let mut node_display_args = node_args.clone();
    if category == "api"
        && action_args.first() == Some(&"add")
        && action_args.len() == 3
        && !action_args[2].trim().is_empty()
    {
        if let Some(last) = node_display_args.last_mut() {
            *last = "<redacted>";
        }
    }

    let flow_started = Instant::now();
    let result = run_cmd_with_display("node", &node_args, &node_display_args, &root);
    let flow_elapsed = flow_started.elapsed();
    println!(
        "[{}] Step 2/2: done ({})",
        scope,
        format_elapsed(flow_elapsed)
    );
    println!(
        "[{}:{}] done in {}",
        scope,
        action,
        format_elapsed(build_elapsed + flow_elapsed)
    );

    result
}

fn should_use_cached_cli_build(root: &Path) -> Result<bool> {
    let cli_dir = root.join("apps").join("cli");
    let marker = cli_dir.join("dist").join("src").join("index.js");
    if !marker.is_file() {
        return Ok(false);
    }

    let marker_mtime = fs::metadata(&marker)?.modified()?;
    for rel in ["src", "test", "package.json", "tsconfig.json"] {
        let input = cli_dir.join(rel);
        let newest = newest_modified_recursive(&input)?;
        let Some(newest) = newest else {
            return Ok(false);
        };

        if newest > marker_mtime {
            return Ok(false);
        }
    }

    Ok(true)
}

fn newest_modified_recursive(path: &Path) -> Result<Option<SystemTime>> {
    if !path.exists() {
        return Ok(None);
    }

    let metadata = fs::metadata(path)?;
    let mut newest = Some(metadata.modified()?);

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_newest = newest_modified_recursive(&entry.path())?;
            if let Some(child_time) = child_newest {
                newest = Some(match newest {
                    Some(current) if current >= child_time => current,
                    _ => child_time,
                });
            }
        }
    }

    Ok(newest)
}

fn format_elapsed(elapsed: Duration) -> String {
    format!("{:.1}s", elapsed.as_secs_f64())
}

#[cfg(test)]
mod tests {
    use super::run;

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn auth_rejects_missing_remove_argument() {
        let err = run(false, &path(&["auth", "remove"]))
            .expect_err("expected remove argument validation error");
        assert!(err.to_string().contains("requires `<id_or_email>`"));
    }

    #[test]
    fn auth_rejects_unknown_action() {
        let err =
            run(false, &path(&["auth", "signin"])).expect_err("expected unknown auth action error");
        assert!(err.to_string().contains("Unknown auth action"));
    }

    #[test]
    fn apis_rejects_missing_add_key() {
        let err = run(false, &path(&["apis", "add", "google-ai-studio"]))
            .expect_err("expected add argument validation error");
        assert!(err.to_string().contains("Google AI Studio"));
        assert!(err
            .to_string()
            .contains("https://aistudio.google.com/app/apikey"));
    }

    #[test]
    fn apis_rejects_missing_add_key_imgbb_hint() {
        let err = run(false, &path(&["apis", "add", "imgbb"]))
            .expect_err("expected add argument validation error");
        assert!(err.to_string().contains("ImgBB API"));
        assert!(err.to_string().contains("https://api.imgbb.com/"));
    }

    #[test]
    fn apis_rejects_unknown_provider() {
        let err = run(false, &path(&["apis", "show", "gemini"]))
            .expect_err("expected unknown provider validation error");
        assert!(err.to_string().contains("Unsupported provider"));
    }

    #[test]
    fn apis_rejects_extra_add_arguments_without_hint_links() {
        let err = run(
            false,
            &path(&["apis", "add", "google-ai-studio", "key", "unexpected"]),
        )
        .expect_err("expected strict add arity validation error");
        assert!(err
            .to_string()
            .contains("Action 'add' requires `<provider> <key>`."));
        assert!(!err.to_string().contains("aistudio.google.com"));
    }

    #[test]
    fn brain_rejects_unknown_action() {
        let err =
            run(false, &path(&["brain", "chat"])).expect_err("expected unknown brain action error");
        assert!(err.to_string().contains("Unknown brain action"));
    }

    #[test]
    fn brain_rejects_missing_analyze_image() {
        let err = run(false, &path(&["brain", "analyze"]))
            .expect_err("expected analyze image path validation error");
        assert!(err.to_string().contains("requires `<image_path>`"));
    }

    #[test]
    fn brain_rejects_missing_prompt_arguments() {
        let err = run(false, &path(&["brain", "prompt"]))
            .expect_err("expected prompt argument validation error");
        assert!(err
            .to_string()
            .contains("requires `<chat_id> <message...>`"));
    }

    #[test]
    fn brain_rejects_chats_extra_arguments() {
        let err = run(false, &path(&["brain", "chats", "unexpected"]))
            .expect_err("expected chats no-args validation error");
        assert!(err
            .to_string()
            .contains("Action 'chats' does not accept arguments."));
    }

    #[test]
    fn auth_rejects_profiles_extra_arguments() {
        let err = run(false, &path(&["auth", "profiles", "unexpected"]))
            .expect_err("expected profiles no-args validation error");
        assert!(err
            .to_string()
            .contains("Action 'profiles' does not accept arguments."));
    }
}
