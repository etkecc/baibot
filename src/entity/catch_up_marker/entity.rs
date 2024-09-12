use mxlink::matrix_sdk::ruma::events::macros::EventContent;

use serde::{Deserialize, Serialize};

use mxlink::helpers::account_data_config::GlobalConfig;
use mxlink::helpers::account_data_config::GlobalConfigCarrierContent;

#[derive(Clone, Debug, Default, Deserialize, Serialize, EventContent)]
#[ruma_event(type = "cc.etke.baibot.catch_up_marker", kind = GlobalAccountData)]
pub struct CatchUpMarkerCarrierContent {
    pub payload: String,
}

impl GlobalConfigCarrierContent for CatchUpMarkerCarrierContent {
    fn payload(&self) -> &str {
        &self.payload
    }

    fn new(payload: String) -> Self {
        Self { payload }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CatchUpMarker {
    pub caught_up_until_event_origin_server_ts_millis: i64,
}

impl CatchUpMarker {
    pub fn new(caught_up_until_event_origin_server_ts_millis: i64) -> Self {
        Self {
            caught_up_until_event_origin_server_ts_millis,
        }
    }
}

impl GlobalConfig for CatchUpMarker {}
