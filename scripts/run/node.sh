#!/bin/bash

export RUST_LOG=info

bin=./target/debug/cess-node
work_dir="./local_run"
inst_seq=${INST_SEQ:-0}
rpc_port=$((${RPC_PORT:-9944} + $inst_seq))
chain_spec=${CHAIN:-dev}
extra_args=${XARGS:---alice}
base_path=$work_dir/chain-$chain_spec-$inst_seq

$bin key generate-node-key --base-path $base_path --chain $chain_spec > /dev/null 2>&1

base_path_args="-d $base_path"
getopts ":t" opt
case ${opt} in
t)
    base_path_args="--tmp"
    ;;
*) ;;
esac

$bin \
    --chain $chain_spec \
    $base_path_args \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    --rpc-port $rpc_port \
    --rpc-external \
    --rpc-max-response-size 32 \
    --state-pruning archive $extra_args