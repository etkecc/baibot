pub fn intro(command_prefix: &str) -> String {
    let message = r#"
## 📖 Usage

To get an **overview of the current configuration affecting this room**, send a `%command_prefix% config status` command.

To **adjust settings**, see `%command_prefix% config`.

### 💬 Text Generation

If there's a text-generation handler agent configured (see `%command_prefix% config status`), the bot **may** respond to messages sent in the room.

Whether the bot responds depends on the **💬 Text Generation / 🗟 Prefix Requirement** setting (see `%command_prefix% config status`).
Sometimes, a prefix (e.g. `%command_prefix%`) is required in front of messages sent to the room for the bot to respond.
For multi-user rooms, this setting defaults to "required"

Room messages start a threaded conversation where you can continue back-and-forth communication with the bot.

### 🗣️ Text-to-Speech

If there's a text-to-speech handler agent configured (see `%command_prefix% config status`), the bot **may** convert text messages sent to the room to audio (voice).

By default, the bot will offer text-to-speech for its own messages which are a response to voice messages coming from you. Simply click the 🗣️ reaction on the bot's message and the bot will convert the text message to audio. This is configurable via the **🗣️ Text-to-Speech / 🪄 Bot Messages Flow Type** setting.

The bot may be configured to also turn your own text messages to audio (voice) via the **🗣️ Text-to-Speech / 🪄 User Messages Flow Type** setting.


### 🦻 Speech-to-Text

If there's a speech-to-text handler agent configured (see `%command_prefix% config status`), the bot **may** transcribe voice messages sent to the room to text.

By default, the bot will also perform 💬 Text Generation on the text. This is configurable via the **🦻 Speech-to-Text / 🪄 Flow Type** setting.

If all your messages are in the same language, you can improve accuracy & latency by configuring the language via the **🦻 Speech-to-Text / 🔤 Language** setting.


### 🖌️ Image Generation

#### Generating images

Simply send a command like `%command_prefix% image A beautiful sunset over the ocean` and the bot will start a threaded conversation and post an image based on your prompt.

You can then, respond in the same message thread with:

- more messages, to add more criteria to your prompt.
- a message saying `again`, to generate one more image with the current prompt.

#### Generating stickers

A variation of **generating images** is to generate "sticker images".

To generate a sticker, send a command like `%command_prefix% sticker A huge bowl of steaming ramen with a mountain of beansprouts on top`.

The difference from **generating images** is that the bot will:

- generate a smaller-resolution image (`256x256`) - smaller/quicker, but still good enough for a sticker
- potentially switch to a different (cheaper or otherwise more suitable) model, if available
- post the image directly to the room (as a reply to your message), without starting a threaded conversation
"#;

    message.replace("%command_prefix%", command_prefix)
}
