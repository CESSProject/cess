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

.PHONY: all node test clippy chain-runtime

all: node 

node:
	OA=${OA} VC=${VC} CHAIN_NETWORK=${CHAIN_NETWORK} cargo build -p cess-node ${XARGS}

chain-runtime:
	OA=${OA} VC=${VC} CHAIN_NETWORK=${CHAIN_NETWORK} cargo build -p cess-node-runtime ${XARGS}


BOOT_NODES=
check-boot-nodes:
	@if [ -z "$(BOOT_NODES)" ]; then \
		echo "Error: BOOT_NODES variable is not set or is empty."; \
		exit 1; \
	fi

check-chain-network:
	@if [ -z "$(CHAIN_NETWORK)" ]; then \
		echo "Error: CHAIN_NETWORK variable is not set or is empty."; \
		exit 1; \
	elif [ "$(CHAIN_NETWORK)" = "dev" ]; then \
		echo "dev does not support stage chain-spec, use devnet instead"; \
		CHAIN_NETWORK=devnet; \
	fi

CES_NODE_BIN_FILE = target/${BUILD}/cess-node
.PHONY: stage-chain-spec
stage-chain-spec: check-boot-nodes check-chain-network node
	@# Convert BOOT_NODES to the array format required by jq
	@BOOT_NODES_ARRAY=$$(echo "$(BOOT_NODES)" | sed 's/,/","/g; s/^/["/; s/$$/"]/'); \
	  ${CES_NODE_BIN_FILE} build-spec --chain cess-initial-${CHAIN_NETWORK} --raw --disable-default-bootnode  | \
	  jq ".bootNodes = \$$BOOT_NODES_ARRAY | .telemetryEndpoints = []" --argjson BOOT_NODES_ARRAY "$$BOOT_NODES_ARRAY" > \
	  ./standalone/chain/node/ccg/${CHAIN_NETWORK}-spec-raw.json


test:
	cargo test --workspace --exclude node-executor --exclude cess-node

clippy:
	cargo clippy --tests
	$(MAKE) clippy -C standalone/teeworker/ceseal

lint:
	cargo dylint --all --workspace

clean:
	cargo clean