FROM lukemathwalker/cargo-chef:latest-rust-1-slim-bullseye AS chef
ARG https_proxy
ARG http_proxy

WORKDIR /cifrost

FROM chef AS planner
COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

## build Rust project use caching manner
FROM chef AS builder
ARG DEBIAN_FRONTEND='noninteractive'
ARG https_proxy
ARG http_proxy
ARG BUILD=release
ARG OA
ARG VC

# Use new protobuf version instead of the default on gramine image, since we need the feature that supporting 'optional' keyword in proto3
RUN apt-get update && apt-get install -y gcc llvm clang cmake make git libssl-dev pkg-config wget unzip
RUN wget --show-progress -q https://github.com/protocolbuffers/protobuf/releases/download/v25.2/protoc-25.2-linux-x86_64.zip \
  && unzip protoc-25.2-linux-x86_64.zip \
  && cp bin/protoc /usr/local/bin/protoc \
  && chmod +x /usr/local/bin/protoc \
  && cp -r include/* /usr/local/include \
  && rm -rf ./* \
  && protoc --version

COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config
COPY --from=planner /cifrost/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .

RUN make cifrost

## We do not need the Rust toolchain to run the binary!
FROM ubuntu:20.04 AS runtime

WORKDIR /opt/cifrost

COPY --from=builder /cifrost/target/release/cifrost .

ENTRYPOINT ["./cifrost"]
