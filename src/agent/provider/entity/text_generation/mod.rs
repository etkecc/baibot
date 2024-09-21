mod prompt_variables;

pub use prompt_variables::TextGenerationPromptVariables;

#[derive(Default)]
pub struct TextGenerationParams {
    pub context_management_enabled: bool,
    pub prompt_override: Option<String>,
    pub temperature_override: Option<f32>,
    pub prompt_variables: TextGenerationPromptVariables,
}

pub struct TextGenerationResult {
    pub text: String,
}
