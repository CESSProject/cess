//! Substrate Node Template CLI library.
#![warn(missing_docs)]
#![allow(
	clippy::type_complexity,
	clippy::too_many_arguments,
	clippy::large_enum_variant
)]
#![cfg_attr(feature = "runtime-benchmarks", deny(unused_crate_dependencies))]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod chain_spec;
mod service;
mod cli;
mod command;
mod rpc;
mod eth;
mod client;

fn main() -> sc_cli::Result<()> {
	command::run()
}
