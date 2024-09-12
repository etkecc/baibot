#[cfg(test)]
mod tests;

use crate::{
    controller::ControllerType,
    entity::roomconfig::{TextGenerationAutoUsage, TextGenerationPrefixRequirementType},
    strings,
};

use super::super::controller_type::ConfigTextGenerationSettingRelatedControllerType;

pub(super) fn determine(
    text: &str,
) -> Result<ConfigTextGenerationSettingRelatedControllerType, ControllerType> {
    if let Some(remaining_text) = text.strip_prefix("context-management-enabled") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "context-management-enabled",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextGenerationSettingRelatedControllerType::GetContextManagementEnabled);
    }

    if let Some(value_string) = text.strip_prefix("set-context-management-enabled") {
        let value_string = value_string.trim().to_owned();
        let value_opt = if value_string.is_empty() {
            None
        } else {
            let value_string_lowercase = value_string.to_lowercase();
            Some(match value_string_lowercase.as_str() {
                "true" => true,
                "false" => false,
                _ => {
                    return Err(ControllerType::Error(
                        strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                    ));
                }
            })
        };

        return Ok(
            ConfigTextGenerationSettingRelatedControllerType::SetContextManagementEnabled(
                value_opt,
            ),
        );
    }

    if let Some(remaining_text) = text.strip_prefix("prefix-requirement-type") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "prefix-requirement-type",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextGenerationSettingRelatedControllerType::GetPrefixRequirementType);
    }

    if let Some(value_string) = text.strip_prefix("set-prefix-requirement-type") {
        let value_string = value_string.trim().to_owned();

        let value_choice = if value_string.is_empty() {
            None
        } else {
            let value_choice =
                TextGenerationPrefixRequirementType::from_str(&value_string.to_lowercase());

            if value_choice.is_none() {
                return Err(ControllerType::Error(
                    strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                ));
            }

            value_choice
        };

        return Ok(
            ConfigTextGenerationSettingRelatedControllerType::SetPrefixRequirementType(
                value_choice,
            ),
        );
    }

    if let Some(remaining_text) = text.strip_prefix("auto-usage") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "auto-usage",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextGenerationSettingRelatedControllerType::GetAutoUsage);
    }

    if let Some(value_string) = text.strip_prefix("set-auto-usage") {
        let value_string = value_string.trim().to_owned();

        let value_choice = if value_string.is_empty() {
            None
        } else {
            let value_choice = TextGenerationAutoUsage::from_str(&value_string.to_lowercase());

            if value_choice.is_none() {
                return Err(ControllerType::Error(
                    strings::cfg::configuration_value_unrecognized(&value_string).to_owned(),
                ));
            }

            value_choice
        };

        return Ok(ConfigTextGenerationSettingRelatedControllerType::SetAutoUsage(value_choice));
    }

    if let Some(remaining_text) = text.strip_prefix("prompt-override") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "prompt-override",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextGenerationSettingRelatedControllerType::GetPromptOverride);
    }

    if let Some(value_string) = text.strip_prefix("set-prompt-override") {
        let value_string = value_string.trim().to_owned();

        if value_string.is_empty() {
            return Ok(ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(None));
        }

        return Ok(
            ConfigTextGenerationSettingRelatedControllerType::SetPromptOverride(Some(value_string)),
        );
    }

    if let Some(remaining_text) = text.strip_prefix("temperature-override") {
        let remaining_text = remaining_text.trim();

        if !remaining_text.is_empty() {
            return Err(ControllerType::Error(
                strings::cfg::configuration_getter_used_with_extra_text(
                    "temperature-override",
                    remaining_text,
                )
                .to_owned(),
            ));
        }

        return Ok(ConfigTextGenerationSettingRelatedControllerType::GetTemperatureOverride);
    }

    if let Some(value_string) = text.strip_prefix("set-temperature-override") {
        let value_string = value_string.trim().to_owned();

        if value_string.is_empty() {
            return Ok(
                ConfigTextGenerationSettingRelatedControllerType::SetTemperatureOverride(None),
            );
        }

        let value_f32 = value_string.parse::<f32>();

        let Ok(value_f32) = value_f32 else {
            return Err(ControllerType::Error(
                strings::cfg::configuration_value_not_f32(&value_string).to_owned(),
            ));
        };

        return Ok(
            ConfigTextGenerationSettingRelatedControllerType::SetTemperatureOverride(Some(
                value_f32,
            )),
        );
    }

    Err(ControllerType::Unknown)
}
