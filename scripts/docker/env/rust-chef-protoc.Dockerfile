FROM lukemathwalker/cargo-chef:latest-rust-1-slim-bullseye AS chef
ARG DEBIAN_FRONTEND='noninteractive'
ARG https_proxy
ARG http_proxy

WORKDIR /root

# Use new protobuf version instead of the default on gramine image, since we need the feature that supporting 'optional' keyword in proto3
RUN apt-get update && apt-get install -y gcc llvm clang cmake make git libssl-dev pkg-config wget unzip
RUN wget --show-progress -q https://github.com/protocolbuffers/protobuf/releases/download/v25.2/protoc-25.2-linux-x86_64.zip \
  && unzip protoc-25.2-linux-x86_64.zip \
  && cp bin/protoc /usr/local/bin/protoc \
  && chmod +x /usr/local/bin/protoc \
  && cp -r include/* /usr/local/include \
  && rm -rf ./* \
  && protoc --version
