
## 💬 Text Generation

Below are some [🛠️ dynamic configuration settings](./README.md#dynamic-configuration) related to Text Generation.

You may also wish to see:

- [🌟 Features / 💬 Text Generation](../features.md#-text-generation) for a higher-level introduction to the Text Generation features
- [📖 Usage / 💬 Text Generation](../usage.md#-text-generation) section for more details on how to use the bot for Text Generation in a room


### 🗟 Prefix Requirement Type

In Direct Message rooms with the bot (1:1 rooms), it most usually makes sense for the bot to respond to **all** of your messages, as shown on this [🖼️ screenshot](../screenshots/text-generation.webp).

In group rooms (with multiple users), it may be more appropriate for the bot to only respond to messages that are **prefixed** with the command prefix (e.g. `!bai`), so that other chat exchange in the room will not trigger it. Such a setup is shown on this [🖼️ screenshot](../screenshots/text-generation-prefix-requirement.webp).

There are exceptions to these rules, and you can configure the bot to respond only to prefixed messages in a 1:1 room, or to respond to all messages even in a multi-user group room.

To support such use-cases, the bot has a `text-generation prefix-requirement-type` setting, which can be set to:

- (default) `no`: indicates that the bot would not require a prefix and would respond to all messages

- `command_prefix`: indicates that the bot would require that messages be prefixed with the command prefix (e.g. `!bai`) and would ignore all messages that are not prefixed

By default, the bot is **auto-configured (upon joining a new room)** to use the `no` setting in rooms that only include 2 users (you and the bot), and `command_prefix` in rooms with more than 2 users. To prevent surprises, the bot will **not** adjust this setting subsequently. You can manually adjust it via `!bai config room text-generation set-prefix-requirement-type VALUE`.

Example: `!bai config room text-generation set-prefix-requirement-type command_prefix` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))

Regardless of this configuration, **the bot will also respond to messages which directly [mention](https://spec.matrix.org/latest/client-server-api/#user-and-room-mentions) the bot** (e.g. `@baibot`), even if they are not prefixed. An example of this can be seen on this [🖼️ screenshot](../screenshots/text-generation-prefix-requirement.webp).


### 🪄 Auto Usage

Text generation is enabled by default (the `text-generation auto-usage` setting being set to `always`), but can be set to:

- (default) `always`: generate text for all messages (also see [🗟 Prefix Requirement Type](#-prefix-requirement-type))

- `never`: never generate text for messages

- `only_for_voice`: only generate text when the original user message was a voice message, later transcribed via [🦻 Speech-to-Text](../features.md#-speech-to-text)

- `only_for_text`: only generate text when original user message was a text message

Example: `!bai config room text-generation set-auto-usage only_for_voice` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))


### ♻️ Context Management

The bot also supports ♻️ **context management**, which automatically adjusts the message history length, etc.

This feature relies on [tokenization](https://en.wikipedia.org/wiki/Large_language_model#Tokenization) performed by the [tiktoken-rs](https://github.com/zurawiki/tiktoken-rs) library which is [poorly well-maintained](https://github.com/zurawiki/tiktoken-rs/issues/50) and only works well for [OpenAI](../providers.md#openai) models.

This setting is **disabled by default**, but can be enabled via `!bai config room text-generation set-context-management-enabled true` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings)).


### ⌨️ Prompt Override

You can override the [system prompt](https://huggingface.co/docs/transformers/en/tasks/prompting) configured at the [🤖 agent](../agents.md) level.

Example (multi-line is supported):

```
!bai config room text-generation set-prompt-override You're a UI/UX expert. Everything you say needs to consider design and usability.

Where appropriate, you'll mention best practices and common pitfalls.
```

A prompt override can also be set globally, see [🛠️ Room Settings](./README.md#room-settings).

Prompts may contain the following **placeholder variables** which will be replaced *every time* the bot is interacted with:

| Placeholder               | Description | Example |
|---------------------------|-------------|---------|
| `{{ baibot_name }}`       | Name of the bot as configured in the `user.name` field in the [Static configuration](./README.md#static-configuration) | `Baibot` |
| `{{ baibot_model_id }}`   | Text-Generation model ID as configured in the [🤖 agent](../agents.md)'s configuration | `gpt-4o` |
| `{{ baibot_now_utc }}`    | Current date and time in UTC | `2024-09-20 (Friday), 14:26:42 UTC` |

Here's a prompt that combines some of the above variables:

> You are a brief, but helpful bot called {{ baibot_name }} powered by the {{ baibot_model_id }} model. The date/time now is: {{ baibot_now_utc }}."


### 🌡️ Temperature Override

You can override the [temperature](https://blogs.novita.ai/what-are-large-language-model-settings-temperature-top-p-and-max-tokens/#what-is-llm-temperature) (randomness / creativity) parameter configured at the [🤖 agent](../agents.md) level.

Example: `!bai config room text-generation set-temperature-override 3.5` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))
