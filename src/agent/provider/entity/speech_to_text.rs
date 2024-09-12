#[derive(Default)]
pub struct SpeechToTextParams {
    pub language_override: Option<String>,
}

pub struct SpeechToTextResult {
    pub text: String,
}
