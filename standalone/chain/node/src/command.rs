// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::Arc;
use futures::TryFutureExt;
// Substrate
use sc_cli::SubstrateCli;
use sc_service::DatabaseSource;
// Frontier
use fc_db::kv::frontier_database_dir;
use crate::{
	chain_spec,
	cli::{Cli, Subcommand},
	eth::db_config_dir,
	service,
};
use cess_node_runtime::{Block};

#[cfg(feature = "runtime-benchmarks")]
use crate::chain_spec::get_account_id_from_seed;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"CESS Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"support.anonymous.an".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
		let spec = match id {
			"" | "cess-testnet" => Box::new(chain_spec::cess_testnet_config()),
			"cess-initial-devnet" => Box::new(chain_spec::cess_testnet_generate_config()),
			"cess-devnet" => Box::new(chain_spec::cess_develop_config()),
			"cess-initial-testnet" => Box::new(chain_spec::cess_main()),
			"dev" => Box::new(chain_spec::development_config()),
			"local" => Box::new(chain_spec::local_testnet_config()),
			path =>
				Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?),
		};
		Ok(spec)
	}

}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::build_full(config, cli.eth, cli.sealing)
					.map_err(Into::into)
					.await
			})
		},
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::Sign(cmd)) => cmd.run(),
		Some(Subcommand::Verify(cmd)) => cmd.run(),
		Some(Subcommand::Vanity(cmd)) => cmd.run(),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, _, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, _, import_queue, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| {
				// Remove Frontier offchain db
				let db_config_dir = db_config_dir(&config);
				match cli.eth.frontier_backend_type {
					crate::eth::BackendType::KeyValue => {
						let frontier_database_config = match config.database {
							DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
								path: frontier_database_dir(&db_config_dir, "db"),
								cache_size: 0,
							},
							DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
								path: frontier_database_dir(&db_config_dir, "paritydb"),
							},
							_ => {
								return Err(format!(
									"Cannot purge `{:?}` database",
									config.database
								)
								.into())
							}
						};
						cmd.run(frontier_database_config)?;
					}
					crate::eth::BackendType::Sql => {
						let db_path = db_config_dir.join("sql");
						match std::fs::remove_dir_all(&db_path) {
							Ok(_) => {
								println!("{:?} removed.", &db_path);
							}
							Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => {
								eprintln!("{:?} did not exist.", &db_path);
							}
							Err(err) => {
								return Err(format!(
									"Cannot purge `{:?}` database: {:?}",
									db_path, err,
								)
								.into())
							}
						};
					}
				};
				cmd.run(config.database)
			})
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|mut config| {
				let (client, backend, _, task_manager, _) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				let aux_revert = {
					let backend = backend.clone();
					Box::new(move |client: Arc<crate::client::Client>, _, blocks| {
						cessc_consensus_rrsc::revert(client.clone(), backend, blocks)?;
						grandpa::revert(client, blocks)?;
						Ok(())
					})
				};
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		#[cfg(not(feature = "runtime-benchmarks"))]		
		Some(Subcommand::Benchmark) => Err("Benchmarking wasn't enabled when building the node. \
			You can enable it with `--features runtime-benchmarks`."
			.into()),
		#[cfg(feature = "runtime-benchmarks")]
		Some(Subcommand::Benchmark(cmd)) => {
			use crate::benchmarking::{
				inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder,
			};
			use frame_benchmarking_cli::{
				BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE,
			};			

			let runner = cli.create_runner(cmd)?;
			match cmd {
				BenchmarkCmd::Pallet(cmd) => runner.sync_run(|config| cmd.run::<Block, ()>(config)),
				BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
					let (client, _, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
					cmd.run(client)
				}),
				BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
					let (client, backend, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
					let db = backend.expose_db();
					let storage = backend.expose_storage();
					cmd.run(config, client, db, storage)
				}),
				BenchmarkCmd::Overhead(cmd) => runner.sync_run(|mut config| {
					let (client, _, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
					let ext_builder = RemarkBuilder::new(client.clone());
					cmd.run(
						config,
						client,
						inherent_benchmark_data()?,
						Vec::new(),
						&ext_builder,
					)
				}),
				BenchmarkCmd::Extrinsic(cmd) => runner.sync_run(|mut config| {
					let (client, _, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
					// Register the *Remark* and *TKA* builders.
					let ext_factory = ExtrinsicFactory(vec![
						Box::new(RemarkBuilder::new(client.clone())),
						Box::new(TransferKeepAliveBuilder::new(
							client.clone(),
							get_account_id_from_seed::<sp_core::ecdsa::Public>("Alice"),
							ExistentialDeposit::get(),
						)),
					]);

					cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
				}),
				BenchmarkCmd::Machine(cmd) => {
					runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
				}
			}
		},
		Some(Subcommand::FrontierDb(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|mut config| {
				let (client, _, _, _, frontier_backend) =
					service::new_chain_ops(&mut config, &cli.eth)?;
				let frontier_backend = match frontier_backend {
					fc_db::Backend::KeyValue(kv) => std::sync::Arc::new(kv),
					_ => panic!("Only fc_db::Backend::KeyValue supported"),
				};
				cmd.run(client, frontier_backend)
			})
		},
		#[cfg(feature = "try-runtime")]
		Some(Subcommand::TryRuntime(cmd)) => {
			use sc_executor::{sp_wasm_interface::ExtendedHostFunctions, NativeExecutionDispatch};
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				// we don't need any of the components of new_partial, just a runtime, or a task
				// manager to do `async_run`.
				let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
				let task_manager =
					sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
						.map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

				Ok((
					cmd.run::<Block, ExtendedHostFunctions<
						sp_io::SubstrateHostFunctions,
						<ExecutorDispatch as NativeExecutionDispatch>::ExtendHostFunctions,
					>>(),
					task_manager,
				))
			})
		},
		#[cfg(not(feature = "try-runtime"))]
		Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
				You can enable it with `--features try-runtime`."
			.into()),
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
	}
}
