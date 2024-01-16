#!/bin/bash

export RUST_LOG=debug,h2=info,hyper=info,reqwest=info,tower=info
export RUST_LOG_SANITIZED=false
export RUST_LOG_ANSI_COLOR=true
export http_proxy=

bin="../cess/target/debug/cifrost"
log_file="./target/cifrost.log"

rm $log_file

$bin \
    --chain-ws-endpoint ws://127.0.0.1:9944 \
    --internal-endpoint http://127.0.0.1:8000 \
    --public-endpoint http://127.0.0.1:19999 \
    --use-dev-key \
    --mnemonic=//Ferdie \
    --attestation-provider none \
|& tee $log_file
