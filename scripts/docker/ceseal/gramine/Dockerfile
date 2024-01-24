FROM gramineproject/gramine:v1.5 AS builder

ARG https_proxy
ARG http_proxy
ARG DEBIAN_FRONTEND='noninteractive'
ARG RUST_TOOLCHAIN=1.73.0

ENV https_proxy ${https_proxy}
ENV http_proxy ${http_proxy}

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


# ====== build ceseal ======

ARG IAS_API_KEY
ARG IAS_SPID

ENV IAS_API_KEY=${IAS_API_KEY} \
    IAS_SPID=${IAS_SPID}

RUN mkdir ces-blockchain
ADD . ces-blockchain

RUN mkdir prebuilt

RUN cd ces-blockchain/standalone/teeworker/ceseal/gramine-build && \
    PATH=$PATH:/root/.cargo/bin make dist PREFIX=/root/prebuilt && \
    make clean && \
    rm -rf /root/.cargo/registry && \
    rm -rf /root/.cargo/git

# ====== runtime ======

FROM ubuntu:20.04 AS runtime

ARG https_proxy
ARG http_proxy
ARG DEBIAN_FRONTEND=noninteractive
ARG TZ=Etc/UTC

WORKDIR /opt/ceseal

RUN apt-get update && \
    apt-get install -y curl gnupg2 tini libprotobuf-c1 && \
    curl -fsSLo /usr/share/keyrings/intel-sgx-deb.asc https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key && \
    echo "deb [arch=amd64 signed-by=/usr/share/keyrings/intel-sgx-deb.asc] https://download.01.org/intel-sgx/sgx_repo/ubuntu focal main" | tee /etc/apt/sources.list.d/intel-sgx.list && \
    apt-get update && \
    apt-get install -y sgx-aesm-service libsgx-ae-epid libsgx-ae-le libsgx-ae-pce libsgx-aesm-ecdsa-plugin libsgx-aesm-epid-plugin libsgx-aesm-launch-plugin libsgx-aesm-pce-plugin libsgx-aesm-quote-ex-plugin libsgx-enclave-common libsgx-epid libsgx-launch libsgx-quote-ex libsgx-uae-service libsgx-urts libsgx-ae-qe3 libsgx-pce-logic libsgx-qe3-logic libsgx-dcap-default-qpl libsgx-ra-network libsgx-ra-uefi && \
    apt-get clean -y && \
    apt-get autoremove  

COPY --from=builder /root/prebuilt/ .
ADD ./scripts/docker/ceseal/gramine/start.sh ./start.sh

ENV SGX=1
ENV SKIP_AESMD=0
ENV SLEEP_BEFORE_START=6
ENV RUST_LOG=info
ENV EXTRA_OPTS=
EXPOSE 8000
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/bin/bash", "./start.sh"]