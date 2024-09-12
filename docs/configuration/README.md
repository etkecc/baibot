## 🛠️ Configuration

The bot's behavior is controlled by a combination of [static](#static-configuration) and [dynamic](#dynamic-configuration) configuration.


### Static configuration

The bot can be configured using a [YAML](https://en.wikipedia.org/wiki/YAML) configuration file as well as [environment variables](https://en.wikipedia.org/wiki/Environment_variable).

When running the bot locally (during [🧑‍💻 development](../development.md)), the bot's configuration is read from the `var/app/config.yml` file.
This file is created from the template found in [etc/app/config.yml.dist](../../etc/app/config.yml.dist).

Certain keys can be left unset, in which case [📝 hardcoded defaults](../../src/entity/cfg/defaults.rs) would be used.

Each configuration key found in the YAML configuration can be overridden by setting an environment variable (dots should be replaced with `_`). Example:

- to override `command_prefix`, set an environment variable `BAIBOT_COMMAND_PREFIX`
- to override `homeserver.server_name`, set an environment variable `BAIBOT_HOMESERVER_SERVER_NAME`

The static configuration contains an `initial_global_config` key, which is used to populate the bot's global configuration (stored as [dynamic configuration](#dynamic-configuration)) the first time the bot starts. Modifying this subsequently will not have any effect. After initial global configuration creation, it's expected to be managed dynamically via chat commands.


### Dynamic configuration

Besides the bot's [static configuration](#static-configuration), **the bot can also be configured dynamically at runtime (via chat messages)**.

This includes changes to [🔒 Access](../access.md), [🤖 Agents](../agents.md) and [🛠️ Room Settings](#room-settings).


#### Room Settings

Room Settings come from 3 different levels with priority in the following order (higher to lower):

- 📍 per-room (`!bai config room ..` commands)
- 🌐 globally (`!bai config global ..` commands)
- 📝 as [hardcoded defaults](../../src/entity/cfg/defaults.rs)

You can adjust the following settings per room and/or globally:

- [💬 Text Generation](text-generation.md)
- [🦻 Speech-to-Text](speech-to-text.md)
- [🗣️ Text-to-Speech](text-to-speech.md)
- [🖌️ Image Generation](image-generation.md)
- [🤝 Handlers](handlers.md)

Refer to the bot's help messages (as a response to a `!bai config` help command) for the most up-to-date information on what Room Settings can be configured.

You can **get an overview of the configuration affecting the current room** (a mix of hardcoded defaults, agent defaults, global and room-level settings) by sending a `!bai config status` command to the room.
