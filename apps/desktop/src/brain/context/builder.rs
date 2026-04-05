// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Processor module - Builds API payloads for initial and subsequent turns.

use crate::brain::context::loader::{
    interpolate, load_frame, load_image_brief_prompt, load_scenes, load_soul, load_system,
    load_title_prompt,
};
use std::collections::HashMap;

/// Build the system prompt for the initial turn (with image).
/// This includes the full soul identity and scenes knowledge base.
pub fn build_initial_system_prompt() -> Result<String, String> {
    let soul = load_soul()?;
    let scenes = load_scenes()?;

    // Serialize scenes to JSON for embedding
    let scenes_json = serde_json::to_string_pretty(&scenes)
        .map_err(|e| format!("Failed to serialize scenes: {}", e))?;

    // Build the system prompt
    let mut prompt = String::new();

    // Identity section
    prompt.push_str(&format!(
        "# Identity\n\
        You are **{}**, {}.\n\
        {}\n\n",
        soul.identity.name, soul.identity.role, soul.identity.description
    ));

    // Core instructions
    prompt.push_str("# Core Instructions\n");
    prompt.push_str(&format!(
        "- **Be Helpful**: {}\n",
        soul.core_instructions.be_helpful
    ));
    prompt.push_str(&format!(
        "- **No Fluff**: {}\n",
        soul.core_instructions.no_fluff
    ));
    prompt.push_str(&format!(
        "- **Emojis**: {}\n",
        soul.core_instructions.balanced_emojis
    ));
    prompt.push_str(&format!(
        "- **Markdown**: {}\n",
        soul.core_instructions.markdown
    ));
    prompt.push_str(&format!(
        "- **Conversational**: {}\n\n",
        soul.core_instructions.conversational
    ));

    // Skill matrix
    prompt.push_str(&format!(
        "# Skill Matrix\n\
        {}\n\n\
        ## Known Scenes\n\
        ```json\n{}\n```\n\n",
        soul.skill_matrix.description, scenes_json
    ));

    // Unknown scenes protocol
    prompt.push_str("# Unknown Scenes Protocol\n");
    prompt.push_str("If the screenshot doesn't match any known scene:\n");
    for step in &soul.unknown_scenes.protocol {
        for (key, value) in step {
            prompt.push_str(&format!("- **{}**: {}\n", key, value));
        }
    }

    Ok(prompt)
}

/// Build the context frame for subsequent turns (without image).
/// Interpolates the frame.md template with conversation context.
pub fn build_turn_context(
    image_description: &str,
    user_first_msg: &str,
    history_log: &str,
    rolling_summary: &str,
) -> String {
    let frame_template = load_frame();

    let mut vars = HashMap::new();
    vars.insert(
        "IMAGE_DESCRIPTION".to_string(),
        image_description.to_string(),
    );
    vars.insert("USER_FIRST_MSG".to_string(), user_first_msg.to_string());
    vars.insert("HISTORY_LOG".to_string(), history_log.to_string());
    vars.insert(
        "ROLLING_SUMMARY".to_string(),
        if rolling_summary.is_empty() {
            "(No prior summary — this is early in the conversation)".to_string()
        } else {
            rolling_summary.to_string()
        },
    );

    interpolate(&frame_template, &vars)
}

/// Get the title generation prompt.
pub fn get_title_prompt() -> Result<String, String> {
    load_title_prompt()
}

/// Get the image brief prompt (for lightweight description via Lite model).
pub fn get_image_brief_prompt() -> Result<String, String> {
    load_image_brief_prompt()
}

/// Build the system instruction for the native Gemini `system_instruction` field.
/// Called on EVERY turn. Interpolates system.yml with runtime data.
pub fn build_system_instruction(
    user_name: &str,
    user_email: &str,
    image_brief: &str,
) -> Result<String, String> {
    let system_config = load_system()?;

    // Collect runtime data
    let now = chrono::Local::now();
    let datetime = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let timezone = iana_time_zone::get_timezone().unwrap_or_else(|_| "Unknown".to_string());

    // OS info via std (tauri_plugin_os requires AppHandle, so we use std::env::consts + sysinfo)
    let os_type = std::env::consts::OS; // "linux", "macos", "windows"
    let os_arch = std::env::consts::ARCH; // "x86_64", "aarch64"
    let platform = if cfg!(target_os = "linux") {
        "desktop/linux"
    } else if cfg!(target_os = "macos") {
        "desktop/macos"
    } else if cfg!(target_os = "windows") {
        "desktop/windows"
    } else {
        "desktop/unknown"
    };

    // OS version: try /etc/os-release on Linux, fallback to consts
    let os_version = get_os_version();

    // Hostname via system command (no extra crate needed)
    let hostname = std::process::Command::new("hostname")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Locale from environment
    let locale = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .unwrap_or_else(|_| "en_US.UTF-8".to_string());

    // Interpolate runtime template
    let mut vars = HashMap::new();
    vars.insert("DATETIME".to_string(), datetime);
    vars.insert("TIMEZONE".to_string(), timezone);
    vars.insert("OS_TYPE".to_string(), os_type.to_string());
    vars.insert("OS_VERSION".to_string(), os_version);
    vars.insert("OS_ARCH".to_string(), os_arch.to_string());
    vars.insert("PLATFORM".to_string(), platform.to_string());
    vars.insert("HOSTNAME".to_string(), hostname);
    vars.insert("LOCALE".to_string(), locale);
    vars.insert("USER_NAME".to_string(), user_name.to_string());
    vars.insert("USER_EMAIL".to_string(), user_email.to_string());
    vars.insert("IMAGE_BRIEF".to_string(), if image_brief.is_empty() {
        "(Image file is attached directly to this request)".to_string()
    } else {
        image_brief.to_string()
    });

    let runtime_section = interpolate(&system_config.runtime_template, &vars);
    let full_instruction = format!("{}\n{}", system_config.identity_brief.trim(), runtime_section);

    Ok(full_instruction)
}

/// Try to get a descriptive OS version string.
fn get_os_version() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if let Some(pretty) = line.strip_prefix("PRETTY_NAME=") {
                    return pretty.trim_matches('"').to_string();
                }
            }
        }
    }
    // Fallback for all platforms
    std::env::consts::OS.to_string()
}

/// Format conversation history for the frame template.
/// Takes the last N message pairs and formats them as markdown.
pub fn format_history_log(messages: &[(String, String)], max_turns: usize) -> String {
    let start = messages.len().saturating_sub(max_turns);
    let recent = &messages[start..];

    let mut log = String::new();
    for (i, (role, content)) in recent.iter().enumerate() {
        if i > 0 {
            log.push_str("\n\n");
        }
        log.push_str(&format!("**{}**: {}", role, content));
    }

    if log.is_empty() {
        log.push_str("(No previous messages)");
    }

    log
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_initial_prompt() {
        let prompt = build_initial_system_prompt().expect("Failed to build prompt");
        assert!(prompt.contains(crate::constants::APP_NAME));
        assert!(prompt.contains("Identity"));
        assert!(prompt.contains("Core Instructions"));
    }

    #[test]
    fn test_build_turn_context() {
        let context = build_turn_context(
            "A VS Code window with Rust code",
            "Fix this bug",
            "**User**: Fix this bug\n**Assistant**: I can see the issue...",
            "",
        );
        assert!(context.contains("VS Code"));
        assert!(context.contains("Fix this bug"));
        assert!(context.contains("No prior summary"));
    }

    #[test]
    fn test_build_turn_context_with_summary() {
        let context = build_turn_context(
            "A VS Code window with Rust code",
            "Fix this bug",
            "**User**: What next?\n**Assistant**: Let's continue...",
            "- User found a lifetime error in main.rs\n- Fixed by adding 'static bound",
        );
        assert!(context.contains("lifetime error"));
        assert!(context.contains("What next?"));
    }

    #[test]
    fn test_format_history() {
        let messages = vec![
            ("User".to_string(), "Hello".to_string()),
            ("Assistant".to_string(), "Hi there!".to_string()),
            ("User".to_string(), "Help me".to_string()),
        ];
        let log = format_history_log(&messages, 2);
        assert!(log.contains("Hi there!"));
        assert!(log.contains("Help me"));
    }
}
