pub fn redaction_reason_done() -> &'static str {
    "Done transcribing"
}

pub fn redaction_reason_failed() -> &'static str {
    "Failed while transcribing"
}

pub fn language_code_invalid(value: &str) -> String {
    format!("The value `{}` is not a valid 2-letter language code as per [ISO 639-1](https://en.wikipedia.org/wiki/List_of_ISO_639_language_codes).", value)
}
