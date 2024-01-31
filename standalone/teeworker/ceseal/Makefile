SGX_MODE ?= SW
BUILD?=release
ifeq ($(BUILD),release)
	XARGS=--release
endif
ifeq ($(OA),1)
	XARGS += --features only-attestation
endif
ifeq ($(VC),1)
	XARGS += --features verify-cesealbin
endif

.PHONY: all clean run app 

all: app 
app: bin/app
run:
	./bin/app -c 0

bin/app: bin/ceseal
	cd bin && ln -sf ceseal app

bin/ceseal: target/${BUILD}/ceseal
	cp $? $@

.PHONY: target/${BUILD}/ceseal
target/${BUILD}/ceseal:
	cargo build ${XARGS}

clean:
	rm -rf bin/*
	cargo clean

.PHONY: clippy
clippy:
	cargo clippy --tests
