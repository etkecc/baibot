## 🧑‍💻 Development

This documentation page contains information about **running the bot locally for development purposes**.
This can also **helpful for quickly testing the bot in a containerized environment, with all dependency services included**.

For running the bot against your Matrix server, see the [🚀 Installation](./installation.md) documentation.

This bot is built in [🦀 Rust](https://www.rust-lang.org/) and uses the [mxlink](https://github.com/etkecc/rust-mxlink) library (built on top of [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk)).

For local development, we run all dependency services in [🐋 Docker](https://www.docker.com/) containers via [docker-compose](https://docs.docker.com/compose/).


### Prerequisites

- [🐋 Docker](https://www.docker.com/) and [docker-compose](https://docs.docker.com/compose/)
- [Just](https://github.com/casey/just)
- (Optional) [🦀 Rust](https://www.rust-lang.org/) - for compiling and running outside of a container
- (Optional) an API key for some Large Language Model [☁️ provider](./providers.md) (e.g. [OpenAI](./providers.md#openai)), though we recommend using [LocalAI](#localai) or [Ollama](#ollama) for local development


### Choosing a homeserver

The development environment supports two homeserver implementations:

- **[Continuwuity](https://continuwuity.org/)** (default) — lightweight, no external database required. Good for most development needs.
- **[Synapse](https://github.com/element-hq/synapse)** — the reference implementation, bundled with Postgres. Use this if you need Synapse-specific behavior.

To choose a homeserver (optional — defaults to Continuwuity if skipped):

```sh
just homeserver-init continuwuity  # or: just homeserver-init synapse
```

The choice is stored in `var/homeserver` and affects all subsequent commands.

> **Note:** If you switch homeservers after initial setup, you will need to:
> - Delete `var/app/local/` and/or `var/app/container/` (app config and data)
> - Delete `var/services/element-web/` (to regenerate its config)
> - Re-run the prepare and user registration steps


### Getting started guide

Developing [locally](#running-locally) is possible, but requires a [Rust](https://www.rust-lang.org/) toolchain.
If this dependency is problematic for you, consider [🐋 running in a container](#running-in-a-container).

In any case, you will need [🐋 Docker](https://www.docker.com/) as [dependency services](../etc/services/) run there.


#### Running locally

1. (Optional) Choose a homeserver: `just homeserver-init continuwuity` (or `synapse`). Default is `continuwuity`.
2. Start the homeserver and Element Web: `just services-start`
3. (Only the first time around) Prepare initial app configuration in `var/app/local/config.yml`: `just app-local-prepare`
4. (Only the first time around) [Prepare your configuration file](#prepare-your-configuration-file)
5. (Only the first time around) Prepare initial default Matrix user accounts (`admin` and `baibot`): `just users-prepare`
6. (Optional) Start additional services depending on which [agent provider you've chosen](#choosing-an-agent-provider):
  - for [LocalAI](#localai):
    - Start services: `just localai-start`
	- Wait a while for LocalAI to start up. It has a lot of models to download. Monitor progress using `just localai-tail-logs`
	- When ready, you'll be able to reach LocalAI's web interface at http://localai.127.0.0.1.nip.io:42027/ (not that you really need it)
  - for [Ollama](#ollama):
    - Start services: `just ollama-start`
    - (Only the first time around) Pull the model configured in `agents.static_definitions` in the configuration file: `just ollama-pull-model gemma2:2b`
7. Start the bot: `just run-locally`
8. Go to http://element.127.0.0.1.nip.io:42025/ and login with `admin` / `admin`
9. Create a new room and invite `@baibot:continuwuity.127.0.0.1.nip.io` (or `@baibot:synapse.127.0.0.1.nip.io` if using Synapse)
10. When done, stop the bot (`Ctrl` + `C`)
11. Stop the services: `just services-stop`
12. (Optional) Stop additional services:
  - for [LocalAI](#localai): `just localai-stop`
  - for [Ollama](#ollama): `just ollama-stop`


#### Running in a container

You can avoid having a [Rust](https://www.rust-lang.org/) toolchain installed locally and build/run this in a container.

1. (Optional) Choose a homeserver: `just homeserver-init continuwuity` (or `synapse`). Default is `continuwuity`.
2. Start the homeserver and Element Web: `just services-start`
3. (Only the first time around) Prepare initial app configuration in `var/app/container/config.yml`: `just app-container-prepare`
4. (Only the first time around) [Prepare your configuration file](#prepare-your-configuration-file)
5. (Only the first time around) Prepare initial default Matrix user accounts (`admin` and `baibot`): `just users-prepare`
6. (Optional) Start additional services depending on which [agent provider you've chosen](#choosing-an-agent-provider):
  - for [LocalAI](#localai):
    - Start services: `just localai-start`
	- Wait a while for LocalAI to start up. It has a lot of models to download. Monitor progress using `just localai-tail-logs`
	- When ready, you'll be able to reach LocalAI's web interface at http://localai.127.0.0.1.nip.io:42027/ (not that you really need it)
  - for [Ollama](#ollama):
    - Start services: `just ollama-start`
    - (Only the first time around) Pull the model configured in `agents.static_definitions` in the configuration file: `just ollama-pull-model gemma2:2b`
7. Start the bot: `just run-in-container`
8. Go to http://element.127.0.0.1.nip.io:42025/ and login with `admin` / `admin`
9. Create a new room and invite `@baibot:continuwuity.127.0.0.1.nip.io` (or `@baibot:synapse.127.0.0.1.nip.io` if using Synapse)
10. When done, stop the bot (`Ctrl` + `C`)
11. Stop the services: `just services-stop`
12. (Optional) Stop additional services:
  - for [LocalAI](#localai): `just localai-stop`
  - for [Ollama](#ollama): `just ollama-stop`


#### Prepare your configuration file

This is about editing your configuration. The initial configuration is created based on `etc/app/config.yml.dist` when you run `just app-local-prepare` or `just app-container-prepare`.

Depending on whether you run locally or in a container, your configuration lives in a different file (`var/app/local/config.yml` and `var/app/container/config.yml`, respectively).

Before starting the bot, you may wish to adjust this configuration.


##### Choosing an agent provider

You can create [🤖 agents](./agents.md) either [statically](./configuration/README.md#static-configuration) or [dynamically](./configuration/README.md#dynamic-configuration) using any of the supported [☁️ providers](./providers.md).

For getting started most quickly (and locally), we recommend using [LocalAI](#localai) or [Ollama](#ollama). These services are already configured to run as [local services via docker-compose](../etc/services/).

**Ollama is most lightweight** (~2GB for the container image + ~1.6GB for the model), but supports only [💬 text-generation](./features.md#-text-generation).

**LocalAI requires 4x more disk space** (~6GB for the container image + ~12GB for the models), but supports [💬 text-generation](./features.md#-text-generation), [🗣️ text-to-speech](./features.md#️-text-to-speech), [🦻 speech-to-text](./features.md#-speech-to-text) and [🖼️ image-generation](./features.md#️-image-creation).

**OpenAI supports all of these capabilities** as well and does not require powerful hardware or lots of disk space. However, it requires signup and an API key.

For local testing, **we recommend LocalAI**, because it runs fully locally and supports more features than Ollama.

###### LocalAI

[LocalAI](./providers.md#localai) supports all [🌟 features](./features.md) of the bot.

If you decided to go with [LocalAI](./providers.md#localai):

- enable the `localai` entry in the `agents.static_definitions` list in the configuration file
- adjust the `initial_global_config.handler.catch_all` setting in the configuration file (`null` -> `static/localai`)

By default, we configure LocalAI to use the [All-In-One images](https://localai.io/basics/container/#all-in-one-images) running on the CPU.
Performance is not great, but it should work reasonably well on good hardware.

If you'd like to use GPU acceleration, you may adjust the `SERVICE_LOCALAI_IMAGE_NAME` variable in [var/services/env](../var/services/env) (this file is automatically prepared for you based on [etc/services/env.dist](../etc/services/env.dist)) to use [other available LocalAI All-In-One images](https://localai.io/basics/container/#available-aio-images).

###### Ollama

[Ollama](./providers.md#ollama) only supports [💬 text-generation](./features.md#-text-generation).

If you decided to go with [Ollama](./providers.md#ollama):

- enable the `ollama` entry in the `agents.static_definitions` list in the configuration file
- adjust the `initial_global_config.handler.catch_all` setting in the configuration file (`null` -> `static/ollama`)

The [gemma2:2b](https://ollama.com/library/gemma2:2b) model was chosen as a default, because it's smallest/lightest and should run well under [Ollama](./providers.md#ollama) on most machines.

###### OpenAI

[OpenAI](./providers.md#openai) supports all [🌟 features](./features.md) of the bot.

If you decided to go with [OpenAI](./providers.md#openai):

- enable the `openai` entry in the `agents.static_definitions` list in the configuration file
- adjust the `initial_global_config.handler.catch_all` setting in the configuration file (`null` -> `static/openai`)
