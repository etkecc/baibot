project_name := "baibot"
container_image_name := "localhost/baibot"
project_container_network := "baibot"

# Show help by default
default:
	@just --list --justfile {{ justfile() }}

# Builds and runs a development binary
run-locally *extra_args: app-local-prepare
	RUST_BACKTRACE=1 \
	BAIBOT_CONFIG_FILE_PATH={{ justfile_directory() }}/var/app/local/config.yml \
	BAIBOT_PERSISTENCE_DATA_DIR_PATH={{ justfile_directory() }}/var/app/local/data \
	cargo run --bin baibot -- {{ extra_args }}

# Builds and runs the http server in a container
run-http-server-locally:
	cargo run --bin http-server

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

# Runs a docker-compose command against the core services
docker-compose-core *extra_args:
	just docker-compose core {{ extra_args }}

# Runs a docker-compose command against the localai services
docker-compose-localai *extra_args:
	just docker-compose localai {{ extra_args }}

# Runs a docker-compose command against the ollama services
docker-compose-ollama *extra_args:
	just docker-compose ollama {{ extra_args }}

# Runs all core dependency components (in the background)
services-start: services-prepare (docker-compose-core "up" "-d")

# Stops all core dependency components
services-stop: (docker-compose-core "down")

# Tails the logs for all running core services
services-tail-logs: (docker-compose-core "logs" "-f")

# Prepares the core services for running
services-prepare: _prepare-var-services-env _prepare-var-services-postgres _prepare-var-services-synapse _prepare-container-network

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
users-prepare: services-prepare
	just -f {{ justfile_directory() }}/justfile synapse-register-admin-user "admin" "admin"
	just -f {{ justfile_directory() }}/justfile synapse-register-regular-user "baibot" "baibot"

# Starts a Postgres CLI (psql)
postgres-cli: services-prepare (docker-compose-core "exec" "postgres" "/bin/sh" "-c" "'PGUSER=synapse PGPASSWORD=synapse-password PGDATABASE=homeserver psql -h postgres'")

# Creates an administrator user
synapse-register-admin-user username password: services-prepare
	just -f {{ justfile_directory() }}/justfile docker-compose-core \
		exec synapse \
		register_new_matrix_user \
		--admin \
		-u {{ username }} \
		-p {{ password }} \
		-c /config/homeserver.yaml \
		http://localhost:8008

# Create a regular user
synapse-register-regular-user username password: services-prepare
	just -f {{ justfile_directory() }}/justfile docker-compose-core \
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

_prepare-var-services-ollama:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/services/ollama ]; then
		mkdir -p var/services/ollama
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
		sed --in-place 's/synapse.127.0.0.1.nip.io:42020/synapse:8008/g' var/app/container/config.yml
		sed --in-place 's/127.0.0.1:42026/ollama:11434/g' var/app/container/config.yml
		sed --in-place 's/127.0.0.1:42027/localai:8080/g' var/app/container/config.yml
	fi

_prepare-var-app-container-data:
	#!/bin/sh
	cd {{ justfile_directory() }};

	if [ ! -f var/app/container/data ]; then
		mkdir -p var/app/container/data
	fi

