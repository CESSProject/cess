#!/bin/bash

port=8000
work_dir="./standalone/teeworker/ceseal/bin"

export RUST_LOG=debug,ceseal=trace,cestory=trace,h2=info,hyper=info,reqwest=info,tower=info
export RUST_LOG_SANITIZED=false
export RUST_LOG_ANSI_COLOR=true

purge_data=0
getopts ":p" opt
case ${opt} in
    p)
        purge_data=1
        ;;
    *)
        ;;
esac

cd $work_dir

bin="./ceseal"
log_file="ceseal.log"
data_dir="data"

if [[ -e $log_file ]]; then
    rm $log_file
fi
if [[ $purge_data -eq 1 && -e $data_dir ]]; then
    echo "purge data ..."
    rm -rf $data_dir
    mkdir $data_dir
fi

$bin \
    --address 0.0.0.0 \
    --port $port \
    --role full \
|& tee $log_file
