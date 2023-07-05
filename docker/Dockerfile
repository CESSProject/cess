# Build cess-node image
FROM ubuntu:20.04
ARG ${http_proxy}
ARG ${https_proxy}
ENV http_proxy=${http_proxy} \
    https_proxy=${https_proxy} \
    DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y curl

COPY cess-node /opt/cess/cess-node

WORKDIR /opt/cess/
