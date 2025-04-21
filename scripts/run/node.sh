#!/bin/bash

export RUST_LOG=${RUST_LOG:-"info,rpc=debug,runtime=debug"}
export RUST_BACKTRACE=1

build=${BUILD:-"debug"}
bin=./target/$build/cess-node
work_dir="./local_run"
inst_seq=${INST_SEQ:-0}
rpc_port=$((${RPC_PORT:-9944} + $inst_seq))
p2p_port=$((${P2P_PORT:-30333} + $inst_seq))
chain_spec=${CHAIN:-dev}
node_key=$(printf %064d $(($inst_seq + 1)))
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
    --validator \
    --chain $chain_spec \
    $base_path_args \
    --no-telemetry \
    --no-prometheus \
    --no-hardware-benchmarks \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
    --rpc-port $rpc_port \
    --rpc-external \
    --rpc-max-response-size 32 \
    --port $p2p_port \
    --node-key $node_key \
    --state-pruning archive $extra_args