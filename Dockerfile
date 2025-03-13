#######################################
#                                     #
# Stage 1: building                   #
#                                     #
#######################################

FROM docker.io/rust:1.85.0-slim-bookworm AS build

RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev

ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/target

WORKDIR /app

COPY . /app

ARG RELEASE_BUILD=true

RUN --mount=type=cache,target=/cargo,sharing=locked \
	--mount=type=cache,target=/target,sharing=locked \
	if [ "$RELEASE_BUILD" = "true" ]; then \
		cargo build --release; \
	else \
		cargo build; \
	fi

# Move it out of the mounted cache, so we can copy it in the next stage.
RUN --mount=type=cache,target=/target,sharing=locked \
	if [ "$RELEASE_BUILD" = "true" ]; then \
		cp /target/release/baibot /baibot; cp /target/release/http-server /http-server; \
	else \
		cp /target/debug/baibot /baibot; cp /target/debug/http-server /http-server;  \
	fi

#######################################
#                                     #
# Stage 2: packaging                  #
#                                     #
#######################################

FROM docker.io/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates sqlite3 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=build /baibot .
COPY --from=build /http-server .

ENTRYPOINT ["/bin/sh", "-c"]
