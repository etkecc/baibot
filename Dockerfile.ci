#######################################
#                                     #
# Stage 1: building                   #
#                                     #
#######################################

FROM docker.io/rust:1.88.0-slim-bookworm AS build

RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev

WORKDIR /app

COPY . /app

RUN cargo build --release

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

COPY --from=build /app/target/release/baibot .

ENTRYPOINT ["/bin/sh", "-c"]

CMD ["/app/baibot"]
