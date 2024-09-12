mod delayed_catch_up_marker_manager;
mod entity;

use mxlink::helpers::account_data_config::GlobalConfigManager as AccountDataGlobalConfigManager;

pub use entity::{CatchUpMarker, CatchUpMarkerCarrierContent};

pub type CatchUpMarkerManager =
    AccountDataGlobalConfigManager<CatchUpMarker, CatchUpMarkerCarrierContent>;

pub use delayed_catch_up_marker_manager::DelayedCatchUpMarkerManager;
