use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub appearance: Appearance,
    pub editor: Editor,
    pub advanced: Advanced,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            general: General::default(),
            appearance: Appearance::default(),
            editor: Editor::default(),
            advanced: Advanced::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct General {
    pub nickname: String,
    pub launch_at_login: bool,
    pub check_updates: bool,
    pub default_page: String,
}

impl Default for General {
    fn default() -> Self {
        General {
            nickname: "there".into(),
            launch_at_login: true,
            check_updates: true,
            default_page: "Home".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Appearance {
    pub theme: String,
    pub accent_index: usize,
    pub density: String,
    pub animation_speed: f32,
    pub reduce_motion: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            theme: "dark".into(),
            accent_index: 1,
            density: "Comfortable".into(),
            animation_speed: 0.5,
            reduce_motion: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Editor {
    pub font_size: f32,
    pub tab_width: usize,
    pub indent_with_spaces: bool,
    pub word_wrap: bool,
    pub line_numbers: bool,
    pub spellcheck: bool,
    pub prompt_template: String,
}

impl Default for Editor {
    fn default() -> Self {
        Editor {
            font_size: 13.0,
            tab_width: 4,
            indent_with_spaces: true,
            word_wrap: true,
            line_numbers: true,
            spellcheck: false,
            prompt_template: "You are a friendly assistant.\nKeep answers short.".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Advanced {
    pub endpoint: String,
    pub timeout_secs: f32,
    pub verify_tls: bool,
    pub log_level: String,
    pub experimental: bool,
}

impl Default for Advanced {
    fn default() -> Self {
        Advanced {
            endpoint: "https://api.local.dev:8443".into(),
            timeout_secs: 30.0,
            verify_tls: true,
            log_level: "info".into(),
            experimental: false,
        }
    }
}

impl Config {
    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| e.to_string())
    }

    pub fn from_toml_string(src: &str) -> Result<Self, String> {
        toml::from_str(src).map_err(|e| e.to_string())
    }
}
