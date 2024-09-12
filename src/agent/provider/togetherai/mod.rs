use super::openai_compat::Config;

pub fn default_config() -> Config {
    let mut config = Config {
        base_url: "https://api.together.xyz/v1".to_owned(),

        text_to_speech: None,
        image_generation: None,
        speech_to_text: None,

        ..Default::default()
    };

    if let Some(ref mut config) = config.text_generation.as_mut() {
        config.model_id = "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".to_owned();
        config.max_context_tokens = 8192;
        config.max_response_tokens = 2048;
    }

    config
}
