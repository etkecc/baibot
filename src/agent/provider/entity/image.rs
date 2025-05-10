use mxlink::mime;

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
    pub mime_type: mime::Mime,
    pub revised_prompt: Option<String>,
}

#[derive(Default)]
pub struct ImageEditParams {}

pub struct ImageEditResult {
    pub bytes: Vec<u8>,
    pub mime_type: mime::Mime,
}

pub struct ImageSource {
    pub filename: String,
    pub bytes: Vec<u8>,
    pub mime_type: mime::Mime,
}

impl ImageSource {
    pub fn new(filename: String, bytes: Vec<u8>, mime_type: mime::Mime) -> Self {
        Self {
            filename,
            bytes,
            mime_type,
        }
    }
}

impl From<ImageSource> for async_openai::types::ImageInput {
    fn from(value: ImageSource) -> Self {
        async_openai::types::ImageInput::from_vec_u8(
            value.filename,
            value.bytes,
            value.mime_type.to_string(),
        )
    }
}
