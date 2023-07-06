# Build for cesslab/ci-linux image
FROM paritytech/ci-linux:production
ARG ${http_proxy}
ARG ${https_proxy}
ENV http_proxy=${http_proxy} \
    https_proxy=${https_proxy}
RUN apt-get update \
    && apt-get install -y lsb-release wget software-properties-common gnupg \
    && curl https://apt.llvm.org/llvm.sh -sSf > llvm.sh \
    && chmod +x llvm.sh \
    && ./llvm.sh 11 \
    && rm ./llvm.sh
