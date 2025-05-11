## 📖 Usage

This document covers how to use the bot in a room.

The [🌟 Features](./features.md) page also includes details about how each feature works and can be configured.


### 💬 Text Generation

This is related to the [💬 Text Generation](./features.md#-text-generation) feature.

If there's a text-generation handler agent configured, the bot **may** respond to messages sent in the room.

Some models also support vision, so you may be able to mix text and images in the same conversation.

See screenshots of:

- 🖼️ [the default Text Generation flow](./screenshots/text-generation.webp) in 1:1 rooms
- 🖼️ [the Text Generation flow in multi-user rooms](./screenshots/text-generation-prefix-requirement.webp) (where the [🗟 Prefix Requirement](./configuration/text-generation.md#-prefix-requirement-type) setting is auto-configured to "required")
- the [on-demand involvement](./features.md#on-demand-involvement) feature

Whether the bot responds depends on:

- ([🔒 access](./access.md)) whether you're a whitelisted bot [👥 user](./access.md#-users)

- [🛠️ configuration](./configuration/README.md) whether there's a configured `text-generation` handler agent (or a `catch-all` handler agent). See [Mixing & matching models](./features.md#-mixing--matching-models)

- (🎨 agent capabilities) whether the configured `text-generation` (or `catch-all`) handler agent actually supports text-generation. The provider may lack support for this feature or it may be disabled in the [🤖 agents](./agents.md) configuration

- (the [🗟 Prefix Requirement](./configuration/text-generation.md#-prefix-requirement-type) setting) whether a prefix (e.g. `!bai`) or user mention (e.g. `@baibot`) is required for messages sent to the room. For multi-user rooms, this setting defaults to "required". See [🌟 Features / 💬 Text Generation / On-demand involvement](./features.md#on-demand-involvement) for details.

Room messages start a threaded conversation where you can continue back-and-forth communication with the bot. Using [on-demand involvement](./features.md#on-demand-involvement), you can can also mention the bot to provoke it to get involved in any conversation thread or reply chain.

Unless you've enabled the [♻️ Context Management](./features.md#️-context-management) feature, all messages will be sent to the agent's API each time. If the context management feature is enabled, older messages may be dropped.


### 🗣️ Text-to-Speech

This is related to the [🗣️ Text-to-Speech](./features.md#️-text-to-speech) feature.

If there's a text-to-speech handler agent configured, the bot **may** convert text messages sent to the room to audio (voice).

See:

- a [🖼️ screenshot](./screenshots/text-to-speech-only-mode.webp) of the bot's [Text-to-Speech-only](./features.md#text-to-speech-only-mode) mode

- a [🖼️ screenshot](./screenshots/text-to-speech-seamless-voice-interaction.webp) of the bot's [Seamless voice interaction](./features.md#seamless-voice-interaction) mode

By default, the bot:

- will offer tex-to-speech for its own messages which are a response to voice message from your, as part of the [Seamless voice interaction](./features.md#seamless-voice-interaction) feature. This can be adjusted via the [🗣️ Text-to-Speech / 🪄 Bot Messages Flow Type](./configuration/text-to-speech.md#-bot-messages-flow-type) setting.

- does not turn your own text messages to audio (voice). If you'd like for the bot to operate in such a mode, use the [🗣️ Text-to-Speech / 🪄 User Messages Flow Type](./configuration/text-to-speech.md#-user-messages-flow-type) setting (see [Text-to-Speech-only mode](./features.md#text-to-speech-only-mode)).


### 🦻 Speech-to-Text

This is related to the [🦻 Speech-to-Text](./features.md#-speech-to-text) feature.

If there's a speech-to-text handler agent configured, the bot **may** transcribe voice messages sent to the room to text.

See a [🖼️ Screenshot of the default flow for Speech-to-Text and Text-Generation](./screenshots/speech-to-text-default-flow.webp).

The speech-to-text feature triggers automatically by default, but can be adjusted via the [🦻 Speech-to-Text / 🪄 Flow Type](./features.md#-speech-to-text-flow-type) setting.

If all your messages are in the same language, you can improve accuracy & latency by configuring the language (see [🦻 Speech-to-Text / 🔤 Language](./configuration/speech-to-text.md#-language)).


### Image Generation

This feature is not configurable at the moment. The configuration (size, quality, style) specified at the [🤖 agent](./agents.md) level will be used.

Capabilities depend on the [☁️ provider](./providers.md) and model used.


#### 🖌️ Creating images

Simply send a command like `!bai image create A beautiful sunset over the ocean` and the bot will start a threaded conversation and post an image based on your prompt.

See a [🖼️ Screenshot of the Image Creation feature](./screenshots/image-creation.webp).

You can then respond in the same message thread with:

- more messages, to add more criteria to your prompt.
- a message saying `again`, to generate one more image with the current prompt.


#### 🎨 Editing images

Simply send a command like `!bai image edit Turn the following image into an anime-style drawing` and the bot will start a threaded conversation asking for more details.

See a [🖼️ Screenshot of the Image Editing feature (manipulating a single image)](./screenshots/image-editing-single-image.webp) and a [🖼️ Screenshot of the Image Editing feature (manipulating multiple images)](./screenshots/image-editing-multiple-images.webp).

You can then respond in the same message thread with:

- more messages, to add more criteria to your prompt.
- one or more images, to provide the images that the bot will operate on.
- a message saying `go`, to start the image generation process.
- a message saying `again`, to prompt the bot to generate one more image edit with the current prompt.


#### 🫵 Creating stickers

A variation of [creating images](#creating-images) is creating "sticker images".

See a [🖼️ Screenshot of the Sticker Creation feature](./screenshots/sticker-generation.webp).

To create a sticker, send a command like `!bai sticker A huge ramen bowl with lots of chashu and a mountain of beansprouts on top`.

The difference from [creating images](#creating-images) is that the bot will:

- generate a smaller-resolution image (currently hardcoded to `256x256`) - smaller/quicker, but still good enough for a sticker
- potentially switch to a different (cheaper or otherwise more suitable) model, if available
- post the image directly to the room (as a reply to your message), without starting a threaded conversation

Some models (like [OpenAI](./providers.md#openai)'s [Dall-E-3](https://openai.com/index/dall-e-3/)) can only generate larger images (`1024x1024`, etc., for a higher charge), so we switching to a smaller/cheaper model (like [Dall-E-2](https://openai.com/index/dall-e-2/)) is a way to generate a sticker cheaply.
