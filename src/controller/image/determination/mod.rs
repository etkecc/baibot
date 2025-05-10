use crate::controller::ControllerType;
mod tests;

pub fn determine_controller(
    text: &str,
) -> ControllerType {
    let text = text.trim();

    if let Some(prompt) = text.strip_prefix("create") {
        return ControllerType::ImageGeneration(prompt.trim().to_owned());
    }

    if let Some(prompt) = text.strip_prefix("edit") {
        return ControllerType::ImageEdit(prompt.trim().to_owned());
    }

    ControllerType::UsageHelp
}
