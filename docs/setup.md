---
title: Setup
---

This page will guide you through the **2 steps** needed to prepare a computer for **CESS** development.
Since Substrate is built with [the Rust programming language](https://www.rust-lang.org/), the first
thing you will need to do is prepare the computer for Rust development - these steps will vary based
on the computer's operating system. Once Rust is configured, you will use its toolchains to interact
with Rust projects; the commands for Rust's toolchains will be the same for all supported,
Unix-based operating systems.

## 1. Build Dependencies 

CESS development is easiest on Unix-based operating systems like macOS or Linux.


### Ubuntu/Debian

Use a terminal shell to execute the following commands:

```bash
sudo apt update
# May prompt for location information
sudo apt install --assume-yes git clang curl libssl-dev llvm libudev-dev make protobuf-compiler
```

### Arch Linux

Run these commands from a terminal:

```bash
pacman -Syu --needed --noconfirm curl git clang make protobuf
```

### Fedora

Run these commands from a terminal:

```bash
sudo dnf update
sudo dnf install clang curl git openssl-devel make protobuf-compiler
```

### OpenSUSE

Run these commands from a terminal:

```bash
sudo zypper install clang curl git openssl-devel llvm-devel libudev-devel make protobuf
```

### macOS

> NOTE: The Apple M1 ARM system on a chip is not very well supported yet by rust,
> and thus you very likely will run into build errors stemming from this.

Open the Terminal application and execute the following commands:

```bash
# Install Homebrew if necessary https://brew.sh/
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"

# Make sure Homebrew is up-to-date, install openssl
brew update
brew install openssl
```
### Windows 

> **Note:** Native development of Substrate is _not_ very well supported! It is _highly_ recommend to
> use [Windows Subsystem Linux](https://docs.microsoft.com/en-us/windows/wsl/install-win10) (WSL)
> and follow the instructions for [Ubuntu/Debian](#ubuntudebian).

Please refer to the separate [guide for native Windows development](windows-users).

## 2. Rust Developer Environment

### Automated `getsubstrate.io` Script

For most users you can run our script to automate the steps listed below:

```bash
curl https://getsubstrate.io -sSf | bash -s -- --fast
```

If this gives any errors, please follow the steps below to manually configure rust on your machine.

### Manual Rust Configuration

This guide uses [`rustup`](https://rustup.rs/) to help manage the Rust toolchain. First install and
configure `rustup`:

```bash
# Install
curl https://sh.rustup.rs -sSf | sh
# Configure
source ~/.cargo/env
```

Configure the Rust toolchain to default to the latest stable version, add nightly and the nightly wasm target:

```bash
rustup default stable
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

### Rust Configuration Check

To see what Rust toolchain you are presently using, run:

```bash
rustup show
```

This will show something like this (Ubuntu example) output:

```text
Default host: x86_64-unknown-linux-gnu
rustup home:  /home/user/.rustup

installed toolchains
--------------------

stable-x86_64-unknown-linux-gnu (default)
nightly-2020-10-06-x86_64-unknown-linux-gnu
nightly-x86_64-unknown-linux-gnu

installed targets for active toolchain
--------------------------------------

wasm32-unknown-unknown
x86_64-unknown-linux-gnu

active toolchain
----------------

stable-x86_64-unknown-linux-gnu (default)
rustc 1.50.0 (cb75ad5db 2021-02-10)
```

As you can see above, the default toolchain is stable, and the
`nightly-x86_64-unknown-linux-gnu` toolchain as well as its `wasm32-unknown-unknown` target is installed.
You also see that `nightly-2020-10-06-x86_64-unknown-linux-gnu` is installed, but is not used unless explicitly defined as illustrated in the [specify your nightly version](#specifying-nightly-version)
section.

### WebAssembly Compilation

Substrate uses [WebAssembly](https://webassembly.org/) (Wasm) to produce portable blockchain
runtimes. You will need to configure your Rust compiler to use
[`nightly` builds](https://doc.rust-lang.org/book/appendix-07-nightly-rust.html) to allow you to
compile Substrate runtime code to the Wasm target.

> There are upstream issues in Rust that need to be resolved before all of Substrate can use
> the stable Rust toolchain -
> [this is our tracking issue](https://github.com/paritytech/substrate/issues/1252)
> if you are curious as to why and how this may be resolved.

#### Latest Nightly for Substrate `master`

Developers who are building Substrate _itself_ should always use the latest bug-free versions of
Rust stable and nightly. This is because the Substrate codebase follows the tip of Rust nightly,
which means that changes in Substrate often depend on upstream changes in the Rust nightly compiler.
To ensure your Rust compiler is always up to date, you should run:

```bash
rustup update
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

> **It may be necessary to occasionally rerun `rustup update`** if a change in the upstream Substrate
> codebase depends on a new feature of the Rust compiler. When you do this, both your nightly
> and stable toolchains will be pulled to the most recent release, and for nightly, it is
> generally _not_ expected to compile WASM without error (although it very often does).
> be sure to [specify your nightly version](#specifying-nightly-version) if you get WASM build errors
> from `rustup` and [downgrade nightly as needed](#downgrading-rust-nightly).

#### Rust Nightly Toolchain

If you want to guarantee that your build works on your computer as you update Rust and other
dependencies, you should use a specific Rust nightly version that is known to be
compatible with the version of Substrate they are using; this version will vary from project to
project and different projects may use different mechanisms to communicate this version to
developers. For instance, the Polkadot client specifies this information in its
[release notes](https://github.com/paritytech/polkadot/releases).

```bash
# Specify the specific nightly toolchain in the date below:
rustup install nightly-<yyyy-MM-dd>
```

#### Wasm Toolchain

Now, configure the nightly version to work with the Wasm compilation target:

```bash
rustup target add wasm32-unknown-unknown --toolchain nightly-<yyyy-MM-dd>
```

#### Specifying Nightly Version

Use the `WASM_BUILD_TOOLCHAIN` environment variable to specify the Rust nightly version a Substrate
project should use for Wasm compilation:

```bash
WASM_BUILD_TOOLCHAIN=nightly-<yyyy-MM-dd> cargo build --release
```

> Note that this only builds _the runtime_ with the specified nightly. The rest of project will be
> compiled with **your default toolchain**, i.e. the latest installed stable toolchain.

#### Downgrading Rust Nightly

If your computer is configured to use the latest Rust nightly and you would like to downgrade to a
specific nightly version, follow these steps:

```sh
rustup uninstall nightly
rustup install nightly-<yyyy-MM-dd>
rustup target add wasm32-unknown-unknown --toolchain nightly-<yyyy-MM-dd>
```


***Reference： [here](https://substrate.dev/docs/en/knowledgebase/getting-started/)***
