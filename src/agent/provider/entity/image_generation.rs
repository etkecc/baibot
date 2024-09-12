#[derive(Default)]
pub struct ImageGenerationParams {
    pub size_override: Option<String>,

    pub cheaper_model_switching_allowed: bool,

    pub cheaper_quality_switching_allowed: bool,
}

impl ImageGenerationParams {
    pub fn with_size_override(mut self, value: Option<String>) -> Self {
        self.size_override = value;
        self
    }

    pub fn with_cheaper_model_switching_allowed(mut self, value: bool) -> Self {
        self.cheaper_model_switching_allowed = value;
        self
    }

    pub fn with_cheaper_quality_switching_allowed(mut self, value: bool) -> Self {
        self.cheaper_quality_switching_allowed = value;
        self
    }
}

pub struct ImageGenerationResult {
    pub bytes: Vec<u8>,
    pub mime_type: mxlink::mime::Mime,
    pub revised_prompt: Option<String>,
}
