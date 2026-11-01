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

    #[serde(default)]
    pub max_response_tokens: u32,

    #[serde(default)]
    pub max_context_tokens: u32,
}

impl Default for TextGenerationConfig {
    fn default() -> Self {
        Self {
            model_id: default_text_model_id(),
            prompt: Some(default_prompt().to_owned()),
            max_response_tokens: 16_384,
            max_context_tokens: 204_800,
        }
    }
}

fn default_text_model_id() -> String {
    "claude-sonnet-5-5".to_owned()
}

#[cfg(test)]
mod tests {
    use super::TextGenerationConfig;

    #[test]
    fn new_agents_use_sonnet_and_a_configured_temperature_is_ignored() {
        let defaults = TextGenerationConfig::default();
        assert_eq!(defaults.model_id, "claude-sonnet-5-5");

        let existing: TextGenerationConfig =
            serde_yaml_ng::from_str("model_id: claude-sonnet-4-5\ntemperature: 1.0\n").unwrap();
        assert_eq!(existing.model_id, "claude-sonnet-4-5");
    }
}
