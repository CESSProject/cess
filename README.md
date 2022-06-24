# ![CESS](https://raw.githubusercontent.com/Cumulus2021/W3F-illustration/main/banner5.png)

[![Substrate version](https://img.shields.io/badge/Substrate-3.0.0-blue?logo=Parity%20Substrate)](https://substrate.dev/) [![GitHub license](https://img.shields.io/badge/license-GPL3%2FApache2-blue)](#LICENSE)
[![build-and-test](https://github.com/CESSProject/cess/actions/workflows/build-and-test.yml/badge.svg)](https://github.com/CESSProject/cess/actions/workflows/build-and-test.yml)


<a href='https://web3.foundation/'><img width='205' alt='web3f_grants_badge.png' src='https://github.com/heyworld88/gitskills/blob/main/web3f_grants_badge.png'></a>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;<a href='https://builders.parity.io/'><img width='240' src='https://github.com/heyworld88/gitskills/blob/main/sbp_grants_badge.png'></a>

  
**[cess.cloud](http://cess.cloud/) is to provide the capabilities of a new global decentralized cloud data storage network by building with the infrastructure of decentralized cloud data network of the [substrate](https://substrate.dev/) while maintaining the data security and reliability guarantees inherent to blockchain technology. Learn more at [white-paper](https://github.com/CESSProject/Whitepaper).** 

## Getting Started


### Install the Guide

Follow [Setup](https://github.com/CESSProject/cess/tree/main/docs/setup.md) to guide you install the CESS development.

### Build the Node

The `cargo run` command will perform an initial build. Use the following command to build the node without launching it:

```
# Fetch the code
git clone https://github.com/CESSProject/cess.git
cd cess

# Build the node (The first build will be long (~30min))
cargo build --release
```

## Run the CESS Node


After the node has finished compiling, you can follow these steps below to run it. 

### Generate Keys

If you already have the keys for Substrate using the [SS58 address encoding format](https://docs.substrate.io/v3/advanced/ss58/), please see the next section.

Begin by compiling and installing the utility ([instructions and more info here](https://substrate.dev/docs/en/knowledgebase/integrate/subkey)). 

Generate a mnemonic (phrase seed) and see the `sr25519` key and address associated with it.

```
# subkey command
subkey generate --scheme sr25519
```

Now see the `ed25519` key and address associated with the same mnemonic (phrase seed).

```
# subkey command
subkey inspect --scheme ed25519 "SECRET PHRASE YOU JUST GENERATED"
```

We recommend a point that you record the above outputs and keep mnemonic in safe.

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
./target/release/cess-node key insert --base-path /tmp/cess --chain cess-testnet --scheme Sr25519  --key-type babe --suri /root/secretKey.txt

./target/release/cess-node key insert --base-path /tmp/cess --chain cess-testnet --scheme Ed25519  --key-type gran --suri /root/secretKey.txt
```

Now you can launch node again:

```
# start
./target/release/cess-node --base-path /tmp/cess --chain cess-testnet
```

## Storage Mining

CESS supports to obtain incentives by contributing idle storage with [storage mining tool](https://github.com/CESSProject/storage-mining-tool), and click [here](https://github.com/CESSProject/cess/tree/main/docs/designs-of-storage-mining.md) to learn more.

## Run Tests


CESS has Rust unit tests, and enable to run locally.

```
# Run all the Rust unit tests
cargo test --release
```

## Module Documentation link


* [Files Bank](https://github.com/CESSProject/cess/tree/main/c-pallets/file-bank)
* [Segment Book](https://github.com/CESSProject/cess/tree/main/c-pallets/segment-book)
* [Sminer](https://github.com/CESSProject/cess/tree/main/c-pallets/sminer)
* [File Map](https://github.com/CESSProject/cess/tree/main/c-pallets/file-map)

## Contribution

Please follow the contributions guidelines as outlined in [`docs/CONTRIBUTING.adoc`](https://github.com/CESSProject/cess/tree/main/docs/CONTRIBUTING.adoc). In all communications and contributions, this project follows the [Contributor Covenant Code of Conduct](https://github.com/paritytech/substrate/blob/master/docs/CODE_OF_CONDUCT.md).

## cess-bootstrap

Please download the code of the current latest release
```
cd cess
#Under the /cess root directory
cargo build --release
```
Go to the /cess/target/release directory to obtain the cess node file
