FROM gramineproject/gramine:v1.5 AS builder

ARG https_proxy
ARG http_proxy
ARG DEBIAN_FRONTEND='noninteractive'
ARG RUST_TOOLCHAIN=1.82.0

# To fix the intel-sgx PublicKey issue on the gramine image
RUN curl -fsSL https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key | apt-key add -

RUN apt-get update && \
    apt-get upgrade -y
RUN apt-get install -y gcc llvm clang cmake make git rsync libssl-dev pkg-config wget unzip

WORKDIR /root

# Use new protobuf version instead of the default on gramine image, since we need the feature that supporting 'optional' keyword in proto3
RUN wget --show-progress -q https://github.com/protocolbuffers/protobuf/releases/download/v25.2/protoc-25.2-linux-x86_64.zip \
  && unzip protoc-25.2-linux-x86_64.zip \
  && cp bin/protoc /usr/local/bin/protoc \
  && chmod +x /usr/local/bin/protoc \
  && cp -r include/* /usr/local/include \
  && rm -rf ./* \
  && protoc --version

RUN curl -fsSL 'https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init' --output rustup-init && \
    chmod +x ./rustup-init && \
    echo '1' | ./rustup-init --default-toolchain "${RUST_TOOLCHAIN}" && \
    echo 'source /root/.cargo/env' >> .bashrc && \
    .cargo/bin/rustup component add rust-src rust-analysis clippy && \
    .cargo/bin/rustup target add wasm32-unknown-unknown && \
    rm rustup-init && rm -rf .cargo/registry && rm -rf .cargo/git