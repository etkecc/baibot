const CONFIG_FILE_PATH: &str = "config.yml";

const NAME: &str = "baibot";
const COMMAND_PREFIX: &str = "!bai";

const PERSISTENCE_SESSION_FILE_NAME: &str = "session.json";
const PERSISTENCE_DB_DIR_NAME: &str = "db";

pub(crate) fn name() -> String {
    NAME.to_owned()
}

pub(crate) fn config_file_path() -> String {
    CONFIG_FILE_PATH.to_owned()
}

pub(super) fn command_prefix() -> String {
    COMMAND_PREFIX.to_owned()
}

pub(super) fn room_post_join_self_introduction_enabled() -> bool {
    true
}

pub(super) fn persistence_data_dir_path() -> Option<String> {
    None
}

pub(super) fn persistence_session_file_name() -> String {
    PERSISTENCE_SESSION_FILE_NAME.to_owned()
}

pub(super) fn persistence_db_dir_name() -> String {
    PERSISTENCE_DB_DIR_NAME.to_owned()
}

pub(super) fn logging() -> String {
    "warn,mxlink=debug,baibot=debug".to_owned()
}
