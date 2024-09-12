mod entity;

use mxlink::helpers::account_data_config::GlobalConfigManager as AccountDataGlobalConfigManager;

pub use entity::{GlobalConfig, GlobalConfigCarrierContent};

pub type GlobalConfigurationManager =
    AccountDataGlobalConfigManager<GlobalConfig, GlobalConfigCarrierContent>;
