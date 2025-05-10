pub fn guide_how_to_proceed() -> String {
    let mut message = String::new();

    message.push_str("💡 Respond in this thread (in any order) with:\n");
    message.push_str("- one or more images: to use the given images for creating an edit\n");
    message.push_str("- more messages: to expand on your original prompt\n");
    message.push_str(
        "- a message saying `go`: to generate an edit with the current prompt\n",
    );
    message.push_str(
        "- a message saying `again`: to generate one more image edit with the current prompt\n",
    );

    message
}
