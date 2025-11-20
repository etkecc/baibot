## ☁️ Providers

[🤖 Agents](./agents.md) are powered by a provider. The provider could be a **local service** or a **cloud service**.

The list of supported providers is below.


### Table of contents

- [How to choose a provider](#how-to-choose-a-provider)
- [How to use a provider](#how-to-use-a-provider)
- [Supported providers](#supported-providers)
  - [Anthropic](#anthropic)
  - [Groq](#groq)
  - [LocalAI](#localai)
  - [Mistral](#mistral)
  - [Ollama](#ollama)
  - [OpenAI](#openai)
  - [OpenAI Compatible](#openai-compatible)
  - [OpenRouter](#openrouter)
  - [Together AI](#together-ai)


### How to choose a provider

If you're not sure which provider to start with, **we recommend [OpenAI](#openai)** as it's the most popular and has the **widest range of capabilities**: [💬 text-generation](./features.md#-text-generation) (no vision), [🖌️ image-generation](./features.md#️image-generation), [🦻 speech-to-text](./features.md#-speech-to-text), [🗣️ text-to-speech](./features.md#️-text-to-speech).

You don't need to choose just one though. The bot supports [mixing & matching models](./features.md#-mixing--matching-models), so you can use multiple providers at the same time.


### How to use a provider

1. 📝 **Sign up for it**

2. 🔑 **Obtain an API key**

3. 🤖 **Create one or more agents** in a given room or globally. Next to each provider in the [list below](#supported-providers) you'll see **🗲 Quick start** commands, but you may also refer to the [agent creation guide](./agents.md#creating-agents).

4. 🤝 **Set the new agent as a handler** for a given use-purpose like text-generation, image-generation, etc. The agent creation wizard will tell you how, but you may also refer to the [🤝 Handlers](./configuration/handlers.md) guide.


### Supported providers

### Anthropic

[Anthropic](https://www.anthropic.com/) is an American AI company founded by former OpenAI engineers and providing powerful language models.

- 🆔 Identifier: `anthropic`
- 🔗 Links: [🏠 Home page](https://www.anthropic.com/), [🌐 Wiki](https://en.wikipedia.org/wiki/Anthropic), [👤 Sign up](https://console.anthropic.com/), [📋 Models list](https://docs.anthropic.com/en/docs/about-claude/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (incl. vision)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local anthropic my-anthropic-agent`
  - create a global agent: `!bai agent create-global anthropic my-anthropic-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/anthropic.yml).


### Groq

[Groq](https://groq.com/) is an American company developing optimized Language Processing Units (LPU) and offering cloud service which runs various models (built by others) with very high performance.

- 🆔 Identifier: `groq`
- 🔗 Links: [🏠 Home page](https://groq.com/), [🌐 Wiki](https://en.wikipedia.org/wiki/Groq), [👤 Sign up](https://console.groq.com/login), [📋 Models list](https://console.groq.com/docs/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local groq my-groq-agent`
  - create a global agent: `!bai agent create-global groq my-groq-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/groq.yml).


### LocalAI

[LocalAI](https://localai.io/) is the free, Open Source OpenAI alternative. LocalAI act as a drop-in replacement REST API that’s compatible with OpenAI API specifications for local inferencing. It allows you to run LLMs, generate images, audio (and not only) locally or on-prem with consumer grade hardware, supporting multiple model families and architectures.

- 🆔 Identifier: `localai`
- 🔗 Links: [🏠 Home page](https://localai.io/), [📋 Models list](https://localai.io/gallery.html)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local localai my-localai-agent`
  - create a global agent: `!bai agent create-global localai my-localai-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/localai.yml).


### Mistral

[Mistral AI](https://mistral.ai/) is a research lab based in Europe (France) which produces their own language models.

- 🆔 Identifier: `mistral`
- 🔗 Links: [🏠 Home page](https://mistral.ai/), [🌐 Wiki](https://en.wikipedia.org/wiki/Mistral_AI), [👤 Sign up](https://auth.mistral.ai/ui/registration), [📋 Models list](https://docs.mistral.ai/getting-started/models/)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local mistral my-mistral-agent`
  - create a global agent: `!bai agent create-global mistral my-mistral-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/mistral.yml).


### Ollama

[Ollama](https://ollama.com/) lets you run various models in a [self-hosted](https://github.com/ollama/ollama?tab=readme-ov-file#ollama) way. This is more advanced and requires powerful hardware for running some of the better models, but ensures your data stays with you.

- 🆔 Identifier: `ollama`
- 🔗 Links: [🏠 Home page](https://ollama.com/), [📋 Models list](https://ollama.com/library)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local ollama my-ollama-agent`
  - create a global agent: `!bai agent create-global ollama my-ollama-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/ollama.yml).


### OpenAI

[OpenAI](https://openai.com/) is an American AI company providing powerful language models.

Use this provider either with the OpenAI API or with other OpenAI-compatible API services which **fully** adhere to the [OpenAI API spec](https://github.com/openai/openai-openapi/).
For services which are not fully compatible with the OpenAI API, consider using the [OpenAI Compatible](#openai-compatible) provider.

- 🆔 Identifier: `openai`
- 🔗 Links: [🏠 Home page](https://openai.com/), [🌐 Wiki](https://en.wikipedia.org/wiki/OpenAI), [👤 Sign up](https://platform.openai.com/signup), [📋 Models list](https://platform.openai.com/docs/models)
- 🌟 Capabilities: [🖌️ image-generation](./features.md#️-image-creation), [💬 text-generation](./features.md#-text-generation) (incl. vision), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local openai my-openai-agent`
  - create a global agent: `!bai agent create-global openai my-openai-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/openai.yml).


### OpenAI Compatible

This provider allows you to use OpenAI-compatible API services like [OpenRouter](https://openrouter.ai/), [Together AI](https://www.together.ai/), etc.

Some of these popular services already have **shortcut** providers (leading to this one behind the scenes) - this make it easier to get started.

This provider is just as featureful as the [OpenAI](#openai) provider, but is more compatible with services which do not fully adhere to the [OpenAI API spec](https://github.com/openai/openai-openapi/).

- 🆔 Identifier: `openai-compatible`
- 🌟 Capabilities: [🖌️ image-generation](./features.md#️-image-creation), [💬 text-generation](./features.md#-text-generation) (no vision), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local openai-compatible my-openai-compatible-agent`
  - create a global agent: `!bai agent create-global openai-compatible my-openai-compatible-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/openai-compatible.yml).


### OpenRouter

[OpenRouter](https://openrouter.ai/) is a unified interface for LLMs. The platform scouts for the lowest prices and best latencies/throughputs across dozens of providers, and lets you choose how to [prioritize](https://openrouter.ai/docs/provider-routing) them.

- 🆔 Identifier: `openrouter`
- 🔗 Links: [🏠 Home page](https://openrouter.ai/), [👤 Sign up](https://openrouter.ai/), [📋 Models list](https://openrouter.ai/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local openrouter my-openrouter-agent`
  - create a global agent: `!bai agent create-global openrouter my-openrouter-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/openrouter.yml).


### Together AI

[Together AI](https://www.together.ai/) makes it easy to run or [fine-tune](https://docs.together.ai/docs/fine-tuning-overview) leading open source models with only a few lines of code.

- 🆔 Identifier: `together-ai`
- 🔗 Links: [🏠 Home page](https://www.together.ai/), [👤 Sign up](https://api.together.ai/signup), [📋 Models list](https://api.together.xyz/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local together-ai my-together-ai-agent`
  - create a global agent: `!bai agent create-global together-ai my-together-ai-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/together-ai.yml).
