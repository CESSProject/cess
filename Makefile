BUILD?=release
OA?=1
VC?=1
XARGS=
DEV=
CHAIN_NETWORK?=dev
ifeq ($(DEV),1)
	OA=0
	VC=0
	BUILD=debug
endif
ifeq ($(BUILD),release)
	XARGS = --release
endif

.PHONY: all node ceseal test clippy

all: node ceseal

node:
	OA=${OA} VC=${VC} CHAIN_NETWORK=${CHAIN_NETWORK} cargo build -p cess-node ${XARGS}
handover:
	cargo build -p handover --release
ceseal:
	$(MAKE) -C standalone/teeworker/ceseal BUILD=${BUILD} OA=${OA} VC=${VC} CHAIN_NETWORK=${CHAIN_NETWORK}
test:
	cargo test --workspace --exclude node-executor --exclude cess-node

clippy:
	cargo clippy --tests
	$(MAKE) clippy -C standalone/teeworker/ceseal

lint:
	cargo dylint --all --workspace

clean:
	cargo clean
	$(MAKE) -C standalone/teeworker/ceseal clean
