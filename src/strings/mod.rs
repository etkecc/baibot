pub mod access;
pub mod agent;
pub mod cfg;
pub mod error;
pub mod global_config;
pub mod help;
pub mod image_generation;
pub mod introduction;
pub mod provider;
pub mod room_config;
pub mod speech_to_text;
pub mod text_to_speech;
pub mod usage;

pub const PROGRESS_INDICATOR_EMOJI: &str = "⏳";

pub fn the_following_commands_are_available() -> &'static str {
    "The following commands are available:"
}
