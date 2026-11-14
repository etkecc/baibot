pub mod defaults;
mod entity;

use mxlink::helpers::account_data_config::RoomConfigManager as AccountDataRoomConfigManager;

pub use entity::{RoomConfig, RoomConfigCarrierContent, RoomSettings, RoomSettingsHandler};
pub use entity::{
    SpeechToTextFlowType, SpeechToTextMessageTypeForNonThreadedOnlyTranscribedMessages,
    TextGenerationAutoUsage, TextGenerationPrefixRequirementType, TextGenerationSenderContextMode,
    TextToSpeechBotMessagesFlowType, TextToSpeechUserMessagesFlowType,
};

pub type RoomConfigurationManager =
    AccountDataRoomConfigManager<RoomConfig, RoomConfigCarrierContent>;

/// Changes the room's configuration without losing concurrent changes.
///
/// While holding the manager's lock, it reads the current configuration (not a copy taken when
/// the triggering message arrived), lets `change` modify it, and persists it.
/// If `change` returns `None`, nothing is persisted.
pub async fn update<R>(
    manager: &tokio::sync::Mutex<RoomConfigurationManager>,
    room: &mxlink::matrix_sdk::Room,
    change: impl FnOnce(&mut RoomConfig) -> Option<R>,
) -> anyhow::Result<Option<R>> {
    let manager = manager.lock().await;

    let mut room_config = manager.get_or_create_for_room(room).await?;

    let Some(result) = change(&mut room_config) else {
        return Ok(None);
    };

    manager.persist(room, &room_config).await?;

    Ok(Some(result))
}
