## 🔒 Access

This bot employs access control to decide who can use its services and manage its configuration.


### 👋 Joining rooms

The bot automatically joins rooms only when invited by someone considered a bot [👥 user](#-users).


### 👥 Users

The bot can be used by users that match some [dynamically](./configuration/README.md#dynamic-configuration) configured [Matrix user id](https://spec.matrix.org/v1.11/#users) patterns.

Users:

- ✅ can **invite the bot to rooms**
- ✅ can **use all the bot's [features](./features.md)** ([💬 Text Generation](./features.md#-text-generation), [🦻 Speech-to-Text](./features.md#-speech-to-text), etc.) by sending room messages
- ✅ can **mention the bot** in threads and reply chains to provoke it to respond to non-user messages (see [🌟 Features / 💬 Text Generation / On-demand involvement](./features.md#on-demand-involvement))
- ✅ can **change the bot's configuration in a room** (e.g. `!bai config room ...` commands), unless `room.config_changes_require_privilege` is enabled in the [static configuration](./configuration/README.md#static-configuration), in which case only [👮‍♂️ Administrators](#️-administrators) and [💼 Room-local agent managers](#-room-local-agent-managers) can
- ❌ cannot **change the bot's global configuration** (e.g. `!bai config global ...` commands)
- ❌ cannot **create new [🤖 Agents](./agents.md)** (neither in rooms, nor globally). See [💼 Room-local agent managers](#-room-local-agent-managers) for controlling which users can create agents.

The following commands are available:
- **Show** the currently allowed users: `!bai access users`
- **Set** the list of allowed users: `!bai access set-users SPACE_SEPARATED_PATTERNS`

Example patterns: `@*:example.com @*:another.com @someone:company.org`


### 👮‍♂️ Administrators

Administrators can **manage the bot's configuration and access control**.

Administrators are [👥 Users](#-users) and [💼 Room-local agent managers](#-room-local-agent-managers) implicitly, so they inherit all their permissions.

The bot can be administrated by users that match some [statically](./configuration/README.md#static-configuration) configured [Matrix user id](https://spec.matrix.org/v1.11/#users) patterns.

Administrators cannot be changed without adjusting the bot's configuration on the server.


### 💼 Room-local agent managers

Room-local agent managers are users privileged to **create their own [agents](./agents.md)** (see `!bai agent`) in rooms.

> [!WARNING]
> Letting regular users create agents which contact arbitrary network services **may be a security issue**.

The following commands are available:
- **Show** the currently allowed users: `!bai access room-local-agent-managers`
- **Set** the list of allowed users: `!bai access set-room-local-agent-managers SPACE_SEPARATED_PATTERNS`

Example patterns: `@*:example.com @*:another.com @someone:company.org`
