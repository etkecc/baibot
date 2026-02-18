project_name := "baibot"
container_image_name := "localhost/baibot"
project_container_network := "baibot"

admin_username := "admin"
admin_password := "admin"
bot_username := "baibot"
bot_password := "baibot"

homeserver := `cat var/homeserver 2>/dev/null || echo continuwuity`

mise_data_dir := env("MISE_DATA_DIR", justfile_directory() / "var/mise")
mise_trusted_config_paths := justfile_directory() / "mise.toml"

# Show help by default
default:
	@just --list --justfile {{ justfile() }}

# Selects which homeserver implementation to use (continuwuity or synapse)
homeserver-init value:
	#!/bin/sh
	mkdir -p {{ justfile_directory() }}/var
	echo {{ value }} > {{ justfile_directory() }}/var/homeserver
	echo ""
	echo "⚠️  If you had already prepared your app configuration (var/app/local/config.yml or var/app/container/config.yml),"
	echo "    you will need to update it manually or delete it and re-run the prepare step."
	echo "    You should also delete var/app/local/data and/or var/app/container/data,"
	echo "    as old application state is not compatible across homeserver implementations."
	echo ""
	echo "⚠️  If Element Web was already prepared, delete var/services/element-web/ to regenerate its config."

# Builds and runs a development binary
run-locally *extra_args: app-local-prepare
	RUST_BACKTRACE=1 \
	BAIBOT_CONFIG_FILE_PATH={{ justfile_directory() }}/var/app/local/config.yml \
	BAIBOT_PERSISTENCE_DATA_DIR_PATH={{ justfile_directory() }}/var/app/local/data \
	cargo run -- {{ extra_args }}

# Builds and runs the bot in a container
run-in-container *extra_args: app-container-prepare build-container-image-debug
	/usr/bin/env docker run \
	-it \
	--rm \
	--name={{ project_name }} \
	--user=$(id -u):$(id -g) \
	--cap-drop=ALL \
	--read-only \
	--network={{ project_container_network }} \
	--env BAIBOT_PERSISTENCE_DATA_DIR_PATH=/data \
	--mount type=bind,src={{ justfile_directory() }}/var/app/container/config.yml,dst=/app/config.yml,ro \
	--mount type=bind,src={{ justfile_directory() }}/var/app/container/data,dst=/data \
	{{ container_image_name }}:latest {{ extra_args }}

# Runs tests
test *extra_args:
	RUST_BACKTRACE=1 cargo test {{ extra_args }}

# Formats the code
fmt:
	RUST_BACKTRACE=1 cargo fmt --all

# Builds a debug binary (target/debug/*)
build-debug *extra_args:
	RUST_BACKTRACE=1 cargo build {{ extra_args }}

# Builds an optimized release binary (target/release/*)
build-release *extra_args: (build-debug "--release")

# Builds a container image (debug mode)
build-container-image-debug tag='latest': (_build-container-image "false" tag)

# Builds a container image (release mode)
build-container-image-release tag='latest': (_build-container-image "true" tag)

_build-container-image release_build tag:
	/usr/bin/env docker build \
	--build-arg RELEASE_BUILD={{ release_build }} \
	-f {{ justfile_directory() }}/Dockerfile \
	-t {{ container_image_name }}:{{ tag }} \
	.

# Runs a docker-compose command
docker-compose services_type *extra_args:
	/usr/bin/docker compose \
	--project-directory var/services \
	--env-file var/services/env \
	-f etc/services/{{ services_type }}/compose.yml \
	-p {{ project_name }}-{{ services_type }} \
	{{ extra_args }}

# Runs a docker-compose command against the synapse services
docker-compose-synapse *extra_args:
	just docker-compose synapse {{ extra_args }}

# Runs a docker-compose command against the element-web services
docker-compose-element-web *extra_args:
	just docker-compose element-web {{ extra_args }}

# Runs a docker-compose command against the localai services
docker-compose-localai *extra_args:
	just docker-compose localai {{ extra_args }}

# Runs a docker-compose command against the ollama services
docker-compose-ollama *extra_args:
	just docker-compose ollama {{ extra_args }}

# Runs a docker-compose command against the continuwuity services
docker-compose-continuwuity *extra_args:
	just docker-compose continuwuity {{ extra_args }}

# Runs the homeserver and Element Web (in the background)
services-start: services-prepare
	just -f {{ justfile_directory() }}/justfile {{ homeserver }}-start
	just -f {{ justfile_directory() }}/justfile element-web-start

# Stops Element Web and the homeserver
services-stop:
	just -f {{ justfile_directory() }}/justfile element-web-stop
	just -f {{ justfile_directory() }}/justfile {{ homeserver }}-stop

# Tails the logs for the homeserver and Element Web
services-tail-logs:
	just -f {{ justfile_directory() }}/justfile {{ homeserver }}-tail-logs

# Prepares the homeserver and Element Web for running
services-prepare:
	just -f {{ justfile_directory() }}/justfile {{ homeserver }}-prepare
	just -f {{ justfile_directory() }}/justfile element-web-prepare

# Runs Synapse (in the background)
synapse-start: synapse-prepare (docker-compose-synapse "up" "-d")

# Stops Synapse
synapse-stop: (docker-compose-synapse "down")

# Tails the logs for Synapse
synapse-tail-logs: (docker-compose-synapse "logs" "-f")

# Prepares Synapse for running
synapse-prepare: _prepare-var-services-env _prepare-var-services-postgres _prepare-var-services-synapse _prepare-container-network

# Runs Element Web (in the background)
element-web-start: element-web-prepare (docker-compose-element-web "up" "-d")

# Stops Element Web
element-web-stop: (docker-compose-element-web "down")

# Tails the logs for Element Web
element-web-tail-logs: (docker-compose-element-web "logs" "-f")

# Prepares Element Web for running
element-web-prepare: _prepare-var-services-env _prepare-var-services-element-web _prepare-container-network

# Runs LocalAI (in the background)
localai-start: localai-prepare (docker-compose-localai "up" "-d")

# Stops LocalAI
localai-stop: (docker-compose-localai "down")

# Tails the logs for LocalAI
localai-tail-logs: (docker-compose-localai "logs" "-f")

# Prepares LocalAI for running
localai-prepare: _prepare-var-services-env _prepare-var-services-localai _prepare-container-network

# Runs Ollama (in the background)
ollama-start: ollama-prepare (docker-compose-ollama "up" "-d")

# Stops Ollama
ollama-stop: (docker-compose-ollama "down")

# Tails the logs for Ollama
ollama-tail-logs: (docker-compose-ollama "logs" "-f")

# Prepares Ollama for running
ollama-prepare: _prepare-var-services-env _prepare-var-services-ollama _prepare-container-network

# Runs Continuwuity (in the background)
continuwuity-start: continuwuity-prepare (docker-compose-continuwuity "up" "-d")

# Stops Continuwuity
continuwuity-stop: (docker-compose-continuwuity "down")

# Tails the logs for Continuwuity
continuwuity-tail-logs: (docker-compose-continuwuity "logs" "-f")

# Prepares Continuwuity for running
continuwuity-prepare: _prepare-var-services-env _prepare-var-services-continuwuity _prepare-container-network

# Registers a user on Continuwuity via the Matrix Client-Server API
continuwuity-register-user username password:
	{{ justfile_directory() }}/etc/services/continuwuity/register-user.sh {{ justfile_directory() }}/var/services/env {{ username }} {{ password }}

# Prepares the Continuwuity user accounts
continuwuity-users-prepare: continuwuity-prepare
	just -f {{ justfile_directory() }}/justfile continuwuity-register-user "{{ admin_username }}" "{{ admin_password }}"
	just -f {{ justfile_directory() }}/justfile continuwuity-register-user "{{ bot_username }}" "{{ bot_password }}"

# Pulls an Ollama model
ollama-pull-model model_id:
	just -f {{ justfile_directory() }}/justfile docker-compose-ollama \
		exec ollama \
		ollama pull {{ model_id }}

# Prepares the app for running locally
app-local-prepare: _prepare-var-app-local-config_yml _prepare-var-app-local-data

# Prepares the app for running in a container
app-container-prepare: _prepare-var-app-container-config_yml _prepare-var-app-container-data

# Prepares the user accounts
users-prepare:
	just -f {{ justfile_directory() }}/justfile {{ homeserver }}-users-prepare

# Prepares the Synapse user accounts
synapse-users-prepare: synapse-prepare
	just -f {{ justfile_directory() }}/justfile synapse-register-admin-user "{{ admin_username }}" "{{ admin_password }}"
	just -f {{ justfile_directory() }}/justfile synapse-register-regular-user "{{ bot_username }}" "{{ bot_password }}"

# Starts a Postgres CLI (psql)
postgres-cli: synapse-prepare (docker-compose-synapse "exec" "postgres" "/bin/sh" "-c" "'PGUSER=synapse PGPASSWORD=synapse-password PGDATABASE=homeserver psql -h postgres'")

# Creates an administrator user on Synapse
synapse-register-admin-user username password: synapse-prepare
	just -f {{ justfile_directory() }}/justfile docker-compose-synapse \
		exec synapse \
		register_new_matrix_user \
		--admin \
		-u {{ username }} \
		-p {{ password }} \
		-c /config/homeserver.yaml \
		http://localhost:8008

# Creates a regular user on Synapse
synapse-register-regular-user username password: synapse-prepare
	just -f {{ justfile_directory() }}/justfile docker-compose-synapse \
		exec synapse \
		register_new_matrix_user \
		--no-admin \
		-u {{ username }} \
		-p {{ password }} \
		-c /config/homeserver.yaml \
		http://localhost:8008

# Runs the clippy linter
clippy *extra_args:
	cargo clippy {{ extra_args }}

# Checks that the code compiles without building
check:
	cargo check

# Invokes mise with the project-local data directory
mise *args: _ensure_mise_data_directory
	#!/bin/sh
	export MISE_DATA_DIR="{{ mise_data_dir }}"
	export MISE_TRUSTED_CONFIG_PATHS="{{ mise_trusted_config_paths }}"
	mise {{ args }}

# Runs prek (pre-commit hooks manager) with the given arguments
prek *args: _ensure_mise_tools_installed
	@just --justfile {{ justfile() }} mise exec -- prek {{ args }}

# Runs pre-commit hooks on staged files
prek-run-on-staged *args: _ensure_mise_tools_installed
	@just --justfile {{ justfile() }} mise exec -- prek run {{ args }}

# Runs pre-commit hooks on all files
prek-run-on-all *args: _ensure_mise_tools_installed
	@just --justfile {{ justfile() }} mise exec -- prek run --all-files {{ args }}

# Installs the git pre-commit hook (runs prek automatically before each commit)
prek-install-git-pre-commit-hook: _ensure_mise_tools_installed
	@just --justfile {{ justfile() }} mise exec -- prek install

# Internal - ensures var/mise directory exists
_ensure_mise_data_directory:
	#!/bin/sh
	if [ ! -d "{{ mise_data_dir }}" ]; then
		mkdir -p "{{ mise_data_dir }}"
	fi

# Internal - ensures mise tools are installed
_ensure_mise_tools_installed: _ensure_mise_data_directory
	@just --justfile {{ justfile() }} mise install --quiet

_prepare-var-services-env:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/env ]; then
		mkdir -p var/services
		cp {{ justfile_directory() }}/etc/services/env.dist var/services/env
		echo 'UID='`id -u` >> var/services/env;
		echo 'GID='`id -g` >> var/services/env;
		echo 'NETWORK_NAME={{ project_container_network }}' >> var/services/env;
	fi

_prepare-var-services-postgres:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/postgres ]; then
		mkdir -p var/services/postgres
		chown `id -u`:`id -g` var/services/postgres
	fi

_prepare-var-services-synapse:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/synapse ]; then
		mkdir -p var/services/synapse/media-store
	fi

_prepare-var-services-element-web:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/element-web/config.json ]; then
		mkdir -p var/services/element-web
		cp {{ justfile_directory() }}/etc/services/element-web/config.json.dist var/services/element-web/config.json

		homeserver="{{ homeserver }}"
		if [ "$homeserver" = "continuwuity" ]; then
			sed --in-place 's|__HOMESERVER_CLIENT_URL__|http://continuwuity.127.0.0.1.nip.io:42030|g' var/services/element-web/config.json
		elif [ "$homeserver" = "synapse" ]; then
			sed --in-place 's|__HOMESERVER_CLIENT_URL__|http://synapse.127.0.0.1.nip.io:42020|g' var/services/element-web/config.json
		fi
	fi

_prepare-var-services-ollama:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/ollama ]; then
		mkdir -p var/services/ollama
	fi

_prepare-var-services-continuwuity:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/continuwuity ]; then
		mkdir -p var/services/continuwuity/data
	fi

_prepare-var-services-localai:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/localai ]; then
		mkdir -p var/services/localai
	fi

_prepare-container-network:
	#!/bin/sh
	network_definition=$(/usr/bin/env docker network ls --filter='name={{ project_container_network }}' --format=json)

	if [ "$network_definition" = "" ]; then
		/usr/bin/docker network create {{ project_container_network }}
	fi

_prepare-var-app-local-config_yml:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/app/local/config.yml ]; then
		mkdir -p var/app/local
		cp {{ justfile_directory() }}/etc/app/config.yml.dist var/app/local/config.yml

		homeserver="{{ homeserver }}"
		if [ "$homeserver" = "continuwuity" ]; then
			sed --in-place 's/__HOMESERVER_SERVER_NAME__/continuwuity.127.0.0.1.nip.io/g' var/app/local/config.yml
			sed --in-place 's|__HOMESERVER_URL__|http://continuwuity.127.0.0.1.nip.io:42030|g' var/app/local/config.yml
		elif [ "$homeserver" = "synapse" ]; then
			sed --in-place 's/__HOMESERVER_SERVER_NAME__/synapse.127.0.0.1.nip.io/g' var/app/local/config.yml
			sed --in-place 's|__HOMESERVER_URL__|http://synapse.127.0.0.1.nip.io:42020|g' var/app/local/config.yml
		fi
	fi

_prepare-var-app-local-data:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/app/local/data ]; then
		mkdir -p var/app/local/data
	fi

_prepare-var-app-container-config_yml:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/app/container/config.yml ]; then
		mkdir -p var/app/container
		cp {{ justfile_directory() }}/etc/app/config.yml.dist var/app/container/config.yml

		homeserver="{{ homeserver }}"
		if [ "$homeserver" = "continuwuity" ]; then
			sed --in-place 's/__HOMESERVER_SERVER_NAME__/continuwuity.127.0.0.1.nip.io/g' var/app/container/config.yml
			sed --in-place 's|__HOMESERVER_URL__|http://continuwuity.127.0.0.1.nip.io:42030|g' var/app/container/config.yml
			sed --in-place 's/continuwuity.127.0.0.1.nip.io:42030/continuwuity:6167/g' var/app/container/config.yml
		elif [ "$homeserver" = "synapse" ]; then
			sed --in-place 's/__HOMESERVER_SERVER_NAME__/synapse.127.0.0.1.nip.io/g' var/app/container/config.yml
			sed --in-place 's|__HOMESERVER_URL__|http://synapse.127.0.0.1.nip.io:42020|g' var/app/container/config.yml
			sed --in-place 's/synapse.127.0.0.1.nip.io:42020/synapse:8008/g' var/app/container/config.yml
		fi

		sed --in-place 's/127.0.0.1:42026/ollama:11434/g' var/app/container/config.yml
		sed --in-place 's/127.0.0.1:42027/localai:8080/g' var/app/container/config.yml
	fi

_prepare-var-app-container-data:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/app/container/data ]; then
		mkdir -p var/app/container/data
	fi
