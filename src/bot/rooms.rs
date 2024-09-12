use mxlink::{
    matrix_sdk::{
        ruma::events::{room::member::StrippedRoomMemberEvent, AnySyncTimelineEvent},
        Room,
    },
    InvitationDecision,
};

use mxlink::CallbackError;

use tracing::Instrument;

use crate::entity::RoomConfigContext;

#[derive(Clone)]
pub struct Rooms {
    bot: super::Bot,
}

impl Rooms {
    pub fn new(bot: super::Bot) -> Self {
        Self { bot }
    }

    pub(super) async fn attach_event_handlers(&self) {
        let matrix_link_rooms = self.bot.matrix_link().rooms();

        let this = self.clone();
        matrix_link_rooms.on_being_last_member(|event, room| async move {
            this.on_being_last_member(event, room).await
        });

        let this = self.clone();
        matrix_link_rooms
            .on_invitation(|event, room| async move { this.on_invitation(event, room).await });

        let this = self.clone();
        matrix_link_rooms.on_joined(|event, room| async move { this.on_joined(event, room).await });
    }

    async fn on_invitation(
        &self,
        room_member: StrippedRoomMemberEvent,
        _room: Room,
    ) -> Result<InvitationDecision, CallbackError> {
        tracing::debug!("Deciding on room invitation");

        let global_config = self
            .bot
            .global_config()
            .await
            .map_err(|e| CallbackError::Unknown(e.into()))?;

        let sender_is_admin = mxidwc::match_user_id(
            room_member.sender.clone().as_str(),
            self.bot.admin_pattern_regexes(),
        );

        let sender_is_allowed_user = match &global_config.access.user_patterns {
            Some(user_patterns) => {
                let allowed_user_regexes = mxidwc::parse_patterns_vector(user_patterns)
                    .map_err(|e| CallbackError::Unknown(e.into()))?;

                mxidwc::match_user_id(room_member.sender.clone().as_str(), &allowed_user_regexes)
            }
            None => false,
        };

        if !(sender_is_admin || sender_is_allowed_user) {
            return Ok(InvitationDecision::Reject);
        }

        Ok(InvitationDecision::Join)
    }

    #[tracing::instrument(name = "bot_on_joined", skip_all, fields(room_id = room.room_id().as_str(), event_id = event.event_id().as_str()))]
    async fn on_joined(
        &self,
        event: AnySyncTimelineEvent,
        room: Room,
    ) -> Result<(), CallbackError> {
        if self
            .bot
            .is_caught_up(event.origin_server_ts())
            .await
            .map_err(|e| {
                CallbackError::Unknown(
                    format!("Failed to determine catch-up state: {:?}", e).into(),
                )
            })?
        {
            tracing::debug!(
                event_origin_server_ts = format!("{:?}", event.origin_server_ts()),
                "Ignoring old room join event",
            );

            return Ok(());
        }

        tracing::info!("Handling room join");

        let global_config = self
            .bot
            .global_config()
            .await
            .map_err(|e| CallbackError::Unknown(e.into()))?;

        let room_config_manager = self.bot.room_config_manager().lock().await;

        // We force-create a new config when we join anew to ensure we:
        // - always start from a known clean state
        // - record the last join timestamp, so we can accurately service the room (ignoring past messages, etc.)
        let room_config = room_config_manager
            .create_new_for_room(&room)
            .await
            .map_err(|e| CallbackError::Unknown(e.into()))?;

        let room_config_context = RoomConfigContext::new(global_config, room_config);

        let event_span = tracing::error_span!("join_controller");

        let result = crate::controller::join::handle(&self.bot, &room, &room_config_context)
            .instrument(event_span)
            .await
            .map_err(|e| CallbackError::Unknown(e.into()));

        self.bot.catch_up(event.origin_server_ts()).await;

        result
    }

    async fn on_being_last_member(
        &self,
        _event: AnySyncTimelineEvent,
        room: mxlink::matrix_sdk::Room,
    ) -> Result<(), CallbackError> {
        tracing::info!(
            "Leaving room {} because we are the last member",
            room.room_id()
        );

        // We are last in this room. Let's just leave
        room.leave().await.map_err(|e| e.into())
    }
}
