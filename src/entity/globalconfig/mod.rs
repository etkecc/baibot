mod entity;

use mxlink::helpers::account_data_config::GlobalConfigManager as AccountDataGlobalConfigManager;

pub use entity::{GlobalConfig, GlobalConfigAccess, GlobalConfigCarrierContent};

pub type GlobalConfigurationManager =
    AccountDataGlobalConfigManager<GlobalConfig, GlobalConfigCarrierContent>;

/// Changes the global configuration without losing concurrent changes.
///
/// While holding the manager's lock, it reads the current configuration (not a copy taken when
/// the triggering message arrived), lets `change` modify it, and persists it.
/// If `change` returns `None`, nothing is persisted.
pub async fn update<R>(
    manager: &tokio::sync::Mutex<GlobalConfigurationManager>,
    change: impl FnOnce(&mut GlobalConfig) -> Option<R>,
) -> anyhow::Result<Option<R>> {
    let mut manager = manager.lock().await;

    let mut global_config = manager.get_or_create().await?;

    let Some(result) = change(&mut global_config) else {
        return Ok(None);
    };

    manager.persist(&global_config).await?;

    Ok(Some(result))
}
