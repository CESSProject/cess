#!/bin/bash

export RUST_LOG=info

inst_seq=${INST_SEQ:-0}
rpc_port=$((${RPC_PORT:-9944} + $inst_seq))
chain_spec=${CHAIN:-dev}
extra_args=${XARGS:---alice}

base_path_args="-d ./local_run/chain-$chain_spec-$inst_seq"
getopts ":t" opt
case ${opt} in
t)
    base_path_args="--tmp"
    ;;
*) ;;
esac

./target/debug/cess-node \
    --chain $chain_spec \
    $base_path_args \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    --rpc-port $rpc_port \
    --rpc-external \
    --rpc-max-response-size 32 \
    --state-pruning archive $extra_args