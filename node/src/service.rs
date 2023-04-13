// This file is part of Substrate.

pub use crate::executor::ExecutorDispatch;
use crate::{
	primitives as node_primitives,
	rpc::{self as node_rpc, EthConfiguration},
};
use cess_node_runtime::{
	AccountId, Balance, Hash, Index as Nonce, RuntimeApi, TransactionConverter,
};
use cessc_consensus_rrsc::{self, SlotProportion};
use fc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use fc_rpc::EthTask;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use futures::prelude::*;
use node_primitives::Block;
use sc_cli::SubstrateCli;
use sc_client_api::{BlockBackend, BlockchainEvents};
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_network_common::{protocol::event::Event, service::NetworkEventStream};
use sc_service::{
	config::Configuration, error::Error as ServiceError, BasePath, PartialComponents, RpcHandlers,
	TFullBackend, TFullClient, TaskManager,
};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker, TelemetryWorkerHandle};
use sp_api::ConstructRuntimeApi;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::{BlakeTwo256, Block as BlockT};
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
	time::Duration,
};

use nimbus_consensus::{
	BuildNimbusConsensusParams, NimbusConsensus, NimbusManualSealConsensusDataProvider,
};

// Cumulus Imports
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_common::ParachainConsensus;
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_client_service::{
	prepare_node_config, start_collator, start_full_node, StartCollatorParams, StartFullNodeParams,
};
use cumulus_primitives_core::ParaId;
use cumulus_primitives_parachain_inherent::{
	MockValidationDataInherentDataProvider, MockXcmConfig,
};
use cumulus_relay_chain_inprocess_interface::build_inprocess_relay_chain;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface, RelayChainResult};
use cumulus_relay_chain_minimal_node::build_minimal_relay_chain_node;

use polkadot_service::CollatorPair;

use substrate_prometheus_endpoint::Registry;

/// The full client type definition.
pub type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
	grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

/// The transaction pool type defintion.
pub type TransactionPool = sc_transaction_pool::FullPool<Block, FullClient>;

pub struct CessRuntimeExecutor;

impl sc_executor::NativeExecutionDispatch for CessRuntimeExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		cess_node_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		cess_node_runtime::native_version()
	}
}


/// Fetch the nonce of the given `account` from the chain state.
///
/// Note: Should only be used for tests.
// pub fn fetch_nonce(client: &FullClient, account: sp_core::sr25519::Pair) -> u32 {
// 	let best_hash = client.chain_info().best_hash;
// 	client
// 		.runtime_api()
// 		.account_nonce(&generic::BlockId::Hash(best_hash), account.public().into())
// 		.expect("Fetching account nonce works; qed")
// }

pub fn frontier_database_dir(config: &Configuration) -> std::path::PathBuf {
	let config_dir = config
		.base_path
		.as_ref()
		.map(|base_path| base_path.config_dir(config.chain_spec.id()))
		.unwrap_or_else(|| {
			BasePath::from_project("", "", &crate::cli::Cli::executable_name())
				.config_dir(config.chain_spec.id())
		});
	config_dir.join("frontier").join("db")
}

pub fn open_frontier_backend<C: sp_blockchain::HeaderBackend<Block>>(
	client: Arc<C>,
	config: &Configuration,
) -> Result<Arc<fc_db::Backend<Block>>, String> {
	Ok(Arc::new(fc_db::Backend::<Block>::new(
		client,
		&fc_db::DatabaseSettings {
			source: fc_db::DatabaseSource::RocksDb {
				path: frontier_database_dir(&config),
				cache_size: 0,
			},
		},
	)?))
}

/// Creates a new partial node.
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		sc_consensus::DefaultImportQueue<Block, FullClient>,
		sc_transaction_pool::FullPool<Block, FullClient>,
		(
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>,
		),
	>,
	ServiceError,
> {
	let telemetry = config
		.telemetry_endpoints
		.clone()
		.filter(|x| !x.is_empty())
		.map(|endpoints| -> Result<_, sc_telemetry::Error> {
			let worker = TelemetryWorker::new(16)?;
			let telemetry = worker.handle().new_telemetry(endpoints);
			Ok((worker, telemetry))
		})
		.transpose()?;

	let executor = NativeElseWasmExecutor::<ExecutorDispatch>::new(
		config.wasm_method,
		config.default_heap_pages,
		config.max_runtime_instances,
		config.runtime_cache_size,
	);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry_worker_handle = telemetry.as_ref().map(|(worker, _)| worker.handle());

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	
	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	let import_queue = nimbus_consensus::import_queue(
		client.clone(),
		client.clone(),
		move |_, _| async move {
			let time = sp_timestamp::InherentDataProvider::from_system_time();

			Ok((time,))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry().clone(),
		true,
	)?;

	Ok(sc_service::PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		task_manager,
		transaction_pool,
		select_chain,
		other: (telemetry, telemetry_worker_handle),
	})
}

async fn build_relay_chain_interface(
	polkadot_config: Configuration,
	parachain_config: &Configuration,
	telemetry_worker_handle: Option<TelemetryWorkerHandle>,
	task_manager: &mut TaskManager,
	collator_options: CollatorOptions,
) -> RelayChainResult<(Arc<(dyn RelayChainInterface + 'static)>, Option<CollatorPair>)> {
	if !collator_options.relay_chain_rpc_urls.is_empty() {
		build_minimal_relay_chain_node(
			polkadot_config,
			task_manager,
			collator_options.relay_chain_rpc_urls,
		)
		.await
	} else {
		build_inprocess_relay_chain(
			polkadot_config,
			parachain_config,
			telemetry_worker_handle,
			task_manager,
			None,
		)
	}
}

/// Creates a full service from the configuration.
pub async fn start_node_impl<RuntimeApi, Executor, RB, BIC>(
	mut parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	eth_config: EthConfiguration,
	disable_hardware_benchmarks: bool,
	id: ParaId,
	_rpc_ext_builder: RB,
	build_consensus: BIC,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
)>
where
	RuntimeApi: ConstructRuntimeApi<Block, TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>
		+ Send
		+ Sync
		+ 'static,
	RuntimeApi::RuntimeApi: sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
		+ sp_api::Metadata<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_api::ApiExt<
			Block,
			StateBackend = sc_client_api::StateBackendFor<TFullBackend<Block>, Block>,
		> + sp_offchain::OffchainWorkerApi<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ cumulus_primitives_core::CollectCollationInfo<Block>
		+ pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>
		+ substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	sc_client_api::StateBackendFor<TFullBackend<Block>, Block>: sp_api::StateBackend<BlakeTwo256>,
	Executor: sc_executor::NativeExecutionDispatch + 'static,
	RB: Fn(
			Arc<TFullClient<Block, RuntimeApi, Executor>>,
		) -> Result<crate::rpc::RpcExtension, sc_service::Error>
		+ Send
		+ 'static,
	BIC: FnOnce(
		Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>>,
		Arc<sc_client_db::Backend<Block>>,
		Option<&Registry>,
		Option<TelemetryHandle>,
		&TaskManager,
		Arc<dyn RelayChainInterface>,
		Arc<
			sc_transaction_pool::FullPool<
				Block,
				TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>,
			>,
		>,
		Arc<NetworkService<Block, Hash>>,
		SyncCryptoStorePtr,
		bool,
	) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error>,
{
	let hwbench = if !disable_hardware_benchmarks {
		parachain_config.database.path().map(|database_path| {
			let _ = std::fs::create_dir_all(&database_path);
			sc_sysinfo::gather_hwbench(Some(database_path))
		})
	} else {
		None
	};

	let parachain_config = prepare_node_config(parachain_config);

	let sc_service::PartialComponents {
		backend,
		client,
		import_queue,
		keystore_container,
		mut task_manager,
		transaction_pool,
		select_chain,
		other: (mut telemetry, telemetry_worker_handle),
	} = new_partial(&parachain_config)?;

	let (relay_chain_interface, collator_key) = build_relay_chain_interface(
		polkadot_config,
		&parachain_config,
		telemetry_worker_handle,
		&mut task_manager,
		collator_options.clone(),
	)
	.await
	.map_err(|e| match e {
		RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
		s => s.to_string().into(),
	})?;

	let block_announce_validator = BlockAnnounceValidator::new(relay_chain_interface.clone(), id);

	let auth_disc_publish_non_global_ips = parachain_config.network.allow_non_globals_in_dht;

	let (network, system_rpc_tx, tx_handler_controller, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &parachain_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: Some(Box::new(|_| {
				Box::new(block_announce_validator)
			})),
			warp_sync: None,
		})?;

	let network_clone = network.clone();
	let frontier_backend = open_frontier_backend(client.clone(), &parachain_config)?;
	let fee_history_cache: FeeHistoryCache = Arc::new(Mutex::new(BTreeMap::new()));
	let fee_history_limit = eth_config.fee_history_limit;
	let max_past_logs = eth_config.max_past_logs;
	let enable_dev_signer = eth_config.enable_dev_signer;
	let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;

	let overrides = crate::rpc::overrides_handle(client.clone());
	let filter_pool: Option<FilterPool> = Some(Arc::new(Mutex::new(BTreeMap::new())));

	let rpc_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.sync_keystore();
		let chain_spec = parachain_config.chain_spec.cloned_box();
		let is_authority = parachain_config.role.is_authority();
		let filter_pool = filter_pool.clone();
		let frontier_backend = frontier_backend.clone();
		let fee_history_cache = fee_history_cache.clone();
		let fee_history_limit = fee_history_limit.clone();
		let overrides = overrides.clone();
		let prometheus_registry = parachain_config.prometheus_registry().cloned();
		let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			overrides.clone(),
			50,
			50,
			prometheus_registry.clone(),
		));

		let rpc_backend = backend.clone();
		
		move |deny_unsafe, subscription_task_executor: SubscriptionTaskExecutor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
				graph: pool.pool().clone(),
				converter: Some(TransactionConverter),
				is_authority,
				enable_dev_signer: enable_dev_signer.clone(),
				network: network_clone.clone(),
				filter_pool: filter_pool.clone(),
				frontier_backend: frontier_backend.clone(),
				backend: backend.clone(),
				max_past_logs: max_past_logs.clone(),
				fee_history_limit: fee_history_limit.clone(),
				fee_history_cache: fee_history_cache.clone(),
				block_data_cache: block_data_cache.clone(),
				overrides: overrides.clone(),
				execute_gas_limit_multiplier: execute_gas_limit_multiplier.clone(),
			};

			node_rpc::create_full(deps, subscription_task_executor.clone(), rpc_backend.clone()).map_err(Into::into)
		}
	};

	if parachain_config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&parachain_config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let role = parachain_config.role.clone();
	let force_authoring = parachain_config.force_authoring;
	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = parachain_config.network.node_name.clone();
	let enable_grandpa = !parachain_config.disable_grandpa;
	let prometheus_registry = parachain_config.prometheus_registry().cloned();

	let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config: parachain_config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		network: network.clone(),
		rpc_builder: Box::new(rpc_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		system_rpc_tx,
		tx_handler_controller,
		telemetry: telemetry.as_mut(),
	})?;

	task_manager.spawn_essential_handle().spawn(
		"frontier-mapping-sync-worker",
		None,
		MappingSyncWorker::new(
			client.import_notification_stream(),
			Duration::new(6, 0),
			client.clone(),
			backend.clone(),
			frontier_backend.clone(),
			3,
			0,
			SyncStrategy::Normal,
		)
		.for_each(|()| futures::future::ready(())),
	);

	// Spawn Frontier EthFilterApi maintenance task.
	if let Some(filter_pool) = filter_pool {
		// Each filter is allowed to stay in the pool for 100 blocks.
		const FILTER_RETAIN_THRESHOLD: u64 = 100;
		task_manager.spawn_essential_handle().spawn(
			"frontier-filter-pool",
			Some("frontier"),
			EthTask::filter_pool_task(client.clone(), filter_pool, FILTER_RETAIN_THRESHOLD),
		);
	}

	// Spawn Frontier FeeHistory cache maintenance task.
	task_manager.spawn_essential_handle().spawn(
		"frontier-fee-history",
		Some("frontier"),
		EthTask::fee_history_task(client.clone(), overrides, fee_history_cache, fee_history_limit),
	);

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);

		if let Some(ref mut telemetry) = telemetry {
			let telemetry_handle = telemetry.handle();
			task_manager.spawn_handle().spawn(
				"telemetry_hwbench",
				None,
				sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
			);
		}
	}

	if let sc_service::config::Role::Authority { .. } = &role {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let client_clone = client.clone();

	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role =
			sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network.event_stream("authority-discovery").filter_map(|e| async move {
				match e {
					Event::Dht(e) => Some(e),
					_ => None,
				}
			});
		let (authority_discovery_worker, _service) =
			sc_authority_discovery::new_worker_and_service_with_config(
				sc_authority_discovery::WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					..Default::default()
				},
				client.clone(),
				network.clone(),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
			);

		task_manager.spawn_handle().spawn(
			"authority-discovery-worker",
			Some("networking"),
			authority_discovery_worker.run(),
		);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore =
		if role.is_authority() { Some(keystore_container.sync_keystore()) } else { None };


	network_starter.start_network();
	Ok((task_manager, client))
}

/// Builds a new service for a full client.
pub async fn start_parachain_node(
	config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	eth_config: EthConfiguration,
	disable_hardware_benchmarks: bool,
	id: ParaId,
) -> sc_service::error::Result<(
	TaskManager,
	Arc<TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<CessRuntimeExecutor>>>,
)> {
	start_node_impl(
		config,
		polkadot_config,
		collator_options,
		eth_config,
		disable_hardware_benchmarks,
		id,
		|_| Ok(crate::rpc::RpcExtension::new(())),
		|client,
		 backend,
		 prometheus_registry,
		 telemetry,
		 task_manager,
		 relay_chain_interface,
		 transaction_pool,
		 _sync_oracle,
		 keystore,
		 force_authoring| {
			let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
				task_manager.spawn_handle(),
				client.clone(),
				transaction_pool,
				prometheus_registry,
				telemetry.clone(),
			);

			Ok(NimbusConsensus::build(BuildNimbusConsensusParams {
				para_id: id,
				proposer_factory,
				block_import: client.clone(),
				backend,
				parachain_client: client.clone(),
				keystore,
				skip_prediction: force_authoring,
				create_inherent_data_providers: move |_,
				                                      (
					relay_parent,
					validation_data,
					_author_id,
				)| {
					let relay_chain_interface = relay_chain_interface.clone();
					async move {
						let parachain_inherent =
							cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
								relay_parent,
								&relay_chain_interface,
								&validation_data,
								id,
							).await;

						let time = sp_timestamp::InherentDataProvider::from_system_time();

						let parachain_inherent = parachain_inherent.ok_or_else(|| {
							Box::<dyn std::error::Error + Send + Sync>::from(
								"Failed to create parachain inherent",
							)
						})?;

						let nimbus_inherent = nimbus_primitives::InherentDataProvider;

						Ok((time, parachain_inherent, nimbus_inherent))
					}
				},
				additional_digests_provider: (),
			}))
		},
	)
	.await
}
