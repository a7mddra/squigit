// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Loader module - Parses YAML, JSON, and MD prompt files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Soul identity configuration from squigit_soul.yml
#[derive(Debug, Deserialize)]
pub struct SoulIdentity {
    pub name: String,
    pub role: String,
    pub description: String,
}

/// Core instructions from squigit_soul.yml
#[derive(Debug, Deserialize)]
pub struct CoreInstructions {
    pub be_helpful: String,
    pub no_fluff: String,
    pub balanced_emojis: String,
    pub markdown: String,
    pub conversational: String,
}

/// Skill matrix configuration
#[derive(Debug, Deserialize)]
pub struct SkillMatrix {
    pub description: String,
    pub scenes: String,
}

/// Unknown scenes protocol
#[derive(Debug, Deserialize)]
pub struct UnknownScenes {
    pub protocol: Vec<HashMap<String, String>>,
}

/// Complete soul configuration
#[derive(Debug, Deserialize)]
pub struct SoulConfig {
    pub identity: SoulIdentity,
    pub core_instructions: CoreInstructions,
    pub skill_matrix: SkillMatrix,
    pub unknown_scenes: UnknownScenes,
}

/// Title prompt configuration
#[derive(Debug, Deserialize)]
pub struct TitleConfig {
    pub generate_title: String,
}

/// Scene definition from scenes.json
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Scene {
    pub scene: String,
    pub triggers: Vec<String>,
    pub action: String,
}

/// Load the soul configuration from embedded YAML
pub fn load_soul() -> Result<SoulConfig, String> {
    let yaml_content = include_str!("../assets/core/squigit_soul.yml");
    serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse squigit_soul.yml: {}", e))
}

/// Load the title prompt from embedded YAML
pub fn load_title_prompt() -> Result<String, String> {
    let yaml_content = include_str!("../assets/helpers/thread_title.yml");
    let config: TitleConfig = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse thread_title.yml: {}", e))?;
    Ok(config.generate_title)
}

/// Load scenes from embedded JSON
pub fn load_scenes() -> Result<Vec<Scene>, String> {
    let json_content = include_str!("../assets/knowledge/known_scenes.json");
    serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse known_scenes.json: {}", e))
}

/// Load the context frame template from embedded MD
pub fn load_frame() -> String {
    include_str!("../assets/core/context_window.md").to_string()
}

/// Interpolate variables in a template string
/// Replaces {{VAR_NAME}} with the provided value
pub fn interpolate(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

/// System runtime configuration from system.yml
#[derive(Debug, Deserialize)]
pub struct SystemConfig {
    pub identity_brief: String,
    pub runtime_template: String,
}

/// Image brief configuration from image_brief.yml
#[derive(Debug, Deserialize)]
pub struct ImageBriefConfig {
    pub describe_image: String,
}

/// Attachment preview context template from attachment_preview_context.yml
#[derive(Debug, Deserialize)]
pub struct AttachmentPreviewContextConfig {
    pub header: String,
    pub success_item_template: String,
    pub error_item_template: String,
}

/// Load the system runtime configuration from embedded YAML
pub fn load_system() -> Result<SystemConfig, String> {
    let yaml_content = include_str!("../assets/core/system_prompt.yml");
    serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse system_prompt.yml: {}", e))
}

/// Load the image brief prompt from embedded YAML
pub fn load_image_brief_prompt() -> Result<String, String> {
    let yaml_content = include_str!("../assets/helpers/image_brief.yml");
    let config: ImageBriefConfig = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse image_brief.yml: {}", e))?;
    Ok(config.describe_image)
}

/// Load the attachment preview template from embedded YAML
pub fn load_attachment_preview_context() -> Result<AttachmentPreviewContextConfig, String> {
    let yaml_content = include_str!("../assets/helpers/attachment_preview_context.yml");
    serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse attachment_preview_context.yml: {}", e))
}

/// Load the web search tool declaration from embedded JSON
pub fn load_web_search_tool_declaration() -> Result<serde_json::Value, String> {
    let json_content = include_str!("../assets/helpers/web_search.json");
    serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse web_search.json: {}", e))
}

/// Load the local attachment reader tool declaration from embedded JSON.
pub fn load_read_local_attachment_context_tool_declaration() -> Result<serde_json::Value, String> {
    let json_content = include_str!("../assets/helpers/read_local_attachment_context.json");
    serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse read_local_attachment_context.json: {}", e))
}

/// Load the chat attachment recall tool declaration from embedded JSON.
pub fn load_recall_chat_attachment_tool_declaration() -> Result<serde_json::Value, String> {
    let json_content = include_str!("../assets/helpers/recall_chat_attachment.json");
    serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse recall_chat_attachment.json: {}", e))
}

/// Load all Gemini tool declarations enabled for chat turns with tools.
pub fn load_gemini_tool_declarations() -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![
        load_web_search_tool_declaration()?,
        load_read_local_attachment_context_tool_declaration()?,
        load_recall_chat_attachment_tool_declaration()?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_soul() {
        let soul = load_soul().expect("Failed to load soul");
        assert_eq!(soul.identity.name, crate::constants::APP_NAME);
    }

    #[test]
    fn test_load_scenes() {
        let scenes = load_scenes().expect("Failed to load scenes");
        assert!(!scenes.is_empty());
    }

    #[test]
    fn test_interpolate() {
        let template = "Hello {{NAME}}, welcome to {{PLACE}}!";
        let mut vars = HashMap::new();
        vars.insert("NAME".to_string(), "User".to_string());
        vars.insert("PLACE".to_string(), crate::constants::APP_NAME.to_string());

        let result = interpolate(template, &vars);
        assert_eq!(
            result,
            format!("Hello User, welcome to {}!", crate::constants::APP_NAME)
        );
    }

    #[test]
    fn test_load_web_search_tool_declaration() {
        let declaration =
            load_web_search_tool_declaration().expect("Failed to load web search tool declaration");
        let name = declaration
            .get("functionDeclarations")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str());
        assert_eq!(name, Some("web_search"));
    }

    #[test]
    fn test_load_read_local_attachment_context_tool_declaration() {
        let declaration = load_read_local_attachment_context_tool_declaration()
            .expect("Failed to load local attachment tool declaration");
        let name = declaration
            .get("functionDeclarations")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str());
        assert_eq!(name, Some("read_local_attachment_context"));
    }

    #[test]
    fn test_load_recall_chat_attachment_tool_declaration() {
        let declaration = load_recall_chat_attachment_tool_declaration()
            .expect("Failed to load recall attachment tool declaration");
        let name = declaration
            .get("functionDeclarations")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("name"))
            .and_then(|v| v.as_str());
        assert_eq!(name, Some("recall_chat_attachment"));
    }

    #[test]
    fn test_load_gemini_tool_declarations() {
        let declarations = load_gemini_tool_declarations().expect("Failed to load declarations");
        assert_eq!(declarations.len(), 3);

        let names = declarations
            .iter()
            .filter_map(|decl| {
                decl.get("functionDeclarations")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.get("name"))
                    .and_then(|v| v.as_str())
            })
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "web_search",
                "read_local_attachment_context",
                "recall_chat_attachment"
            ]
        );
    }

    #[test]
    fn test_load_attachment_preview_context() {
        let cfg = load_attachment_preview_context().expect("Failed to load preview context");
        assert!(cfg.header.contains("local files"));
        assert!(cfg.success_item_template.contains("{{PATH}}"));
        assert!(cfg.error_item_template.contains("{{ERROR_CODE}}"));
    }
}
