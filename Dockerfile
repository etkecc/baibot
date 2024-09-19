#######################################
#                                     #
# Stage 1: building                   #
#                                     #
#######################################

FROM docker.io/rust:1.81.0-slim-bookworm AS build

RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev

ENV CARGO_HOME=/cargo
ENV CARGO_TARGET_DIR=/target

WORKDIR /app

COPY . /app

RUN --mount=type=cache,target=/cargo,sharing=locked \
	--mount=type=cache,target=/target,sharing=locked \
	cargo build --release

# Move it out of the mounted cache, so we can copy it in the next stage.
RUN --mount=type=cache,target=/target,sharing=locked \
	cp /target/release/baibot /baibot


#######################################
#                                     #
# Stage 2: packaging                  #
#                                     #
#######################################

FROM docker.io/debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates sqlite3

WORKDIR /app

COPY --from=build /baibot .

ENTRYPOINT ["/bin/sh", "-c"]

CMD ["/app/baibot"]
