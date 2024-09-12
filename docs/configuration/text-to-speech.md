
## 🗣️ Text-to-Speech

Below are some configuration settings related to Text-to-Speech.

You may also wish to see:

- [🌟 Features / 🗣️ Text-to-Speech](../features.md#-text-generation) for a higher-level introduction to the Text-to-Speech features
- [📖 Usage / 🗣️ Text-to-Speech](../usage.md#-text-generation) section for more details on how to use the bot for Text-to-Speech in a room


### 🪄 Bot Messages Flow Type

Controls how automatic text-to-speech functions for **messages sent by the bot**.

The following configuration values are recognized:

- (default) `on_demand_for_voice`: the bot will turn its own text messages into audio (voice) messages only after an allowed [👥 user](../access.md#-users) **reacts** to a bot's message with 🗣️. To make it easier for users to react without having to hunt for this emoji, the bot will automatically add a 🗣️ reaction to its own messages which are in response to a user audio (voice) message.

- `on_demand_always`: the bot will turn its own text messages into audio (voice) messages only after an allowed [👥 user](../access.md#-users) **reacts** to a bot's message with 🗣️. To make it easier for users to react without having to hunt for this emoji, the bot will automatically add a 🗣️ reaction to **all of its own messages**.

- `only_for_voice`: the bot will turn its own text messages into audio (voice) messages only if the original user message was a voice message. This is to allow for [Seamless voice interaction](../features.md#seamless-voice-interaction), where you can speak to the bot and then hear its responses

- `never`: the bot will never turn its own text messags into audio (voice) messages

- `always`: the bot will turn all its text messages into audio (voice) messages. This also allows for [Seamless voice interaction](../features.md#seamless-voice-interaction).

Example: `!bai config room text-to-speech set-bot-msgs-flow-type never` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))


### 🪄 User Messages Flow Type

Controls how automatic text-to-speech functions for **messages sent by [👥 users](../access.md#-users)**.

**Only works when automatic text-generation is disabled** (see [💬 Text Generation / 🪄 Auto Usage](./text-generation.md#-auto-usage)).

The following configuration values are recognized:

- (default) `never`: the bot will never turn [👥 user](../access.md#-users) text messages into audio (voice) messages

- `on_demand`: the bot will turn [👥 user](../access.md#-users) text messages into audio (voice) messages if the text message receives a 🗣️ reaction

- `always`: the bot will turn all [👥 user](../access.md#-users) text messages into audio (voice) messages. This is to allow for [Text-to-Speech-only mode](../features.md/#text-to-speech-only-mode).

Example: `!bai config room text-to-speech set-user-msgs-flow-type always` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))


### 🗲 Speed override

The speed override setting lets you speed up/down speech relative to the default speed configured at the [🤖 agent](../agents.md) level (usually `1.0`).

Values typically range from `0.25` to `4.0`, but may vary depending on the selected model.

Example: `!bai config room text-to-speech set-speed-override 1.5` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))


### 👫 Voice override

The voice override setting lets you change the voice being used by the text-to-speech model configured at the [🤖 agent](../agents.md) level (usually `onyx` when using [OpenAI](../providers.md#openai)).

Possible values (e.g. `onyx`) depend on the model you're using. For example, for [OpenAI](../providers.md#openai)'s Whisper model, [these voices](https://platform.openai.com/docs/guides/text-to-speech/voice-options) are available.

Example: `!bai config room text-to-speech set-voice-override nova` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))
