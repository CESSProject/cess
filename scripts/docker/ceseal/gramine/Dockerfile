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
EOF

COPY ./scripts/docker/cargo-config.toml /usr/local/cargo/config
COPY pallets ./cess-code/pallets
COPY crates ./cess-code/crates
COPY standalone ./cess-code/standalone
COPY Cargo.toml Cargo.lock rustfmt.toml rust-toolchain.toml Makefile ./cess-code/

RUN mkdir prebuilt

ENV VERGEN_GIT_SHA=${GIT_SHA}

RUN <<EOF
  set -e
  cd cess-code/standalone/teeworker/ceseal/gramine-build
  PATH=$PATH:/root/.cargo/bin
  make dist PREFIX=/root/prebuilt
  make clean
  rm -rf /root/.cargo/registry
  rm -rf /root/.cargo/git
EOF

# ====== runtime ======

FROM cesslab/intel-sgx-env:latest AS runtime

ARG https_proxy
ARG http_proxy

WORKDIR /opt/ceseal

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