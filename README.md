# ![CESS](https://raw.githubusercontent.com/Cumulus2021/W3F-illustration/main/banner2.76d5edd0.png)

[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-blue?logo=Parity%20Substrate)](https://substrate.dev/) [![GitHub license](https://img.shields.io/badge/license-GPL3%2FApache2-blue)](#LICENSE)

-----------❇️-----------**An infrastructure of decentralized cloud data network built with [Substrate](https://substrate.dev/)**----------❇️----------

-------------------❇️-------------------**Learn more at [cess.cloud](http://cess.cloud/) & with [white-paper](https://github.com/Cumulus2021/Whitepaper)**-------------------❇️------------------

## Getting Started


### Install Guide

Follow [Setup](https://github.com/Cumulus2021/cess/blob/main/docs/setup.md) to guide you install the CESS development.

### Build Node

The `cargo run` command will perform an initial build. Use the following command to build the node without launching it:

```
# Fetch the code
git clone https://github.com/Cumulus2021/cess.git
cd cess

# Build the node (The first build will be long (~30min))
cargo build --release
```

## Run The CESS Node


After the node has finished compiling, you can follow these steps below to run it. 

### Generate Keys

If you already have keys for Substrate using the [SS58 address encoding format](https://github.com/paritytech/substrate/wiki/External-Address-Format-(SS58)), please see the next section.

Begin by compiling and installing the utility ([instructions and more info here](https://substrate.dev/docs/en/knowledgebase/integrate/subkey)). 

Generate a mnemonic (Secret phrase) and see the `sr25519` key and address associated with it.

```
# subkey command
subkey generate --scheme sr25519
```

Now see the `ed25519` key and address associated with the same mnemonic (Secret phrase).

```
# subkey command
subkey inspect --scheme ed25519 "SECRET PHRASE YOU JUST GENERATED"
```

We recommend that you record the above outputs and keep mnemonic in safe.

### Run Testnet C-ALPHA

Launch node on the C-ALPHA with:

```
# start
./target/release/cess-node --base-path /tmp/cess --chain C-ALPHA
```

Then you can add an account with:

```
# create key file
vim secretKey.txt

# add secret phrase for the node in the file
YOUR ACCOUNT'S SECRET PHRASE
```

```
# add key to node
./target/release/cess-node key insert --base-path /tmp/cess --chain C-ALPHA --scheme Sr25519  --key-type aura --suri /root/secretKey.txt

./target/release/cess-node key insert --base-path /tmp/cess --chain C-ALPHA --scheme Ed25519  --key-type gran --suri /root/secretKey.txt
```

Now you can launch node again:

```
# start
./target/release/cess-node --base-path /tmp/cess --chain C-ALPHA
```

### Run in Docker

Install [Docker](https://docs.docker.com/get-docker/) first, and run the following command to start a node on the C-ALPHA:

```
docker pull cesstech/c-alpha:v0.0.1
docker run --network host cesstech/c-alpha:v0.0.1 ./CESS-v0.0.1/target/release/cess-node --base-path /tmp/cess --chain C-ALPHA
```

## Run Tests


CESS has Rust unit tests, and can be run locally.

```
# Run all the Rust unit tests
cargo test --release
```

## Module Documentation


* [Files Bank](https://github.com/Cumulus2021/cess/tree/main/c-pallets/files-bank)
* [Files map](https://github.com/Cumulus2021/cess/tree/main/c-pallets/files-map)
* [Sminer](https://github.com/Cumulus2021/cess/tree/main/c-pallets/sminer)

## Contribute


Please follow the contributions guidelines as outlined in [`docs/CONTRIBUTING.adoc`](https://github.com/Cumulus2021/cess/blob/main/docs/CONTRIBUTING.adoc). In all communications and contributions, this project follows the [Contributor Covenant Code of Conduct](https://github.com/paritytech/substrate/blob/master/docs/CODE_OF_CONDUCT.md).
