#!/bin/bash

export RUST_LOG=debug,hyper=info,reqwest=info
export http_proxy=
./target/debug/cifrost \
    --chain-ws-endpoint ws://127.0.0.1:9944 \
    --ceseal-endpoint http://127.0.0.1:8000 \
    --use-dev-key \
    --mnemonic=//Ferdie \
    --attestation-provider none

