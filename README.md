# ![CESS](https://raw.githubusercontent.com/Cumulus2021/W3F-illustration/main/banner2.76d5edd0.png)

[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-blue?logo=Parity%20Substrate)](https://substrate.dev/) [![GitHub license](https://img.shields.io/badge/license-GPL3%2FApache2-blue)](#LICENSE)

---------🌌---------**An infrastructure of decentralized cloud data network built with [Substrate](https://substrate.dev/)**--------🌌--------

----------------🌌----------------**Learn more at [cess.cloud](http://cess.cloud/) & with [white-paper](https://github.com/CESSProject/Whitepaper)**----------------🌌--------------

## Getting Started


### Install Guide

Follow [Setup](https://github.com/CESSProject/cess/tree/v0.1.1/docs/setup.md) to guide you install the CESS development.

### Build Node

The `cargo run` command will perform an initial build. Use the following command to build the node without launching it:

```
# Fetch the code
git clone https://github.com/CESSProject/cess.git
cd cess

# Build the node (The first build will be long (~30min))
cargo build --release
```

## Run The CESS Node


After the node has finished compiling, you can follow these steps below to run it. 

### Generate Keys

If you already have keys for Substrate using the [SS58 address encoding format](https://docs.substrate.io/v3/advanced/ss58/), please see the next section.

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

### Run Testnet

Launch node on the cess-testnet with:

```
# start
./target/release/cess-node --base-path /tmp/cess --chain cess-testnet
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
./target/release/cess-node key insert --base-path /tmp/cess --chain cess-testnet --scheme Sr25519  --key-type rrsc --suri /root/secretKey.txt

./target/release/cess-node key insert --base-path /tmp/cess --chain cess-testnet --scheme Ed25519  --key-type gran --suri /root/secretKey.txt
```

Now you can launch node again:

```
# start
./target/release/cess-node --base-path /tmp/cess --chain cess-testnet
```

### Run in Docker

Install [Docker](https://docs.docker.com/get-docker/) first, and run the following command to start a node on the cess-testnet:

```
docker pull cesstech/cess-testnet:v0.1.1
docker run --network host cesstech/cess-testnet:v0.1.1 ./cess/target/release/cess-node --base-path /tmp/cess --chain cess-testnet
```

## Storage Mining

CESS supports to obtain incentives by contributing idle storage with [storage mining tool](https://github.com/CESSProject/storage-mining-tool), and click [here](https://github.com/CESSProject/cess/tree/v0.1.1/docs/designs-of-storage-mining.md) to learn more.

## Run Tests


CESS has Rust unit tests, and can be run locally.

```
# Run all the Rust unit tests
cargo test --release
```

## Module Documentation


* [Files Bank](https://github.com/CESSProject/cess/tree/v0.1.1/c-pallets/file-bank)
* [Segment Book](https://github.com/CESSProject/cess/tree/v0.1.1/c-pallets/segment-book)
* [Sminer](https://github.com/CESSProject/cess/tree/v0.1.1/c-pallets/sminer)

## Contribute


Please follow the contributions guidelines as outlined in [`docs/CONTRIBUTING.adoc`](https://github.com/CESSProject/cess/tree/v0.1.1/docs/CONTRIBUTING.adoc). In all communications and contributions, this project follows the [Contributor Covenant Code of Conduct](https://github.com/paritytech/substrate/blob/master/docs/CODE_OF_CONDUCT.md).
