## 🔒 Access

This bot employs access control to decide who can use its services and manage its configuration.


### 👋 Joining rooms

The bot automatically joins rooms when invited by someone considered a bot [user](#-users).


### 👥 Users

The bot will ignore messages (and room invitations) from unallowed users.

Users can **use all the bot's [features](./features.md)** ([💬 Text Generation](./features.md#-text-generation), [🦻 Speech-to-Text](./features.md#-speech-to-text), etc.), but **cannot manage the bot's configuration**.

The bot can be used by users that match some [dynamically](./configuration/README.md#dynamic-configuration) configured [Matrix user id](https://spec.matrix.org/v1.11/#users) patterns.

The following commands are available:
- **Show** the currently allowed users: `!bai access users`
- **Set** the list of allowed users: `!bai access set-users SPACE_SEPARATED_PATTERNS`

Example patterns: `@*:example.com @*:another.com @someone:company.org`


### 👮‍♂️ Administrators

Administrators can **manage the bot's configuration and access control**.

The bot can be administrated by users that match some [statically](./configuration/README.md#static-configuration) configured [Matrix user id](https://spec.matrix.org/v1.11/#users) patterns.

Administrators cannot be changed without adjusting the bot's configuration on the server.


### 💼 Room-local agent managers

Room-local agent managers are users privileged to **create their own [agents](./agents.md)** (see `!bai agent`) in rooms.
Letting regular users create agents which contact arbitrary network services **may be a security issue**.

No room-local agent manager patterns are configured, so new agents can only be created by administrators.

The following commands are available:
- **Show** the currently allowed users: `!bai access room-local-agent-managers`
- **Set** the list of allowed users: `!bai access set-room-local-agent-managers SPACE_SEPARATED_PATTERNS`

Example patterns: `@*:synapse.127.0.0.1.nip.io @*:another.com @someone:company.org`
