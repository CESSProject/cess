BUILD?=release
OA?=1
VC?=1
XARGS=
DEV=
ifeq ($(DEV),1)
	OA=0
	VC=0
	BUILD=debug
endif
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
handover:
	cargo build -p handover --release
ceseal:
	$(MAKE) -C standalone/teeworker/ceseal BUILD=${BUILD} OA=${OA} VC=${VC}
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
