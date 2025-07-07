SGX_SIGNER_KEY ?= ./private.dev.pem
SGX ?= 1
BUILD ?= release
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

ARCH_LIBDIR ?= /lib/$(shell $(CC) -dumpmachine)

CFLAGS = -Wall -Wextra

GRAMINE_LOG_LEVEL = error

USE_MUSL ?= 0

BIN_NAME = ceseal

DATA_DIRS = data/protected_files \
            data/storage_files

GRAMINE_DIR ?= $(shell ./gramine-dir libs)
GRAMINE_LIBOS ?= $(shell ./gramine-dir libos)
GRAMINE_RUNTIME_DIR ?= $(shell ./gramine-dir runtime)

RUNTIME_DIR = cruntime
LIBOS_BASENAME ?= $(shell basename ${GRAMINE_LIBOS})
LIBOS ?= ${RUNTIME_DIR}/${LIBOS_BASENAME}

BIN_TARGET_DIR = ../../../../target
ifeq ($(USE_MUSL),1)
BIN_FILE = ${BIN_TARGET_DIR}/x86_64-unknown-linux-musl/${BUILD}/${BIN_NAME}
CARGO_ARGS = --target x86_64-unknown-linux-musl
HOST_LIBC_DIR = /lib/x86_64-linux-musl
else
BIN_FILE = ${BIN_TARGET_DIR}/${BUILD}/${BIN_NAME}
CARGO_ARGS =
HOST_LIBC_DIR = /lib/x86_64-linux-gnu
endif

ifeq ($(V),1)
CARGO_ARGS += -vv
endif

PREFIX ?= ../bin
CESEAL_DATA_DIR ?= data
CESEAL_SEAL_DIR ?= ${CESEAL_DATA_DIR}/protected_files
CESEAL_STORAGE_DIR ?= ${CESEAL_DATA_DIR}/storage_files

.PHONY: check-gramine-env
check-gramine-env:
	@./gramine-dir libs >/dev/null 2>&1 || { echo "Error: gramine-dir script failed, please check Gramine environment!"; exit 1; }

.PHONY: all
all: check-gramine-env ${BIN_NAME} LibOS

.PHONY: signed-sgx-artifacts
signed-sgx-artifacts: check-gramine-env ${BIN_NAME}.manifest ${BIN_NAME}.manifest.sgx ${BIN_NAME}.sig

.PHONY: ${BIN_FILE}
${BIN_FILE}:
	@echo "SGX_PROFILE: ${SGX_PROFILE}"
	@echo "BUILD: ${BUILD}"
	@echo "OA: ${OA}"
	@echo "VC: ${VC}"
	OA=${OA} VC=${VC} CHAIN_NETWORK=${CHAIN_NETWORK} cargo build --manifest-path ../Cargo.toml -p ceseal ${XARGS}

${BIN_NAME}: ${BIN_FILE}
	cp ${BIN_FILE} ${BIN_NAME}	

${BIN_NAME}.manifest: check-gramine-env ${BIN_NAME}.manifest.template
	gramine-manifest \
		-Dinstall_dir=$(INSTALL_DIR) \
		-Dlog_level=$(GRAMINE_LOG_LEVEL) \
		-Dseal_dir=${CESEAL_SEAL_DIR} \
		-Dstorage_dir=${CESEAL_STORAGE_DIR} \
		-Dlibdir=${RUNTIME_DIR}/lib/ \
		-Darch_libdir=${ARCH_LIBDIR}/ \
		-Dlibos=${LIBOS} \
		$(word 2,$^) $@

${BIN_NAME}.manifest.sgx: ${BIN_NAME}.manifest ${BIN_NAME} LibOS
	@test -s $(SGX_SIGNER_KEY) || \
	    { echo "SGX signer private key was not found, please specify SGX_SIGNER_KEY!"; exit 1; }
	gramine-sgx-sign \
		--key $(SGX_SIGNER_KEY) \
		--manifest $< \
		--output $@

.PHONY: LibOS
LibOS: check-gramine-env
	mkdir -p ${RUNTIME_DIR}
	rsync -r --no-links ${GRAMINE_RUNTIME_DIR}/ ${RUNTIME_DIR}/lib
ifeq ($(USE_MUSL),0)
	cp -rfL ${HOST_LIBC_DIR}/libgcc_s.so.1 ${RUNTIME_DIR}/lib/
endif
ifeq ($(SGX),1)
	cp -rfL ${GRAMINE_DIR}/sgx ${RUNTIME_DIR}/
endif
	cp -rfL ${GRAMINE_LIBOS} ${RUNTIME_DIR}/

${BIN_NAME}.sig: ${BIN_NAME}.manifest.sgx

.PHONY: sig
sig: ${BIN_NAME}.sig

.PHONY: dirs
dirs: ${DATA_DIRS}

${DATA_DIRS}:
	mkdir -p $@

.PHONY: clean
clean:
	$(RM) *.sig *.manifest.sgx *.manifest ${BIN_NAME}.o ${BIN_NAME}
	$(RM) -rf data
	$(RM) -rf ${RUNTIME_DIR}


.PHONY: pre-dist
pre-dist: all
	mkdir -p ${PREFIX}/data
	cp ${BIN_NAME} ${PREFIX}/
	cp -rfL ${RUNTIME_DIR} ${PREFIX}/

.PHONY: dist
ifeq ($(SGX),1)
dist: pre-dist ${BIN_NAME}.manifest signed-sgx-artifacts
	cp ${BIN_NAME}.manifest ${PREFIX}/
	cp ${BIN_NAME}.manifest.sgx ${PREFIX}/
	cp ${BIN_NAME}.sig ${PREFIX}/
	cp gramine-sgx ${PREFIX}/
else
dist: pre-dist ${BIN_NAME}.manifest
	cp ${BIN_NAME}.manifest ${PREFIX}/
endif

.PHONY: run
run: all sig
	make dirs
ifeq ($(SGX),1)
	./gramine-sgx ceseal
else
	gramine-direct ceseal
endif
