// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Processor module - Builds API payloads for initial and subsequent turns.

use crate::brain::loader::{interpolate, load_frame, load_scenes, load_soul, load_title_prompt};
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
) -> String {
    let frame_template = load_frame();

    let mut vars = HashMap::new();
    vars.insert(
        "IMAGE_DESCRIPTION".to_string(),
        image_description.to_string(),
    );
    vars.insert("USER_FIRST_MSG".to_string(), user_first_msg.to_string());
    vars.insert("HISTORY_LOG".to_string(), history_log.to_string());

    interpolate(&frame_template, &vars)
}

/// Get the title generation prompt.
pub fn get_title_prompt() -> Result<String, String> {
    load_title_prompt()
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
        );
        assert!(context.contains("VS Code"));
        assert!(context.contains("Fix this bug"));
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
