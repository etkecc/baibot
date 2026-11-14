## 🤖 Agents

An agent is an instantiation and configuration of some [☁️ provider](./providers.md).
It can support different capabilities (text-generation, speech-to-text, etc.) depending on the provider used and on the configuration of the agent.

Agents can be set as **[🤝 handlers](./configuration/handlers.md) for various purposes** (text-generation, speech-to-text, etc.) globally or in specific rooms. Send a `!bai config status` command to see the current configuration.

Agents can be **defined [statically](./configuration/README.md#static-configuration)** (in the server configuration) **or dynamically** (via commands sent to the bot).

When [creating agents](#creating-agents) dynamically, you can do it **per-room or globally**.
Globally-defined agents can be used by any authorized bot user in any room, while room-local agents can only be used in the room where they were defined.

Agent configuration (like all other configuration) is stored in the Matrix Account Data of the bot user and is **potentially encrypted** (if enabled in the configuration), so that your configuration data is safe even on untrusted homeservers.


### Listing agents

To **list** all available agents: `!bai agent list`


#### Creating agents

See a [🖼️ Screenshot of the agent creation process](./screenshots/agent-creation.webp).

To **create** a new agent, you need to specify the [provider](./providers.md) and an agent id of your choosing.

- **Create** a new agent:
    - (Accessible in **this room only**) `!bai agent create-room-local PROVIDER_ID AGENT_ID`
    - (Accessible in **all rooms**) `!bai agent create-global PROVIDER AGENT_ID`
    - Example: `!bai agent create-room-local openai my-openai-agent`

The `AGENT_ID` is a unique identifier for the agent. It can be any string which **doesn't contain spaces and `/`**.

Depending on where the agent is defined (within a room, globally, or [statically](./configuration/README.md#static-configuration)), this id will get a prefix (e.g. `room-local/`, `global/` or `static/`). The combined id (prefix + agent id) makes the **full agent identifier** (refered to as `FULL_AGENT_IDENTIFIER` in commands below).

When creating an agent, you will be given some sample [YAML](https://en.wikipedia.org/wiki/YAML) configuration which you can use to customize the agent's behavior.

This configuration varies depending on the [☁️ provider](./providers.md) used and the capabilities of the agent. Based on the configuration keys you pass, certain features will be enabled or disabled. For example, if you skip the `image_generation` key for an [OpenAI](./providers.md#openai) agent, it won't be able to generate images (see [🖌️ Image Creation](./features.md#-image-creation), [🎨 Image Editing](./features.md#-image-editing), [🫵 Sticker Creation](./features.md#-sticker-creation)).

After making your modifications to the sample YAML, you submit it back to the bot and the new agent will be created.

**To make use of the agent**, you need to [🤝 configure it as a handler for a given purpose](./configuration/handlers.md).


### Showing agent details

To **show** full details for a given agent: `!bai agent details FULL_AGENT_IDENTIFIER`

Secrets in the configuration (such as the API key) are redacted. To **show the secrets too**: `!bai agent details-revealing-secrets FULL_AGENT_IDENTIFIER`. They are posted in the room where you run it, so consider who else can read that room.

Only administrators can show `static/` and `global/` agents. Room-local agents can be shown by [room-local agent managers](./access.md#-room-local-agent-managers).

This command requires a full agent identifier (e.g. `room-local/agent-id`).


### Deleting agents

To **delete** an agent: `!bai agent delete FULL_AGENT_IDENTIFIER`

This command requires a full agent identifier (e.g. `room-local/agent-id`).


### Updating agents

To **update** a given agent's configuration: show the agent's [details](#showing-agent-details) (current configuration), then [delete](#deleting-agents) it and finally [re-create](#creating-agents) it. The details redact secrets, so put the API key back in when re-creating it (or use `details-revealing-secrets`).
