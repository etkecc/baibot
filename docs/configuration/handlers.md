## 🤝 Handlers

### Introduction

You can use **different models in different rooms** (e.g. [OpenAI](../providers.md#openai) GPT-4o alongside [Llama](https://en.wikipedia.org/wiki/Llama_(language_model)) running on [Groq](../providers.md#groq), etc.)

You can also use **different models within the same room** (e.g. [💬 text-generation](#-text-generation) handled by one [agent](./agents.md), [🦻 speech-to-text](#-speech-to-text) handled by another, [🗣️ text-to-speech](#️-text-to-speech) by a 3rd, etc.)

The bot supports the following use-purposes:

- [💬 text-generation](../features.md#-text-generation): communicating with you via text (though certain models may "see" images as well)
- [🦻 speech-to-text](../features.md#-speech-to-text): turning your voice messages into text
- [🗣️ text-to-speech](../features.md#️-text-to-speech): turning bot or users text messages into voice messages
- [🖌️ image-generation](../features.md#image-generation): generating images based on instructions

In a given room, each different purpose can be served by a different [provider](../providers.md) and model. This combination of provider and model configuration is called an [🤖 agent](../agents.md). Each purpose can be served by a different **handler** agent.

See a [🖼️ Screenshot of an example room configuration](./screenshots/config-status-handlers.webp).


### Configuring

Handlers can be configured [dynamically](./README.md#dynamic-configuration):

- either per-room (e.g. `!bai config room set-handler text-generation room-local/openai-gpt-4o`)
- or globally (e.g. `!bai config global set-handler text-generation global/openai-gpt-4o`)

The per-room configuration takes priority over the global configuration.

There's also a `catch-all` purpose that can be used as a fallback handler for messages that don't match any other handler.

💡 It's a good idea to globally-configure a powerful agent as a catch-all handler, so that the bot can always handle messages of any kind. You can then override individual handlers per room or globally.
