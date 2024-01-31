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
