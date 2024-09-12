pub mod access;
pub mod agent;
pub mod cfg;
pub mod provider;
pub mod usage;

pub fn heading_introduction() -> String {
    "👋 Introduction".to_owned()
}

pub fn available_commands_intro() -> &'static str {
    "You can run the following commands:"
}

pub fn learn_more_send_a_command(command_prefix: &str, command_parts: &str) -> String {
    format!("To learn more, send a `{command_prefix} {command_parts}` command.")
}
