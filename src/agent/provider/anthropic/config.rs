use serde::{Deserialize, Serialize};

use crate::agent::{default_prompt, provider::ConfigTrait};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,

    pub api_key: String,

    pub text_generation: Option<TextGenerationConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "https://api.anthropic.com/v1".to_owned(),
            api_key: "YOUR_API_KEY_HERE".to_owned(),
            text_generation: Some(TextGenerationConfig::default()),
        }
    }
}

impl ConfigTrait for Config {
    fn validate(&self) -> Result<(), String> {
        if self.base_url.is_empty() {
            return Err("The base URL must not be empty.".to_owned());
        }
        if !self.base_url.ends_with("/v1") {
            return Err("The base URL must end with '/v1'.".to_owned());
        }
        if self.api_key.is_empty() {
            return Err("The API key must not be empty.".to_owned());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextGenerationConfig {
    #[serde(default = "default_text_model_id")]
    pub model_id: String,

    #[serde(default)]
    pub prompt: Option<String>,

    #[serde(default = "super::super::default_temperature")]
    pub temperature: f32,

    #[serde(default)]
    pub max_response_tokens: u32,

    #[serde(default)]
    pub max_context_tokens: u32,

    #[serde(default)]
    pub tools: ToolsConfig,
}

impl Default for TextGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_model_id(),
            prompt: Some(default_prompt().to_owned()),
            temperature: super::super::default_temperature(),
            max_response_tokens: 8192,
            max_context_tokens: 204_800,
            tools: ToolsConfig::default(),
        }
    }
}

fn default_text_model_id() -> String {
    "claude-sonnet-5-5".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub web_search: bool,

    #[serde(default)]
    pub web_fetch: bool,
}

#[cfg(test)]
mod tests {
    use super::TextGenerationConfig;

    #[test]
    fn tools_are_disabled_by_default_and_explicit_values_load() {
        let defaults = TextGenerationConfig::default();
        assert!(!defaults.tools.web_search);
        assert!(!defaults.tools.web_fetch);

        let missing: TextGenerationConfig = serde_yaml_ng::from_str("{}\n").unwrap();
        assert!(!missing.tools.web_search);
        assert!(!missing.tools.web_fetch);

        let enabled: TextGenerationConfig =
            serde_yaml_ng::from_str("tools:\n  web_search: true\n  web_fetch: true\n").unwrap();
        assert!(enabled.tools.web_search);
        assert!(enabled.tools.web_fetch);
    }
}
