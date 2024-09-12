#[derive(Default)]
pub struct TextToSpeechParams {
    pub speed_override: Option<f32>,
    pub voice_override: Option<String>,
}

pub struct TextToSpeechResult {
    pub bytes: Vec<u8>,
    pub mime_type: mxlink::mime::Mime,
}
