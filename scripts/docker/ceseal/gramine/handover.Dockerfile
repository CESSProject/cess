# ====== build ceseal ======

FROM cesslab/gramine-rust-env:latest AS builder

WORKDIR /root

ARG https_proxy
ARG http_proxy
ARG IAS_API_KEY
ARG IAS_SPID
ARG IAS_ENV
ARG BUILD=release
ARG OA
ARG VC
ARG GIT_SHA

RUN <<EOF
  set -e
  : "${IAS_API_KEY:?IAS_API_KEY needs to be set and non-empty.}"
  : "${IAS_SPID:?IAS_SPID needs to be set and non-empty.}"
  mkdir cess-code
  mkdir prebuilt
EOF

# COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config
COPY pallets ./cess-code/pallets
COPY crates ./cess-code/crates
COPY standalone ./cess-code/standalone
COPY Cargo.toml Cargo.lock rustfmt.toml rust-toolchain.toml Makefile ./cess-code/

ENV VERGEN_GIT_SHA=${GIT_SHA}

RUN <<EOF
  set -e
  PATH=$PATH:/root/.cargo/bin
  cd /root/cess-code
  make handover
  cp ./target/release/handover /root/prebuilt
  cd /root/cess-code/standalone/teeworker/ceseal/gramine-build
  make dist PREFIX=/root/prebuilt
  make clean
  rm -rf /root/.cargo/registry
  rm -rf /root/.cargo/git
EOF

# ====== runtime ======

FROM cesslab/intel-sgx-deno-env:latest AS runtime

ARG https_proxy
ARG http_proxy
ARG CESEAL_VERSION
RUN : "${CESEAL_VERSION:?CESEAL_VERSION needs to be set and a long integer.}"
ARG CESEAL_HOME=/opt/ceseal
ARG CESEAL_DIR=${CESEAL_HOME}/releases/${CESEAL_VERSION}
ARG CESEAL_DATA_DIR=${CESEAL_HOME}/${CESEAL_VERSION}/data
ARG REAL_CESEAL_DATA_DIR=${CESEAL_HOME}/data/${CESEAL_VERSION}

COPY --from=builder /root/prebuilt/ ${CESEAL_DIR}
ADD --chmod=0755 ./scripts/docker/ceseal/gramine/start.sh ${CESEAL_DIR}/start.sh
ADD --chmod=0755 ./scripts/docker/ceseal/gramine/start-with-handover.sh ${CESEAL_HOME}/start.sh


RUN <<EOF
  set -e
  ln -s ${CESEAL_DIR} ${CESEAL_HOME}/releases/current
  mkdir -p ${REAL_CESEAL_DATA_DIR}
  rm -rf ${CESEAL_DIR}/data
  ln -s ${REAL_CESEAL_DATA_DIR} ${CESEAL_DIR}/data
EOF

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
HEALTHCHECK --start-period=8s --timeout=5s \
  CMD curl -s --fail --http2-prior-knowledge http://localhost:8000 || exit 1
