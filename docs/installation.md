## 🚀 Installation

☁️ The easiest way to use the bot is to **get a managed Matrix server from [etke.cc](https://etke.cc/)** and order baibot via the [order form](https://etke.cc/order/). Existing customers can request the inclusion of this additional service by [contacting support](https://etke.cc/contacts/).

💻 If you're managing your Matrix server with the help of the [matrix-docker-ansible-deploy](https://github.com/spantaleev/matrix-docker-ansible-deploy) Ansible playbook, you can easily **install the bot via the Ansible playbook**. See the playbook's [Setting up baibot](https://github.com/spantaleev/matrix-docker-ansible-deploy/blob/master/docs/configuring-playbook-bot-baibot.md) documentation page.

🐋 In other cases, we **recommend using our [prebuilt container images](https://github.com/etkecc/baibot/pkgs/container/baibot) and [running in a container](#-running-in-a-container)**. You can also [build a container image](#building-a-container-image) yourself.

🔨 If containers are not your thing, you can [build a binary](#-building-a-binary) yourself and [run it](#-running-a-binary).

🗲 For a quick experiment, you can refer to the [🧑‍💻 development documentation](./development.md) which contains information on how to build and run the bot (and its various dependency services) locally.


### 🐋 Building a container image

We provide prebuilt container images for the `amd64` and `arm64` architectures, so **you don't necessarily need to build images yourself** and can jump to [Running in a container](#-running-in-a-container).

If you nevertheless wish to build a container image yourself, you can do so by running `just build-container-image`.
This will build and tag your container image as `localhost/baibot:latest`.


### 🐋 Running in a container

We recommend using a **tagged-release** (e.g. `v1.0.0`, not `latest`) of our [prebuilt container images](https://github.com/etkecc/baibot/pkgs/container/baibot), but you can also [build a container image](#-building-a-container-image) yourself.

You should:

- [🛠️ prepare a configuration file](#-preparing-a-configuration-file) (e.g. `cp etc/app/config.yml.dist /path/to/config.yml` & edit it)
- prepare a data directory (`mkdir /path/to/data`)

The example below uses [🐋 Docker](https://www.docker.com/) to run the container, but other container runtimes like [Podman](https://podman.io/) should work as well.

```sh
# Adjust the version tag to point to the latest available tagged version.
# If building your own container image name, adjust to something like `localhost/baibot:latest`.
CONTAINER_IMAGE_NAME=ghcr.io/etkecc/baibot:v1.0.0

/usr/bin/env docker run \
  -it \
  --rm \
  --name=baibot \
  --user=$(id -u):$(id -g) \
  --cap-drop=ALL \
  --read-only \
  --env BAIBOT_PERSISTENCE_DATA_DIR_PATH=/data \
  --mount type=bind,src=/path/to/config.yml,dst=/app/config.yml,ro \
  --mount type=bind,src=/path/to/data,dst=/data \
  $CONTAINER_IMAGE_NAME
```

💡 If you've defined the `persistence.data_dir_path` setting in the `config.yml` file, you can skip the `BAIBOT_PERSISTENCE_DATA_DIR_PATH` environment variable.


### 🔨 Building a binary

To build a binary, you need a [🦀 Rust](https://www.rust-lang.org/) toolchain.

Consult the [Dockerfile](../Dockerfile) file to learn what some of the build dependencies are (e.g. `libssl-dev`, `libsqlite3-dev`, etc., on Debian-based distros).

You can build a binary from the current project's source code:

- in `debug` mode via: `just build-debug`, yielding a binary in `target/debug/baibot`
- (recommended) in `release` mode via: `just build-release`, yielding a binary in `target/release/baibot`

💡 Unless you're [🧑‍💻 developing](./development.md), you probably wish to build in release mode, as that provides a much smaller and more optimized binary.

📦 You can also install from the [baibot](https://crates.io/crates/baibot) crate published to [crates.io](https://crates.io) with the help of the [cargo](https://doc.rust-lang.org/cargo/) package manager by running: `cargo install baibot`.


### 🖥️ Running a binary

Once you've [🔨 built a binary](#-building-a-binary) and [🛠️ prepared a configuration file](#-preparing-a-configuration-file), you can run it.

Consult the [Dockerfile](../Dockerfile) file to learn what some of the runtime dependencies are (e.g. `ca-certificates`, `sqlite3`, etc., on Debian-based distros).

You can run the binary like this:

```sh
BAIBOT_CONFIG_FILE_PATH=/path/to/config.yml \
BAIBOT_PERSISTENCE_DATA_DIR_PATH=/path/to/data \
./target/release/baibot
```

💡 If you've defined the `persistence.data_dir_path` setting in the `config.yml` file, you can skip the `BAIBOT_PERSISTENCE_DATA_DIR_PATH` environment variable.

💡 If your `config.yml` file is in your working directory (which may be different than the directory the binary lives in), you can skip the `BAIBOT_CONFIG_FILE_PATH` environment variable.


### 🛠️ Preparing a configuration file

For an introduction to the configuration file, see the [🛠️ Configuration](./configuration/README.md) page.

Generally, you need to copy the configuration file template ([etc/app/config.yml.dist](../etc/app/config.yml.dist)) and make modifications as needed.
