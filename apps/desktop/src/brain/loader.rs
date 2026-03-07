// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

//! Loader module - Parses YAML, JSON, and MD prompt files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Soul identity configuration from soul.yml
#[derive(Debug, Deserialize)]
pub struct SoulIdentity {
    pub name: String,
    pub role: String,
    pub description: String,
}

/// Core instructions from soul.yml
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
    let yaml_content = include_str!("prompts/core/soul.yml");
    serde_yaml::from_str(yaml_content).map_err(|e| format!("Failed to parse soul.yml: {}", e))
}

/// Load the title prompt from embedded YAML
pub fn load_title_prompt() -> Result<String, String> {
    let yaml_content = include_str!("prompts/core/title.yml");
    let config: TitleConfig = serde_yaml::from_str(yaml_content)
        .map_err(|e| format!("Failed to parse title.yml: {}", e))?;
    Ok(config.generate_title)
}

/// Load scenes from embedded JSON
pub fn load_scenes() -> Result<Vec<Scene>, String> {
    let json_content = include_str!("knowledge/scenes.json");
    serde_json::from_str(json_content).map_err(|e| format!("Failed to parse scenes.json: {}", e))
}

/// Load the context frame template from embedded MD
pub fn load_frame() -> String {
    include_str!("prompts/frame.md").to_string()
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
}
