mod entity;
mod room_display_name_fetcher;
mod room_event_fetcher;
mod utils;

pub(crate) use room_display_name_fetcher::RoomDisplayNameFetcher;
pub(crate) use room_event_fetcher::RoomEventFetcher;

pub(crate) use entity::{
    ConversationBound, MatrixMessage, MatrixMessageContent, MatrixMessageProcessingParams,
    SenderFilter,
};

pub(crate) use utils::*;

/// A content field which marks messages from the bot containing secrets (like API keys),
/// so that the bot never passes them on (e.g. to a model, when reading the room's history).
pub(crate) const CONTAINS_SECRETS_FIELD: &str = "cc.etke.baibot.contains_secrets";

/// How the bot's messages showing an agent's configuration start (see `agent details`).
///
/// Older versions didn't mark those revealing secrets (see [`CONTAINS_SECRETS_FIELD`]), so all of them are
/// treated as if they contained secrets.
pub(crate) const AGENT_CONFIGURATION_MESSAGE_PREFIX: &str = "Configuration for agent `";
