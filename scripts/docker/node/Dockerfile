FROM cesslab/rust-chef-protoc-env:latest as chef
WORKDIR /node

FROM chef AS planner
COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

## build Rust project use caching manner
FROM chef AS builder
ARG https_proxy
ARG http_proxy
ARG BUILD=release
ARG OA
ARG VC

COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config
COPY --from=planner /node/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .

RUN make node

## We do not need the Rust toolchain to run the binary!
FROM ubuntu:20.04 AS runtime

WORKDIR /opt/cess

COPY --from=builder /node/target/release/cess-node .
RUN apt-get update && apt-get install -y curl && apt-get clean -y && apt-get autoremove

ENTRYPOINT ["./cess-node"]