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
- ✅ can **change the bot's configuration in a room** (e.g. `!bai config room ...` commands)
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

Administrators can also **manage other rooms** from any room (for example, a private room with the bot), using `config` and `agent` commands. Replies are sent to the room the command was sent in.

- **List** the rooms the bot is in: `!bai room list`
- **Run** a command in another room: `!bai in ROOM_ID_OR_ALIAS COMMAND`, for example `!bai in #room:example.com config status` or `!bai in !abc123:example.com agent list`


### 💼 Room-local agent managers

Room-local agent managers are users privileged to **create their own [agents](./agents.md)** (see `!bai agent`) in rooms.

> [!WARNING]
> Letting regular users create agents which contact arbitrary network services **may be a security issue**.

The following commands are available:
- **Show** the currently allowed users: `!bai access room-local-agent-managers`
- **Set** the list of allowed users: `!bai access set-room-local-agent-managers SPACE_SEPARATED_PATTERNS`

Example patterns: `@*:example.com @*:another.com @someone:company.org`
