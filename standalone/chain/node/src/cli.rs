use crate::eth::EthConfiguration;
use polkadot_sdk::*;

/// An overarching CLI command definition.
#[derive(Debug, clap::Parser)]
pub struct Cli {
	/// Possible subcommand with parameters.
	#[command(subcommand)]
	pub subcommand: Option<Subcommand>,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub run: sc_cli::RunCmd,

	#[command(flatten)]
	pub eth: EthConfiguration,
	/// Disable automatic hardware benchmarks.
	///
	/// By default these benchmarks are automatically ran at startup and measure
	/// the CPU speed, the memory bandwidth and the disk speed.
	///
	/// The results are then printed out in the logs, and also sent as part of
	/// telemetry, if telemetry is enabled.
	#[arg(long)]
	pub no_hardware_benchmarks: bool,

	#[allow(missing_docs)]
	#[clap(flatten)]
	pub storage_monitor: sc_storage_monitor::StorageMonitorParams,
}

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
	/// The custom inspect subcommand for decoding blocks and extrinsics.
	#[command(
		name = "inspect",
		about = "Decode given block or extrinsic using current native runtime."
	)]
	Inspect(staging_node_inspect::cli::InspectCmd),

	/// Sub-commands concerned with benchmarking.
	/// The pallet benchmarking moved to the `pallet` sub-command.
	#[command(subcommand)]
	Benchmark(frame_benchmarking_cli::BenchmarkCmd),

	/// Key management cli utilities
	#[command(subcommand)]
	Key(sc_cli::KeySubcommand),

	/// Verify a signature for a message, provided on STDIN, with a given (public or secret) key.
	Verify(sc_cli::VerifyCmd),

	/// Generate a seed that provides a vanity address.
	Vanity(sc_cli::VanityCmd),

	/// Sign a message, with a given (secret) key.
	Sign(sc_cli::SignCmd),

	/// Build a chain specification.
	BuildSpec(sc_cli::BuildSpecCmd),

	/// Validate blocks.
	CheckBlock(sc_cli::CheckBlockCmd),

	/// Export blocks.
	ExportBlocks(sc_cli::ExportBlocksCmd),

	/// Export the state of a given block into a chain spec.
	ExportState(sc_cli::ExportStateCmd),

	/// Import blocks.
	ImportBlocks(sc_cli::ImportBlocksCmd),

	/// Remove the whole chain.
	PurgeChain(sc_cli::PurgeChainCmd),

	/// Revert the chain to a previous state.
	Revert(sc_cli::RevertCmd),

	/// Db meta columns information.
	ChainInfo(sc_cli::ChainInfoCmd),

	/// Db meta columns information.
	FrontierDb(fc_cli::FrontierDbCmd),
}
