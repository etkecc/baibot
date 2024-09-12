pub fn create_error_message_text(text: &str) -> String {
    format!("⚠️ Error: {}", text)
}

pub fn create_success_message_text(text: &str) -> String {
    format!("✅ {}", text)
}

pub fn create_tooltip_message_text(text: &str) -> String {
    format!("💡 {}", text)
}
