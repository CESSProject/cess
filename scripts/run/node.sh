#!/bin/bash

#    -d target/chain-dev-local \

export RUST_LOG=info
#export RUST_LOG=debug,wasmtime_cranelift=info,netlink_proto
./target/debug/cess-node \
    --dev --alice \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    --rpc-port 9944 \
    --rpc-external \
    --rpc-max-response-size 32 \
    --validator \
    --state-pruning archive