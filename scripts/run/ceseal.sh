#!/bin/bash

export ROCKET_PORT=8000
export RUST_LOG=info,ceseal=trace,cestory=trace
cd standalone/teeworker/ceseal/bin
rm *.seal*
rm -rf ./data
mkdir data
./ceseal \
    --allow-cors \
    --address 0.0.0.0 \
    --port 8000 \
|& tee ceseal.log

