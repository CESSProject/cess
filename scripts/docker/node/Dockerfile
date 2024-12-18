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
COPY scripts/wasm_overrides ./wasm_overrides

RUN make node

## We do not need the Rust toolchain to run the binary!
FROM ubuntu:20.04 AS runtime
ARG https_proxy
ARG http_proxy
ARG CHAIN_NETWORK

WORKDIR /opt/cess

COPY --from=builder /node/target/release/cess-node .
COPY --from=builder /node/wasm_overrides/ .wasm_overrides
RUN if [ "$CHAIN_NETWORK" = "testnet" ]; then ln -s ./.wasm_overrides/testnet ./wasms; else rm -rf .wasm_overrides; fi
RUN apt-get update && apt-get install -y curl && apt-get clean -y && apt-get autoremove

HEALTHCHECK --interval=30s --timeout=5s --start-period=1800s --retries=10 \
    CMD bash -c 'printf "GET / HTTP/1.1\n\n" > /dev/tcp/127.0.0.1/9944 || exit 1'

ENTRYPOINT ["./cess-node"]