pub fn heading() -> String {
    "🤖 Agents".to_owned()
}

pub fn intro(command_prefix: &str) -> String {
    format!("An agent is an instantiation and configuration of some **☁️ provider** (see `{command_prefix} provider`).")
}

pub fn intro_handler_relation(command_prefix: &str) -> String {
    format!(
        "Agents can be set as **handlers for various purposes** (text-generation, speech-to-text, etc.) globally or in specific rooms. Send a `{command_prefix} config status` command to see the current configuration."
    )
}

pub fn intro_capabilities() -> String {
    "It can support different capabilities (text-generation, speech-to-text, etc.) depending on the provider used and on the configuration of the agent.".to_string()
}

pub fn no_permission_to_create_agents() -> &'static str {
    "⚠️ You are neither a bot administrator, nor a room-local agent manager, so **you cannot create new agents by yourself**."
}

pub fn list_agents(command_prefix: &str) -> String {
    format!("- **List** all available agents: `{command_prefix} agent list`")
}

pub fn show_agent_details(command_prefix: &str) -> String {
    format!("- **Show** full details for a given agent: `{command_prefix} agent details FULL_AGENT_IDENTIFIER`")
}

pub fn create_agent_intro() -> &'static str {
    "- **Create** a new agent:"
}

pub fn create_agent_room_local(command_prefix: &str) -> String {
    format!("\t- (Accessible in **this room only**) `{command_prefix} agent create-room-local PROVIDER_ID AGENT_ID`")
}

pub fn create_agent_global(command_prefix: &str) -> String {
    format!("\t- (Accessible in **all rooms**) `{command_prefix} agent create-global PROVIDER_ID AGENT_ID`")
}

pub fn create_agent_example(command_prefix: &str) -> String {
    format!("\t- Example: `{command_prefix} agent create-room-local openai my-openai-agent`")
}

pub fn delete_agent(command_prefix: &str) -> String {
    format!("- **Delete** an agent: `{command_prefix} agent delete FULL_AGENT_IDENTIFIER`")
}

pub fn available_commands_outro_update_note() -> &'static str {
    "To **update** a given agent's configuration: show the agent's **details** (current configuration), then **delete** it and finally **re-create** it."
}
