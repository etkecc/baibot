use std::path::Path;
use std::sync::Arc;
use std::{future::Future, pin::Pin};

use mxlink::matrix_sdk::Room;
use mxlink::matrix_sdk::media::{MediaFormat, MediaRequestParameters};
use mxlink::matrix_sdk::ruma::{
    MilliSecondsSinceUnixEpoch, OwnedUserId, events::room::MediaSource,
};

use mxlink::{
    InitConfig, LoginConfig, LoginCredentials, LoginEncryption, MatrixLink, PersistenceConfig,
    TypingNoticeGuard,
};

use mxlink::helpers::account_data_config::{
    ConfigError, GlobalConfigManager as AccountDataGlobalConfigManager,
    RoomConfigManager as AccountDataRoomConfigManager,
};
use mxlink::helpers::encryption::Manager as EncryptionManager;

use crate::agent::Manager as AgentManager;
use crate::controller::chat_completion::message_aggregator::MessageAggregator;
use crate::entity::catch_up_marker::{
    CatchUpMarker, CatchUpMarkerManager, DelayedCatchUpMarkerManager,
};
use crate::entity::cfg::Config;
use crate::entity::globalconfig::{GlobalConfig, GlobalConfigurationManager};
use crate::entity::roomconfig::{RoomConfig, RoomConfigurationManager};

use crate::agent::Manager;

use crate::conversation::matrix::{RoomDisplayNameFetcher, RoomEventFetcher};
use crate::repository::sqlite::{SqliteBotRepository, SqliteConn};
use crate::repository::BotRepository;

const ROOM_EVENT_FETCHER_LRU_CACHE_SIZE: usize = 1000;
const ROOM_DISPLAY_NAME_FETCHER_LRU_CACHE_SIZE: usize = 1000;
const ROOM_CONFIG_MANAGER_LRU_CACHE_SIZE: usize = 1000;

const LOGO_BYTES: &[u8] = include_bytes!("../../etc/assets/baibot-torso-768.png");
const LOGO_MIME_TYPE: &str = "image/png";

/// Controls how often we persist the catch-up marker to Account Data.
/// Consult the `DelayedCatchUpMarkerManager` documentation for more information.
const DELAYED_CATCH_UP_MARKER_MANAGER_PERSIST_INTERVAL_DURATION: std::time::Duration =
    std::time::Duration::from_secs(10);

/// Controls what federation delay we will tolerate. The timestamp that gets persisted
/// will be based on the last seen event's `origin_server_ts` minus this duration.
/// Consult the `DelayedCatchUpMarkerManager` documentation for more information.
const DELAYED_CATCH_UP_MARKER_MANAGER_FEDERATION_DELAY_TOLERANCE_DURATION: std::time::Duration =
    std::time::Duration::from_secs(90);

struct BotInner {
    config: Config,

    matrix_link: MatrixLink,
    delayed_catch_up_marker_manager: DelayedCatchUpMarkerManager,
    global_config_manager: tokio::sync::Mutex<GlobalConfigurationManager>,
    room_config_manager: tokio::sync::Mutex<RoomConfigurationManager>,
    room_event_fetcher: Arc<RoomEventFetcher>,
    room_display_name_fetcher: Arc<RoomDisplayNameFetcher>,
    agent_manager: Manager,
    admin_pattern_regexes: Vec<regex::Regex>,
    chat_completion_message_aggregator: Arc<MessageAggregator>,
    repository: Arc<dyn BotRepository>
}

/// Bot represents a bot instance.
///
/// All of the state is held in an `Arc` so the `Bot` can be cloned freely.
#[derive(Clone)]
pub struct Bot {
    inner: Arc<BotInner>,
}

impl Bot {
    pub fn chat_completion_message_aggregator(&self) -> Arc<MessageAggregator> {
        Arc::clone(&self.inner.chat_completion_message_aggregator)
    }

    pub fn repository(&self) -> Arc<dyn BotRepository> {
        Arc::clone(&self.inner.repository)
    }

    pub async fn new(config: Config) -> anyhow::Result<Self> {
        // Take some potentially problematic configuration values out of the config early on.
        // If we'd be failing, we'd like it to happen early, before we log in, etc.

        let initial_global_config: GlobalConfig =
            config.initial_global_config.clone().try_into()?;

        let admin_pattern_regexes = config.access.admin_pattern_regexes()?;
        let persistence_config_encryption_key = config.persistence.config_encryption_key()?;

        let agent_manager = AgentManager::new(config.agents.static_definitions.clone())?;

        let encryption_manager = EncryptionManager::new(persistence_config_encryption_key);

        let matrix_link = create_matrix_link(&config).await?;

        let catch_up_marker_manager = create_catch_up_marker_manager(matrix_link.clone());

        let delayed_catch_up_marker_manager = DelayedCatchUpMarkerManager::new(
            catch_up_marker_manager,
            DELAYED_CATCH_UP_MARKER_MANAGER_PERSIST_INTERVAL_DURATION,
            DELAYED_CATCH_UP_MARKER_MANAGER_FEDERATION_DELAY_TOLERANCE_DURATION,
        );

        let global_config_manager = tokio::sync::Mutex::new(create_global_configuration_manager(
            matrix_link.clone(),
            encryption_manager.clone(),
            initial_global_config,
        ));

        let room_config_manager = tokio::sync::Mutex::new(create_room_configuration_manager(
            matrix_link.clone(),
            encryption_manager.clone(),
        ));

        let room_event_fetcher = RoomEventFetcher::new(Some(ROOM_EVENT_FETCHER_LRU_CACHE_SIZE));

        let room_display_name_fetcher = RoomDisplayNameFetcher::new(
            matrix_link.clone(),
            Some(ROOM_DISPLAY_NAME_FETCHER_LRU_CACHE_SIZE),
        );

        let chat_completion_message_aggregator =
            MessageAggregator::new(config.chat_completion_aggregator.clone());

        let sqlite_conn = SqliteConn::new(Path::new(&config.sqlite_db_path));
        let repository = SqliteBotRepository::new(Arc::new(sqlite_conn));

        Ok(Self {
            inner: Arc::new(BotInner {
                config,

                matrix_link,
                delayed_catch_up_marker_manager,
                global_config_manager,
                room_config_manager,
                room_event_fetcher: Arc::new(room_event_fetcher),
                room_display_name_fetcher: Arc::new(room_display_name_fetcher),
                agent_manager,
                admin_pattern_regexes,
                chat_completion_message_aggregator: Arc::new(chat_completion_message_aggregator),
                repository: Arc::new(repository)
            }),
        })
    }

    pub fn bot_uniqe_id(&self) -> String {
        self.inner.config.uniqe_bot_id.clone()
    }
    
    pub(crate) fn admin_patterns(&self) -> &Vec<String> {
        &self.inner.config.access.admin_patterns
    }

    pub(crate) fn name(&self) -> &str {
        &self.inner.config.user.name
    }

    pub(crate) fn command_prefix(&self) -> &str {
        &self.inner.config.command_prefix
    }

    pub(crate) fn post_join_self_introduction_enabled(&self) -> bool {
        self.inner.config.room.post_join_self_introduction_enabled
    }

    pub(crate) fn homeserver_name(&self) -> &str {
        &self.inner.config.homeserver.server_name
    }

    pub(crate) fn global_config_manager(&self) -> &tokio::sync::Mutex<GlobalConfigurationManager> {
        &self.inner.global_config_manager
    }

    pub(crate) fn room_config_manager(&self) -> &tokio::sync::Mutex<RoomConfigurationManager> {
        &self.inner.room_config_manager
    }

    pub(crate) fn room_event_fetcher(&self) -> Arc<RoomEventFetcher> {
        self.inner.room_event_fetcher.clone()
    }

    pub(crate) fn room_display_name_fetcher(&self) -> Arc<RoomDisplayNameFetcher> {
        self.inner.room_display_name_fetcher.clone()
    }

    pub(crate) fn agent_manager(&self) -> &Manager {
        &self.inner.agent_manager
    }

    pub(crate) fn matrix_link(&self) -> &MatrixLink {
        &self.inner.matrix_link
    }

    pub(crate) fn user_id(&self) -> &OwnedUserId {
        self.matrix_link().user_id()
    }

    pub(crate) async fn user_display_name_in_room(&self, room: &Room) -> Option<String> {
        let bot_display_name = self
            .room_display_name_fetcher()
            .own_display_name_in_room(room)
            .await;

        match bot_display_name {
            Ok(value) => value,
            Err(err) => {
                tracing::warn!(
                    ?err,
                    "Failed to fetch bot display name. Proceeding without it"
                );
                None
            }
        }
    }

    pub(crate) fn reacting(&self) -> super::reacting::Reacting {
        super::reacting::Reacting::new(self.clone())
    }

    pub(crate) fn rooms(&self) -> super::rooms::Rooms {
        super::rooms::Rooms::new(self.clone())
    }

    pub(crate) fn messaging(&self) -> super::messaging::Messaging {
        super::messaging::Messaging::new(self.clone())
    }

    pub(crate) fn admin_pattern_regexes(&self) -> &Vec<regex::Regex> {
        &self.inner.admin_pattern_regexes
    }

    pub(crate) async fn global_config(&self) -> Result<GlobalConfig, ConfigError> {
        let mut global_config_manager_guard = self.inner.global_config_manager.lock().await;

        global_config_manager_guard.get_or_create().await
    }

    pub(crate) async fn is_caught_up(
        &self,
        event_origin_server_ts: MilliSecondsSinceUnixEpoch,
    ) -> Result<bool, ConfigError> {
        self.inner
            .delayed_catch_up_marker_manager
            .is_caught_up(event_origin_server_ts.0.into())
            .await
    }

    pub(crate) async fn catch_up(&self, event_origin_server_ts: MilliSecondsSinceUnixEpoch) {
        self.inner
            .delayed_catch_up_marker_manager
            .catch_up(event_origin_server_ts.0.into())
            .await
    }

    pub(crate) async fn start_typing_notice(&self, room: &Room) -> TypingNoticeGuard {
        self.inner
            .matrix_link
            .rooms()
            .start_typing_notice(room)
            .await
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        self.rooms().attach_event_handlers().await;
        self.messaging().attach_event_handlers().await;
        self.reacting().attach_event_handlers().await;

        self.inner.delayed_catch_up_marker_manager.start().await;

        self.prepare_profile().await?;

        let cloned_aggregator = Arc::clone(&self.inner.chat_completion_message_aggregator);

        let chat_completion_message_aggregator_handler =
            tokio::spawn(async move { cloned_aggregator.listen().await });

        let cloened_inner = Arc::clone(&self.inner);

        let bot_runner = tokio::spawn(async move {
            cloened_inner
                .matrix_link
                .start()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to sync: {:?}", e))
        });

        chat_completion_message_aggregator_handler.await.unwrap();
        bot_runner.await.unwrap()
    }

    async fn prepare_profile(&self) -> anyhow::Result<()> {
        use std::time::Duration;
        use tokio::time::sleep;

        let mut delay = Duration::from_secs(3);
        let max_delay = Duration::from_secs(30);

        loop {
            match self.do_prepare_profile().await {
                Ok(_) => return Ok(()),
                Err(err) => {
                    tracing::warn!(
                        ?err,
                        ?delay,
                        "Failed to prepare profile.. Will retry after delay..."
                    );

                    sleep(delay).await;

                    delay = std::cmp::min(delay * 2, max_delay);
                }
            }
        }
    }

    async fn do_prepare_profile(&self) -> anyhow::Result<()> {
        tracing::debug!("Preparing profile..");

        let account = self.inner.matrix_link.client().account();
        let media = self.inner.matrix_link.client().media();

        let desired_display_name = self.inner.config.user.name.clone();

        let profile = account
            .fetch_user_profile()
            .await
            .map_err(|e| anyhow::anyhow!("Failed fetching profile: {:?}", e))?;

        let should_update_display_name = match &profile.displayname {
            Some(displayname) => displayname != &desired_display_name,
            None => true,
        };

        if should_update_display_name {
            tracing::info!(
                ?profile.displayname,
                ?desired_display_name,
                "Updating display name.."
            );

            if let Err(err) = account.set_display_name(Some(&desired_display_name)).await {
                return Err(anyhow::anyhow!("Failed setting display name: {:?}", err));
            }
        }

        let should_update_avatar = match &profile.avatar_url {
            Some(avatar_url) => {
                let request = MediaRequestParameters {
                    source: MediaSource::Plain(avatar_url.to_owned()),
                    format: MediaFormat::File,
                };

                let content = media
                    .get_media_content(&request, true)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed fetching existing avatar: {:?}", e))?;

                content.as_slice() != LOGO_BYTES
            }
            None => true,
        };

        if should_update_avatar {
            tracing::info!("Updating avatar..");

            let mime_type = LOGO_MIME_TYPE
                .parse()
                .expect("Failed parsing mime type for logo");

            account
                .upload_avatar(&mime_type, LOGO_BYTES.to_vec())
                .await
                .map_err(|e| anyhow::anyhow!("Failed uploading avatar: {:?}", e))?;
        }

        Ok(())
    }
}

async fn create_matrix_link(config: &Config) -> anyhow::Result<MatrixLink> {
    let session_file_path = config.persistence.session_file_path()?;
    let session_encryption_key = config.persistence.session_encryption_key()?;
    let db_dir_path: std::path::PathBuf = config.persistence.db_dir_path()?;

    let login_creds = LoginCredentials::UserPassword(
        config.user.mxid_localpart.to_owned(),
        config.user.password.to_owned(),
    );

    let login_encryption = LoginEncryption::new(
        config.user.encryption.recovery_passphrase.clone(),
        config.user.encryption.recovery_reset_allowed,
    );

    let login_config = LoginConfig::new(
        config.homeserver.url.to_owned(),
        login_creds,
        Some(login_encryption),
        config.user.name.to_owned(),
    );

    let persistence_config =
        PersistenceConfig::new(session_file_path, session_encryption_key, db_dir_path);

    let init_config = InitConfig::new(login_config, persistence_config);

    mxlink::init(&init_config).await.map_err(|e| e.into())
}

pub fn create_global_configuration_manager(
    matrix_link: MatrixLink,
    encryption_manager: EncryptionManager,
    initial_global_config: GlobalConfig,
) -> GlobalConfigurationManager {
    let initial_global_config_callback = move || {
        let initial_global_config = initial_global_config.clone();

        let future = create_initial_global_config(initial_global_config);

        // Explicitly box the future to match the expected type
        Box::pin(future) as Pin<Box<dyn Future<Output = GlobalConfig> + Send>>
    };

    AccountDataGlobalConfigManager::new(
        matrix_link,
        encryption_manager,
        initial_global_config_callback,
    )
}

async fn create_initial_global_config(initial_global_config: GlobalConfig) -> GlobalConfig {
    initial_global_config
}

pub fn create_room_configuration_manager(
    matrix_link: MatrixLink,
    encryption_manager: EncryptionManager,
) -> RoomConfigurationManager {
    let initial_room_config_callback = |room: Room| {
        let future = create_initial_room_config(room);

        // Explicitly box the future to match the expected type
        Box::pin(future) as Pin<Box<dyn Future<Output = RoomConfig> + Send>>
    };

    AccountDataRoomConfigManager::new(
        matrix_link.user_id().clone(),
        encryption_manager,
        initial_room_config_callback,
        Some(ROOM_CONFIG_MANAGER_LRU_CACHE_SIZE),
    )
}

async fn create_initial_room_config(room: Room) -> RoomConfig {
    RoomConfig::default().with_room(room).await
}

pub fn create_catch_up_marker_manager(matrix_link: MatrixLink) -> CatchUpMarkerManager {
    let initial_global_config_callback = || {
        let future = create_initial_catch_up_marker();

        // Explicitly box the future to match the expected type
        Box::pin(future) as Pin<Box<dyn Future<Output = CatchUpMarker> + Send>>
    };

    // Intentionally not using encryption, to make this resilient even if we lose our encryption key.
    // We're not worried about the catch-up marker being read or tampered with, as it's not sensitive data.
    let encryption_manager = EncryptionManager::new(None);

    let catch_up_marker_manager: CatchUpMarkerManager = AccountDataGlobalConfigManager::new(
        matrix_link.clone(),
        encryption_manager,
        initial_global_config_callback,
    );

    catch_up_marker_manager
}

async fn create_initial_catch_up_marker() -> CatchUpMarker {
    CatchUpMarker::new(0)
}
