
## 💬 Text Generation

Below are some [🛠️ dynamic configuration settings](./README.md#dynamic-configuration) related to Text Generation.

You may also wish to see:

- [🌟 Features / 💬 Text Generation](../features.md#-text-generation) for a higher-level introduction to the Text Generation features
- [📖 Usage / 💬 Text Generation](../usage.md#-text-generation) section for more details on how to use the bot for Text Generation in a room


### 🗟 Prefix Requirement Type

In Direct Message rooms with the bot (1:1 rooms), it most usually makes sense for the bot to respond to **all** of your messages, as shown on this [🖼️ screenshot](../screenshots/text-generation.webp).

In group rooms (with multiple users), it may be more appropriate for the bot to only respond to messages that are **prefixed** with the command prefix (e.g. `!bai`) or which are [mentioning](https://spec.matrix.org/latest/client-server-api/#user-and-room-mentions) the bot (e.g. `@baibot`), so that other chat exchange in the room will not trigger it. Such a setup is shown on the [🖼️ On-demand involvement in the room](../screenshots/text-generation-prefix-requirement.webp) screenshot.

There are exceptions to these rules, and you can configure the bot to respond only to prefixed messages in a 1:1 room, or to respond to all messages even in a multi-user group room.

To support such use-cases, the bot has a `text-generation prefix-requirement-type` setting, which can be set to:

- (default) `no`: indicates that the bot would not require a prefix and would respond to all messages

- `command_prefix`: indicates that the bot would require that messages be prefixed with the command prefix (e.g. `!bai`) and would ignore all messages that are not prefixed

By default, the bot is **auto-configured (upon joining a new room)** to use the `no` setting in rooms that only include 2 users (you and the bot), and `command_prefix` in rooms with more than 2 users. To prevent surprises, the bot will **not** adjust this setting subsequently. You can manually adjust it via `!bai config room text-generation set-prefix-requirement-type VALUE`.

Example: `!bai config room text-generation set-prefix-requirement-type command_prefix` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))

Regardless of this configuration, **the bot will also respond to messages by allowed [👥 Users](../access.md#-users) which directly [mention](https://spec.matrix.org/latest/client-server-api/#user-and-room-mentions) the bot** (e.g. `@baibot`), even if they are not prefixed. An example of this can be seen on these screenshots:

- [🖼️ On-demand involvement in a thread](../screenshots/text-generation-on-demand-thread-involvement.webp)
- [🖼️ On-demand involvement in a reply chain](../screenshots/text-generation-on-demand-reply-involvement.webp)


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


### 👤 Sender Context Mode

In multi-user rooms, it may be useful for the model to know which participant sent each message in the conversation context.

To support this, the bot has a `text-generation sender-context-mode` setting, which can be set to:

- (default) `none`: do not attach sender metadata to messages before sending them to the model

- `matrix_user_id`: prefix text messages with the sender's Matrix user ID, for example: `[sender=@alice:example.com] Hello bot`

- `matrix_user_id_and_timestamp`: prefix text messages with the sender's Matrix user ID and the message timestamp, for example: `[sender=@alice:example.com sent_at=2026-03-23T14:30:00Z] Hello bot`

This sender metadata is attached to conversation messages before they are sent to the model provider. It applies to user and assistant text messages, but not to system prompts or non-text content.

⚠️ Enabling this sends Matrix user IDs, and optionally timestamps, to the model provider.

Example: `!bai config room text-generation set-sender-context-mode matrix_user_id` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))


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
| `{{ baibot_now_utc }}`    | Current date and time in UTC (⚠️ usage may break prompt caching - see below) | `2024-09-20 (Friday), 14:26:42 UTC` |
| `{{ baibot_conversation_start_time_utc }}` | The date and time in UTC that the conversation started | `2024-09-20 (Friday), 14:26:42 UTC` |

💡 `{{ baibot_now_utc }}` changes as time goes on, which prevents [prompt caching](https://platform.openai.com/docs/guides/prompt-caching) from working. It's better to use `{{ baibot_conversation_start_time_utc }}` in prompts, as its value doesn't change yet still orients the bot to the current date/time.

Here's a prompt that combines some of the above variables:

> You are a brief, but helpful bot called {{ baibot_name }} powered by the {{ baibot_model_id }} model. The date/time of this conversation's start is: {{ baibot_conversation_start_time_utc }}."


### 🌡️ Temperature Override

You can override the [temperature](https://blogs.novita.ai/what-are-large-language-model-settings-temperature-top-p-and-max-tokens/#what-is-llm-temperature) (randomness / creativity) parameter configured at the [🤖 agent](../agents.md) level.

Example: `!bai config room text-generation set-temperature-override 3.5` (this can also be set globally, see [🛠️ Room Settings](./README.md#room-settings))
