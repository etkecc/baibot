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
  - [Venice](#venice)


### How to choose a provider

If you're not sure which provider to start with, **we recommend [Venice](#venice)**: it's the most capable provider baibot supports (covering [💬 text-generation](./features.md#-text-generation) with vision, file inputs, prompt caching, and native web search, plus [🖌️ image-generation](./features.md#️-image-creation) incl. editing, [🦻 speech-to-text](./features.md#-speech-to-text), and [🗣️ text-to-speech](./features.md#️-text-to-speech)) and the only one that runs inference with no logging and no training on your data. If you'd rather start with the most widely-used option, [OpenAI](#openai) is a solid, well-supported choice too.

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
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (incl. vision, no tools)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local anthropic my-anthropic-agent`
  - create a global agent: `!bai agent create-global anthropic my-anthropic-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/anthropic.yml).


### Groq

[Groq](https://groq.com/) is an American company developing optimized Language Processing Units (LPU) and offering cloud service which runs various models (built by others) with very high performance.

- 🆔 Identifier: `groq`
- 🔗 Links: [🏠 Home page](https://groq.com/), [🌐 Wiki](https://en.wikipedia.org/wiki/Groq), [👤 Sign up](https://console.groq.com/login), [📋 Models list](https://console.groq.com/docs/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision, no tools), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local groq my-groq-agent`
  - create a global agent: `!bai agent create-global groq my-groq-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/groq.yml).


### LocalAI

[LocalAI](https://localai.io/) is the free, Open Source OpenAI alternative. LocalAI act as a drop-in replacement REST API that’s compatible with OpenAI API specifications for local inferencing. It allows you to run LLMs, generate images, audio (and not only) locally or on-prem with consumer grade hardware, supporting multiple model families and architectures.

- 🆔 Identifier: `localai`
- 🔗 Links: [🏠 Home page](https://localai.io/), [📋 Models list](https://localai.io/gallery.html)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision, no tools), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local localai my-localai-agent`
  - create a global agent: `!bai agent create-global localai my-localai-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/localai.yml).


### Mistral

[Mistral AI](https://mistral.ai/) is a research lab based in Europe (France) which produces their own language models.

- 🆔 Identifier: `mistral`
- 🔗 Links: [🏠 Home page](https://mistral.ai/), [🌐 Wiki](https://en.wikipedia.org/wiki/Mistral_AI), [👤 Sign up](https://auth.mistral.ai/ui/registration), [📋 Models list](https://docs.mistral.ai/getting-started/models/)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision, no tools)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local mistral my-mistral-agent`
  - create a global agent: `!bai agent create-global mistral my-mistral-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/mistral.yml).


### Ollama

[Ollama](https://ollama.com/) lets you run various models in a [self-hosted](https://github.com/ollama/ollama?tab=readme-ov-file#ollama) way. This is more advanced and requires powerful hardware for running some of the better models, but ensures your data stays with you.

- 🆔 Identifier: `ollama`
- 🔗 Links: [🏠 Home page](https://ollama.com/), [📋 Models list](https://ollama.com/library)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision, no tools)
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
- 🌟 Capabilities: [🖌️ image-generation](./features.md#️-image-creation), [💬 text-generation](./features.md#-text-generation) (incl. vision, incl. [🛠️ tools](./features.md#️-built-in-tools-openai-only)), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local openai my-openai-agent`
  - create a global agent: `!bai agent create-global openai my-openai-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/openai.yml).


### OpenAI Compatible

This provider allows you to use OpenAI-compatible API services like [OpenRouter](https://openrouter.ai/), [Together AI](https://www.together.ai/), etc.

Some of these popular services already have **shortcut** providers (leading to this one behind the scenes) - this make it easier to get started.

This provider is just as featureful as the [OpenAI](#openai) provider, but is more compatible with services which do not fully adhere to the [OpenAI API spec](https://github.com/openai/openai-openapi/).

- 🆔 Identifier: `openai-compatible`
- 🌟 Capabilities: [🖌️ image-generation](./features.md#️-image-creation), [💬 text-generation](./features.md#-text-generation) (no vision, no tools), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local openai-compatible my-openai-compatible-agent`
  - create a global agent: `!bai agent create-global openai-compatible my-openai-compatible-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/openai-compatible.yml).


### OpenRouter

[OpenRouter](https://openrouter.ai/) is a unified interface for LLMs. The platform scouts for the lowest prices and best latencies/throughputs across dozens of providers, and lets you choose how to [prioritize](https://openrouter.ai/docs/provider-routing) them.

- 🆔 Identifier: `openrouter`
- 🔗 Links: [🏠 Home page](https://openrouter.ai/), [👤 Sign up](https://openrouter.ai/), [📋 Models list](https://openrouter.ai/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision, no tools)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local openrouter my-openrouter-agent`
  - create a global agent: `!bai agent create-global openrouter my-openrouter-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/openrouter.yml).


### Together AI

[Together AI](https://www.together.ai/) makes it easy to run or [fine-tune](https://docs.together.ai/docs/fine-tuning-overview) leading open source models with only a few lines of code.

- 🆔 Identifier: `together-ai`
- 🔗 Links: [🏠 Home page](https://www.together.ai/), [👤 Sign up](https://api.together.ai/signup), [📋 Models list](https://api.together.xyz/models)
- 🌟 Capabilities: [💬 text-generation](./features.md#-text-generation) (no vision, no tools)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local together-ai my-together-ai-agent`
  - create a global agent: `!bai agent create-global together-ai my-together-ai-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/together-ai.yml).


### Venice

[Venice AI](https://venice.ai/chat?ref=kpXDe6) _(ref link with a $10 bonus for you)_ runs inference on Venice-controlled GPUs or zero-data-retention partner infrastructure and stores no prompts or responses, so your conversations don't linger anywhere. It serves both frontier proprietary models and the latest open-source ones.

- 🆔 Identifier: `venice`
- 🔗 Links: [🏠 Home page](https://venice.ai/chat?ref=kpXDe6), [👤 Sign up](https://venice.ai/chat?ref=kpXDe6), [📋 Models list](https://docs.venice.ai/models/overview)
- 🌟 Capabilities: [🖌️ image-generation](./features.md#️-image-creation) (incl. editing, via the native knob-rich `/image/generate` and `/image/edit` endpoints), [💬 text-generation](./features.md#-text-generation) (incl. vision, file inputs like PDF and DOCX, and prompt caching; native web search via the `venice_parameters` config), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text)
- 🗲 Quick start:
  - create a room-local agent: `!bai agent create-room-local venice my-venice-agent`
  - create a global agent: `!bai agent create-global venice my-venice-agent`

💡 When creating an agent, the bot will show you an up-to-date sample configuration for this provider which looks [like this](./sample-provider-configs/venice.yml).

Unlike the [OpenAI Compatible](#openai-compatible) provider (which can talk to Venice but drops images and can't reach its audio or native image endpoints), this is a first-class Venice integration that exposes Venice's full parameter set. Image generation uses the native `/image/generate` endpoint rather than the OpenAI-compatible `/images/generations` shim, so every Venice-specific knob below is available.

#### Configuration reference

Every parameter below is optional unless marked otherwise. Omitting a knob lets Venice apply its own server-side default; this is **not** the same as setting it to `false`, which actively sends `false`.

**`text_generation`** (top-level knobs) — sampling, caching, and reasoning controls that sit directly on `text_generation`, next to `model_id`, `prompt`, `temperature`, `max_response_tokens`, and `max_context_tokens`. They map to top-level fields on Venice's request, separate from the `venice_parameters` bag below.

| Knob | What it does | Default |
|------|--------------|---------|
| `top_p` | Nucleus sampling, `0.0`–`1.0`. An alternative to `temperature`. | — |
| `frequency_penalty` | Penalize tokens by how often they have already appeared, `-2.0`–`2.0`. | — |
| `presence_penalty` | Penalize tokens that have appeared at all, `-2.0`–`2.0`. | — |
| `repetition_penalty` | Penalize repetition. Values above `1.0` discourage repeats. | — |
| `reasoning_effort` | Reasoning budget for models that support it: `low`, `medium`, `high`. | — |
| `prompt_cache_retention` | How long Venice keeps the prompt prefix cached: `default`, `extended`, or `24h`. `24h` is the lever that makes a long, stable system prompt cheap across a day of conversations. | `24h` |
| `show_reasoning` | Append the model's reasoning (its `reasoning_content`) below the answer, as a collapsible `💭 Reasoning` block that stays folded until clicked. Reads a field separate from the answer text, so it works regardless of `strip_thinking_response`. | `false` |

**`text_generation.venice_parameters`** — Venice-specific request knobs sent in the `venice_parameters` bag. Set any of them to override Venice's behavior. The `Default` column shows the value baibot's sample config ships; a `—` means the knob is left unset, so Venice's own default applies.

| Knob | What it does | Default |
|------|--------------|---------|
| `enable_web_search` | Web search mode: `auto` (model decides), `on` (always), or `off`. | `auto` |
| `enable_web_citations` | Append source citations to web-search answers. | — |
| `enable_web_scraping` | Allow the model to scrape page contents during web search. | — |
| `enable_x_search` | Include X (Twitter) in web search. | — |
| `include_search_results_in_stream` | Stream search results back as they arrive. | — |
| `return_search_results_as_documents` | Return search results as structured documents. | — |
| `include_venice_system_prompt` | Prepend Venice's own system prompt alongside yours. | — |
| `character_slug` | Use a public Venice character by its slug. | — |
| `strip_thinking_response` | Strip `<think></think>` blocks from reasoning models so the user sees only the answer. | `true` |
| `disable_thinking` | Disable the model's reasoning step entirely. | — |
| `enable_e2ee` | Run in end-to-end-encrypted mode rather than the default TEE-only mode. | `false` |
| `verbosity` | Response verbosity for models that support it: `low`, `medium`, `high`. | — |

**`text_to_speech`**:

| Knob | What it does | Default |
|------|--------------|---------|
| `model_id` | The Venice TTS model (e.g. `tts-kokoro`, `tts-qwen3-1-7b`, `tts-xai-v1`). | `tts-kokoro` |
| `voice` | The voice to synthesize with. Model-specific (Kokoro: `af_*`/`am_*`/`bf_*`/`bm_*`); a cloned-voice handle (`vv_<id>`) also works. | `af_sky` |
| `response_format` | Audio format: `mp3`, `opus`, `aac`, `flac`, `wav`, or `pcm`. | `mp3` |
| `speed` | Playback speed, `0.25`–`4.0`. | `1.0` |
| `prompt` | A style prompt steering emotion/delivery. Only Qwen 3 TTS honors it. | — |
| `temperature` | Sampling temperature, `0.0`–`2.0`. Only Qwen 3 / Orpheus / Chatterbox HD honor it. | — |
| `top_p` | Nucleus sampling, `0.0`–`1.0`. Only Qwen 3 TTS honors it. | — |

**`image_generation`**:

| Knob | What it does | Default |
|------|--------------|---------|
| `model_id` | The image-generation model. | `chroma` |
| `negative_prompt` | A description of what should **not** appear in the image. | — |
| `cfg_scale` | CFG scale, `0`–`20`. Higher values adhere more closely to the prompt. | — |
| `steps` | Number of inference steps. Model-specific; some models ignore it. | — |
| `style_preset` | A named style to apply (e.g. `3D Model`). | — |
| `seed` | Random seed, `-999999999`–`999999999`. Fix it for reproducible results. | random |
| `safe_mode` | Blur images classified as adult content. | `true` |
| `hide_watermark` | Hide the Venice watermark (may be ignored for some content). | `false` |
| `format` | Output format: `jpeg`, `png`, or `webp`. | `webp` |
| `width` / `height` | Image dimensions in pixels, each `1`–`1280`. | `1024` |
| `aspect_ratio` | Aspect ratio for models that support it (e.g. `1:1`, `16:9`). Alternative to `width`/`height`. | — |
| `resolution` | Resolution tier for models that support it (`1K`, `2K`, `4K`). | — |
| `quality` | Output quality for supported models: `low`, `medium`, `high`. Higher can cost more. | — |
| `lora_strength` | Lora strength, `0`–`100`. Only applies if the model uses additional Loras. | — |
| `embed_exif_metadata` | Embed the generation prompt into the image's EXIF metadata. | `false` |
| `enable_web_search` | Let the model pull the latest info from the web. Model-specific; costs extra credits. | — |

**`image_generation.edit`** — image editing reuses the `image_generation` block; only the model and a few output knobs differ:

| Knob | What it does | Default |
|------|--------------|---------|
| `model_id` | The image-edit model. | `firered-image-edit` |
| `output_format` | Output format: `jpeg`, `png`, or `webp`. When omitted, Venice infers it (PNG at 1K, JPEG at 2K/4K). | inferred |
| `aspect_ratio` | Aspect ratio of the result: `auto`, `1:1`, `3:2`, `16:9`, `21:9`, `9:16`, `2:3`, `3:4`, `4:5` (model-specific). | — |
| `resolution` | Resolution tier: `1K`, `2K`, `4K` (model-specific). | `1K` |
| `safe_mode` | Blur images classified as adult content. | `true` |
