pub fn unknown_command_see_help(command_prefix: &str) -> String {
    format!("Unknown command. See help (`{command_prefix} help`).")
}

pub fn error_while_processing_message() -> &'static str {
    "An error occurred while processing your message. Please try again."
}

pub fn message_is_encrypted() -> &'static str {
    "This message is encrypted and I cannot decrypt it right now, so I cannot properly serve you."
}

pub fn first_message_in_thread_is_encrypted() -> &'static str {
    "The first message in this chat thread is encrypted and I cannot decrypt it right now, so I cannot properly serve you."
}
