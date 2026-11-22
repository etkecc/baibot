## 🧰 Bot Tools

While answering, the model can ask the bot to do things for it, like reading the room's recent messages or creating an image. You just ask in chat, and the model decides when to use a tool.

Bot tools run in the bot, so they work with every supported provider (see [☁️ Providers](./providers.md)), as long as the model supports function calling (see [🤖 Agent settings](#-agent-settings)). They are separate from provider tools (like web search), which run on the provider's side (see [🌟 Features / 🛠️ Provider Tools](./features.md#️-provider-tools)).

| Tool | What it does |
| --- | --- |
| [📜 Room history](#-room-history) (`builtin.room.history`) | Reads the room's recent messages |
| [🖌️ Image creation](#️-image-creation) (`builtin.image.generate`) | Creates an image and posts it in the room |
| [🎨 Image editing](#-image-editing) (`builtin.image.edit`) | Edits an image and posts the result |


### 📜 Room history

> What do you think about what Alice said?

> Catch me up on the last 30 messages.

The bot reads the room's main timeline, also when you ask from a thread. Thread replies are skipped.

By default, it reads up to 10 messages before the one it's answering. You can ask it to read more, look further back, or read only some people's messages.

It includes everyone's messages, except from people explicitly denied `text-generation` by a deny rule (see [🔒 Access / 📜 Rules](./access.md#-rules)).


### 🖌️ Image creation

> Draw a steaming bowl of ramen.

The bot creates the image and posts it in the room. It's like the `!bai image create` command, but you can ask while chatting.

The room needs an agent for image creation (see [🌟 Features / 🖌️ Image Creation](./features.md#️-image-creation)). Your provider may charge for each image.


### 🎨 Image editing

> Make the bowl blue.

By default, the bot edits the most recent image in the thread or reply chain. You can also ask it to combine several images, if the room's image generation agent can (see [🌟 Features / 🎨 Image Editing](./features.md#-image-editing)).

The room's image generation agent needs to support image editing. Images posted by people explicitly denied `image-generation` (by a deny rule) can't be edited.


### 💡 Things to keep in mind

- Tools act on behalf of the person whose message the bot is answering, but everyone in the room sees the answer.
- Room history goes to the text-generation agent's provider. Image prompts and images to edit go to the image generation agent's provider.
- With the thinking notice on, it shows what the tools are doing, e.g. "📜 Reading up to 10 messages" (see [🛠️ Configuration / 💬 Text Generation / 💭 Thinking Notice](./configuration/text-generation.md#-thinking-notice)).
- Text from tools (like other people's messages) could steer the model. With provider tools that reach the internet (like web fetch or web scraping), that could leak room content to a website.


### 🔒 Tool permissions

Each tool is also a permission in the [🔒 Access](./access.md) rules: `tool:` followed by the tool's id.

| Tool permission | Comes with |
| --- | --- |
| 📜 `tool:builtin.room.history` | `text-generation` |
| 🖌️ `tool:builtin.image.generate` | `text-generation` + `image-generation` |
| 🎨 `tool:builtin.image.edit` | `text-generation` + `image-generation` |

Each built-in tool is allowed by default for people who have the feature permissions listed above. A deny rule turns the tool off. For example, to turn off room history in a room:

```
!bai access room deny tool:builtin.room.history @*:*
```

`tool:builtin.image.*` covers both image tools, and `tool:builtin.*` covers all built-in tools.

Only administrators can add or remove rules with tool permissions. These rules apply to administrators too.

`!bai access explain` shows your tool permissions in a room.


### 🤖 Agent settings

Bot tools are on by default for text-generation agents, but the model needs to support function calling. With OpenAI Compatible providers (like Ollama, see [☁️ Providers / OpenAI Compatible](./providers.md#openai-compatible)) and Venice (see [☁️ Providers / Venice](./providers.md#venice)), the bot retries without tools when the provider says the model doesn't support them.

To turn bot tools off for an agent, set `bot_tools: false` in its `text_generation` configuration (see [📄 Sample provider configurations](./sample-provider-configs/) and [🤖 Agents / Updating agents](./agents.md#updating-agents)).
