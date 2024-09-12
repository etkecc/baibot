pub fn revised_prompt(prompt: &str) -> String {
    format!("💭 Revised prompt to: {}", prompt)
}

pub fn guide_how_to_proceed() -> String {
    let mut message = String::new();

    message.push_str("💡 Respond in this thread with:\n");
    message.push_str("- more messages: to expand on your original prompt\n");
    message.push_str(
        "- a message saying `again`: to generate one more image with the current prompt.\n",
    );

    message
}
