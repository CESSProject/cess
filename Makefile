.PHONY: all node ceseal test clippy

all: node ceseal

node:
	cargo build --release
ceseal:
	make -C standalone/teeworker/ceseal
test:
	cargo test --workspace --exclude node-executor --exclude ces-node

clippy:
	cargo clippy --tests
	make clippy -C standalone/teeworker/ceseal

lint:
	cargo dylint --all --workspace
