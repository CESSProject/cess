FROM cesslab/intel-sgx-env:latest
ARG https_proxy
ARG http_proxy

RUN curl -fsSL https://deno.land/x/install/install.sh | sh
ENV DENO_INSTALL=/root/.deno
ENV PATH=/root/.deno/bin:${PATH}