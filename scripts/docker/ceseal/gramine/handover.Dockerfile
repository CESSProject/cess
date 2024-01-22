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
RUN apt-get install -y gcc llvm clang cmake make git rsync

WORKDIR /root

# Use new protobuf version instead of the default on gramine image, since we need the feature that supporting 'optional' keyword in proto3
RUN git clone --depth 1 --branch v25.0 --recurse-submodules https://github.com/protocolbuffers/protobuf.git
RUN cd protobuf && \
    cmake -Dprotobuf_BUILD_TESTS=OFF -DCMAKE_CXX_STANDARD=14 --config Release . && \
    cmake --build . && \
    cmake --install . && \
    protoc --version

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

RUN : "${IAS_API_KEY:?IAS_API_KEY needs to be set and non-empty.}" \
    && : "${IAS_SPID:?IAS_SPID needs to be set and non-empty.}" \
    && mkdir to_build_source \
    && mkdir prebuilt
ADD . to_build_source
RUN cd to_build_source/standalone/teeworker/ceseal/gramine-build && \
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

RUN apt-get update && \
    apt-get install -y curl gnupg2 tini libprotobuf-c1 unzip && \
    curl -fsSLo /usr/share/keyrings/intel-sgx-deb.asc https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key && \
    echo "deb [arch=amd64 signed-by=/usr/share/keyrings/intel-sgx-deb.asc] https://download.01.org/intel-sgx/sgx_repo/ubuntu focal main" | tee /etc/apt/sources.list.d/intel-sgx.list && \
    apt-get update && \
    apt-get install -y sgx-aesm-service libsgx-ae-epid libsgx-ae-le libsgx-ae-pce libsgx-aesm-ecdsa-plugin libsgx-aesm-epid-plugin libsgx-aesm-launch-plugin libsgx-aesm-pce-plugin libsgx-aesm-quote-ex-plugin libsgx-enclave-common libsgx-epid libsgx-launch libsgx-quote-ex libsgx-uae-service libsgx-urts libsgx-ae-qe3 libsgx-pce-logic libsgx-qe3-logic libsgx-dcap-default-qpl libsgx-ra-network libsgx-ra-uefi && \
    apt-get clean -y && \
    apt-get autoremove  

ARG CESEAL_VERSION
RUN : "${CESEAL_VERSION:?CESEAL_VERSION needs to be set and a long integer.}"
ARG CESEAL_HOME=/opt/ceseal
ARG CESEAL_DIR=${CESEAL_HOME}/releases/${CESEAL_VERSION}
ARG CESEAL_DATA_DIR=${CESEAL_HOME}/${CESEAL_VERSION}/data
ARG REAL_CESEAL_DATA_DIR=${CESEAL_HOME}/data/${CESEAL_VERSION}

COPY --from=builder /root/prebuilt/ ${CESEAL_DIR}
ADD --chmod=0755 ./scripts/docker/ceseal/gramine/start.sh ${CESEAL_DIR}/start.sh
ADD --chmod=0755 ./scripts/docker/ceseal/gramine/start-with-handover.sh ${CESEAL_HOME}/start.sh
ADD ./scripts/docker/ceseal/gramine/handover.ts ${CESEAL_HOME}/handover.ts

RUN ln -s ${CESEAL_DIR} ${CESEAL_HOME}/releases/current \
    && mkdir -p ${REAL_CESEAL_DATA_DIR} \
    && rm -rf ${CESEAL_DIR}/data \
    && ln -s ${REAL_CESEAL_DATA_DIR} ${CESEAL_DIR}/data

RUN curl -fsSL https://deno.land/x/install/install.sh | sh
ENV DENO_INSTALL=/root/.deno
ENV PATH=/root/.deno/bin:${PATH}

RUN deno cache --reload ${CESEAL_HOME}/handover.ts

WORKDIR ${CESEAL_HOME}/releases/current

ENV SGX=1
ENV SKIP_AESMD=0
ENV SLEEP_BEFORE_START=6
ENV RUST_LOG=info
ENV EXTRA_OPTS=
ENV CESEAL_HOME=${CESEAL_HOME}

EXPOSE 8000
SHELL ["/bin/bash", "-c"]
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ${CESEAL_HOME}/start.sh
