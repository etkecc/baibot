mod entity;

use mxlink::helpers::account_data_config::GlobalConfigManager as AccountDataGlobalConfigManager;

pub use entity::{GlobalConfig, GlobalConfigAccess, GlobalConfigCarrierContent};

pub type GlobalConfigurationManager =
    AccountDataGlobalConfigManager<GlobalConfig, GlobalConfigCarrierContent>;
