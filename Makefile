.PHONY: all node ceseal ciforst test clippy

all: node ceseal

node:
	cargo build
ciforst:
	cargo build -p cifrost
ceseal:
	make -C standalone/teeworker/ceseal
test:
	cargo test --workspace --exclude node-executor --exclude cess-node

clippy:
	cargo clippy --tests
	make clippy -C standalone/teeworker/ceseal

lint:
	cargo dylint --all --workspace

clean:
	cargo clean
	make -C standalone/teeworker/ceseal clean