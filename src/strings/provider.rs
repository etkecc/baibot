use crate::agent::AgentInstantiationError;
use crate::agent::AgentProvider;
use crate::agent::AgentProviderInfo;
use crate::agent::AgentPurpose;

pub fn invalid(provider: &str) -> String {
    let choices_string = AgentProvider::choices()
        .iter()
        .map(|choice| format!("`{}`", choice.to_static_str(),))
        .collect::<Vec<String>>()
        .join(", ");

    format!(
        "`{}` is not a valid provider choice. Valid choices are: {}",
        provider, choices_string
    )
}

pub fn invalid_configuration_for_provider(
    provider: &AgentProvider,
    err: AgentInstantiationError,
) -> String {
    format!(
        "The provided configuration is not valid for the `{}` provider:\n```\n{:?}\n```",
        provider, err
    )
}

pub fn providers_list_intro() -> String {
    "The list of supported providers is below.".to_owned()
}

pub fn help_how_to_choose_heading() -> String {
    "How to choose a provider".to_string()
}

pub fn help_how_to_choose_description(command_prefix: &str) -> String {
    let str = r#"
If you're not sure which provider to start with, **we recommend OpenAI** as it's the most popular and has the **widest range of capabilities**.

You don't need to choose just one though. The bot supports **mixing & matching models** (by setting different handlers for different types of messages - see `%command_prefix% config`), so you can use multiple providers at the same time.
"#;

    str.replace("%command_prefix%", command_prefix)
        .trim()
        .to_owned()
}

pub fn help_how_to_use_heading() -> String {
    "How to use a provider".to_string()
}

pub fn help_how_to_use_description(command_prefix: &str) -> String {
    let str = r#"
1. 📝 **Sign up for it**
2. 🔑 **Obtain an API key**
3. 🤖 %create_one_or_more_agents%
4. 🤝 %set_new_agent_as_handler%
"#;

    str.replace("%command_prefix%", command_prefix)
        .replace(
            "%create_one_or_more_agents%",
            &super::introduction::create_one_or_more_agents(command_prefix),
        )
        .replace(
            "%set_new_agent_as_handler%",
            &super::introduction::set_new_agent_as_handler(command_prefix),
        )
        .trim()
        .to_owned()
}

pub fn help_provider_heading(provider_name: &str, homepage_url: &Option<String>) -> String {
    match homepage_url {
        Some(url) => format!("[{}]({})", provider_name, url),
        None => provider_name.to_owned(),
    }
}

pub fn help_provider_details(id: &str, info: &AgentProviderInfo) -> String {
    let mut message = String::new();

    message.push_str(info.description.trim());
    message.push_str("\n\n");

    message.push_str(&format!("- 🆔 Identifier: `{}`\n", id));

    let mut links = Vec::new();
    if let Some(url) = info.homepage_url {
        links.push(format!("[🏠 Home page]({})", url));
    }
    if let Some(url) = info.wiki_url {
        links.push(format!("[🌐 Wiki]({})", url));
    }
    if let Some(url) = info.sign_up_url {
        links.push(format!("[👤 Sign up]({})", url));
    }
    if let Some(url) = info.models_list_url {
        links.push(format!("[📋 Models list]({})", url));
    }

    if !links.is_empty() {
        message.push_str(&format!("- 🔗 Links: {}\n", links.join(", ")));
    }

    let mut capabilities = vec![];
    for purpose in info.supported_purposes.iter() {
        let mut purpose_line = format!("{} {}", purpose.emoji(), purpose.as_str());

        if let AgentPurpose::TextGeneration = purpose {
            if info.text_generation_supports_vision {
                purpose_line = format!("{} ({})", purpose_line, "incl. vision");
            } else {
                purpose_line = format!("{} ({})", purpose_line, "no vision");
            }
        }

        capabilities.push(purpose_line);
    }

    message.push_str(&format!("- 🌟 Capabilities: {}\n", capabilities.join(", ")));

    message
}
