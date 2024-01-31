FROM cesslab/rust-chef-protoc-env:latest as chef
WORKDIR /cifrost

FROM chef AS planner
ARG https_proxy
ARG http_proxy

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
