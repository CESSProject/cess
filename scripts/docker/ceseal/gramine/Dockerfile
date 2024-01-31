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

RUN : "${IAS_API_KEY:?IAS_API_KEY needs to be set and non-empty.}" \
    && : "${IAS_SPID:?IAS_SPID needs to be set and non-empty.}"
    
RUN mkdir ces-blockchain
ADD . ces-blockchain

RUN mkdir prebuilt

RUN cd ces-blockchain/standalone/teeworker/ceseal/gramine-build && \
    PATH=$PATH:/root/.cargo/bin make dist PREFIX=/root/prebuilt && \
    make clean && \
    rm -rf /root/.cargo/registry && \
    rm -rf /root/.cargo/git

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