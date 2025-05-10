use base64::{Engine as _, engine::general_purpose::STANDARD};

pub(crate) fn base64_decode(base64_string: &str) -> Result<Vec<u8>, base64::DecodeError> {
    STANDARD.decode(base64_string)
}

pub(crate) fn base64_encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}
