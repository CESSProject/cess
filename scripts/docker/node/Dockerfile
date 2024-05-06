FROM cesslab/rust-chef-protoc-env:latest as chef
WORKDIR /node
COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config

FROM chef AS planner

COPY pallets pallets
COPY crates crates
COPY standalone standalone
COPY Cargo.toml Cargo.lock rustfmt.toml rust-toolchain.toml Makefile ./

RUN --mount=type=cache,target=/usr/local/cargo/registry cargo chef prepare --recipe-path recipe.json

## build Rust project use caching manner
FROM chef AS builder
ARG https_proxy
ARG http_proxy
ARG BUILD=release
ARG OA
ARG VC
ARG GIT_SHA

ENV SUBSTRATE_CLI_GIT_COMMIT_HASH=${GIT_SHA}

COPY --from=planner /node/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo chef cook --release --recipe-path recipe.json

# Build application
COPY pallets pallets
COPY crates crates
COPY standalone standalone
COPY Cargo.toml Cargo.lock rustfmt.toml rust-toolchain.toml Makefile ./

RUN make node

## We do not need the Rust toolchain to run the binary!
FROM ubuntu:20.04 AS runtime

WORKDIR /opt/cess

COPY --from=builder /node/target/release/cess-node .
RUN apt-get update && apt-get install -y curl && apt-get clean -y && apt-get autoremove

ENTRYPOINT ["./cess-node"]