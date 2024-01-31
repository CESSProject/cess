BUILD?=release
XARGS =
ifeq ($(BUILD),release)
	XARGS = --release
endif
ifeq ($(OA),1)
	XARGS += --features only-attestation
endif
ifeq ($(VC),1)
	XARGS += --features verify-cesealbin
endif

.PHONY: all node ceseal test clippy

all: ceseal
	cargo build ${XARGS}
node:
	cargo build -p cess-node ${XARGS}
cifrost:
	cargo build -p cifrost ${XARGS}
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