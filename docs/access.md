## 🔒 Access

This bot uses access rules to decide who can use its features and manage its configuration, globally and in each room.

Send `!bai access` to see this help in chat, and `!bai access explain` to see your own permissions in a room.


### 🔑 Permissions

| Permission | Allows |
| --- | --- |
| 💬 `text-generation` | Chatting with the bot ([💬 Text Generation](./features.md#-text-generation)), including sending it images and files |
| 🦻 `speech-to-text` | Having voice messages transcribed ([🦻 Speech-to-Text](./features.md#-speech-to-text)) |
| 🗣️ `text-to-speech` | Having messages read aloud ([🗣️ Text-to-Speech](./features.md#-text-to-speech)), including via the 🗣️ reaction |
| 🖌️ `image-generation` | Generating and editing images, and generating stickers ([Image Generation](./features.md#image-generation)) |
| 🌟 `use` | All of the above. Features added to the bot later may also be added to `use` (release notes say so) |
| 🛠️ `manage-config` | Viewing and changing the room's settings (`!bai config room ...`, `!bai config status`) |
| 🤖 `manage-agents` | Creating, inspecting and deleting [🤖 agents](./agents.md) in the room |
| 🔒 `manage-access` | Viewing and changing the room's access rules |
| 👋 `invite` | Inviting the bot into rooms. Only global rules can grant it |
| 🧰 `tool:ID` | Letting the bot use a bot tool while chatting. See [🧰 Bot Tools / 🔒 Tool permissions](./bot-tools.md#-tool-permissions) |

Anyone with any permission in a room (other than `invite` and tools) can talk to the bot there, see help and list agents. Messages from everyone else are ignored.

The bot's voice flow (transcribe, reply, read the reply aloud) checks each step separately. For example, someone with only `speech-to-text` gets transcriptions, but no replies.

When the bot builds conversation context, it includes messages from people who may use the feature in that room. When someone mentions the bot in a thread or a reply chain, it includes everyone's messages there, except from people denied the feature. An image thread started by someone without `image-generation` cannot be continued.

> [!WARNING]
> Letting regular users create agents (`manage-agents`) which contact arbitrary network services **may be a security issue**.


### 📜 Rules

Each rule **allows** or **denies** some permissions to users matching [Matrix user ID](https://spec.matrix.org/v1.11/#users) patterns (like `@*:example.com` or `@someone:example.com`). A rule can also require a minimum [power level](https://spec.matrix.org/v1.11/client-server-api/#mroompower_levels) in the room (for example, `50` for moderators).

Rules are either:

- **global**: they apply in every room. Only administrators can change them.
- **room rules**: they apply in one room.

A user has a permission in a room if some rule (global or of that room) allows it and **no rule denies it**. Built-in bot tools need no allow rule: they come with the features they use (see [🧰 Bot Tools / 🔒 Tool permissions](./bot-tools.md#-tool-permissions)). Rules have no order: a deny always wins, and a room rule cannot undo a global deny. To make exceptions, leave people out of broad allow rules, rather than denying broadly and allowing again.

Each rule has a short identifier (like `k3vq`), used to remove it.

Commands:

- **Explain** your permissions in this room: `!bai access explain` (with `manage-access`, also another user's: `!bai access explain @someone:example.com`)
- **List** the global rules and this room's rules: `!bai access list` (requires `manage-access`)
- **Add** a room rule: `!bai access room allow|deny PERMISSIONS USER_PATTERNS [min-power-level=N]` (requires `manage-access`)
- **Remove** a room rule: `!bai access room remove RULE_ID` (requires `manage-access`)
- **Add** or **remove** a global rule: the same, with `global` instead of `room` (administrators only)

Separate several permissions or user patterns with commas or spaces.

Examples:

- Let bridged contacts have voice messages transcribed in this room: `!bai access room allow speech-to-text @*:bridge.example.com`
- Let the room's moderators change its settings: `!bai access room allow manage-config @*:example.com min-power-level=50`
- Keep someone from generating images in this room: `!bai access room deny image-generation @someone:example.com`

People with `manage-access` who are not administrators can only add or remove room rules made of feature permissions (`use`, `text-generation`, `speech-to-text`, `text-to-speech`, `image-generation`) which they have themselves. Rules with other permissions (including tools) are managed by administrators.


### 🌱 Initial rules

On its first run, the bot creates global rules from the `initial_global_config.access_rules` [static configuration](./configuration/README.md#static-configuration) setting. Each rule has the same parts as a rule added with a command: an `effect` (`allow` or `deny`), `permissions`, `users` (MXID patterns) and an optional `min_power_level`. The bot gives each rule a random id.

The [sample configuration](../etc/app/config.yml.dist) lets everyone on the homeserver:

- **use** the bot's features and **invite** it into rooms
- **manage the configuration** of rooms where their power level is at least 50, so room moderators and admins

```yaml
initial_global_config:
  access_rules:
    - effect: allow
      permissions: [use, invite]
      users: ["@*:example.com"]
    - effect: allow
      permissions: [manage-config]
      users: ["@*:example.com"]
      min_power_level: 50
```

Without initial rules, only administrators (see below) can use the bot until they add rules.

The bot refuses to start if a rule is invalid (an unknown permission or key, a malformed tool permission, no users, or `invite` with `min_power_level`). Unknown tools only cause a warning in the log.

After the first run, rules are only managed with the commands above. Changing `initial_global_config.access_rules` later has no effect.

The `initial_global_config.user_patterns` setting (and its `BAIBOT_INITIAL_GLOBAL_CONFIG_USER_PATTERNS` environment variable) used by earlier versions is no longer supported, and the bot refuses to start while it is set. Replace it with `access_rules` as shown above. Rules have no environment variable.


### ⬆️ Upgrading from user lists

Earlier versions used 2 lists instead of rules: allowed users and room-local agent managers. The bot turns them into global rules automatically:

| Before | After |
| --- | --- |
| Allowed users | A rule allowing `use,invite`, and a rule allowing `manage-config` with a power level of at least 50 |
| Room-local agent managers | A rule allowing `manage-agents` |

What changes for existing deployments:

- Allowed users **can no longer change room settings, or see `!bai config status`, in rooms where their power level is below 50**. To restore the old behavior: `!bai access global allow manage-config SAME_PATTERNS_AS_BEFORE`.
- Room-local agent managers who were not also allowed users could not do anything before (they could not reach the bot). They can now manage agents. To prevent this, remove that rule (see `!bai access list`) and add a narrower one.
- The `!bai access users`, `set-users`, `room-local-agent-managers` and `set-room-local-agent-managers` commands are replaced by rules.

The old lists are removed once turned into rules. After going back to an older version, only administrators have access until they set the allowed users again (`!bai access set-users SPACE_SEPARATED_PATTERNS`). Upgrading again turns that list into rules once more.


### 👋 Joining rooms

The bot automatically joins rooms when invited by someone with the `invite` permission.


### 👮‍♂️ Administrators

Administrators can **do anything**, in any room, regardless of rules (but tool rules apply to them too, see [🧰 Bot Tools / 🔒 Tool permissions](./bot-tools.md#-tool-permissions)). Only they can manage the global configuration, global agents and global rules.

The bot can be administrated by users that match some [statically](./configuration/README.md#static-configuration) configured [Matrix user ID](https://spec.matrix.org/v1.11/#users) patterns.

Administrators cannot be changed without adjusting the bot's configuration on the server.
