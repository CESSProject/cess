#!/bin/bash

inst_seq=${INST_SEQ:-0}
pub_port=$((${PUB_PORT:-19999} + $inst_seq))
mnemonic=${MNEMONIC:-//Ferdie}
inject_key=$(printf %064d $(($inst_seq + 1)))

export RUST_LOG=${RUST_LOG:-"info,ceseal=debug,cestory=debug,json-rpc=debug"}
export RUST_LOG_SANITIZED=false
export RUST_LOG_ANSI_COLOR=true
export RUST_BACKTRACE=1

purge_data=0
getopts ":p" opt
case ${opt} in
p)
    purge_data=1
    ;;
*) ;;
esac

build=${BUILD:-"debug"}
bin=./target/$build/ceseal
work_dir="./local_run/ceseal"
data_dir="$work_dir/data-$inst_seq"
log_file="$data_dir/ceseal.log"

if [[ -e $log_file ]]; then
    rm $log_file
fi
if [[ $purge_data -eq 1 && -e $data_dir ]]; then
    echo "purge data ..."
    rm -rf $data_dir
    mkdir $data_dir
fi

$bin \
    --listening-port $pub_port \
    --data-dir $data_dir \
    --public-endpoint http://127.0.0.1:$pub_port \
    --inject-key $inject_key \
    --mnemonic $mnemonic \
    --longevity 16 \
    --role full \
    --stash-account cXjHGCWMUM8gM9YFJUK2rqq2tiFWB4huBKWdQPkWdcXcZHhHA |&
    tee $log_file
