#!/bin/bash

export RUST_LOG=debug,h2=info,hyper=info,reqwest=info,tower=info
export RUST_LOG_SANITIZED=false
export RUST_LOG_ANSI_COLOR=true
export http_proxy=

inst_seq=${INST_SEQ:-0}
ceseal_port=$((${CESEAL_PORT:-8000} + $inst_seq))
pub_port=$((${PUB_PORT:-19999} + $inst_seq))
mnemonic=${MNEMONIC:-//Ferdie}
inject_key=$(printf %064d $(($inst_seq + 1)))

bin="../cess/target/debug/cifrost"
log_file="./target/cifrost-$inst_seq.log"

if [[ -e $log_file ]]; then
    rm $log_file
fi

$bin \
    --chain-ws-endpoint ws://127.0.0.1:9944 \
    --internal-endpoint http://127.0.0.1:$ceseal_port \
    --public-endpoint http://127.0.0.1:$pub_port \
    --inject-key $inject_key \
    --mnemonic $mnemonic \
    --attestation-provider none \
    --longevity 16 \
    --operator cXjHGCWMUM8gM9YFJUK2rqq2tiFWB4huBKWdQPkWdcXcZHhHA |&
    tee $log_file
