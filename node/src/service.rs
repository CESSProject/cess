// This file is part of Substrate.

pub use crate::executor::ExecutorDispatch;
use crate::{
	primitives as node_primitives, 
	rpc::{self as node_rpc, EthConfiguration}
};
use cess_node_runtime::{opaque::Block, Hash, RuntimeApi, TransactionConverter};
use fc_mapping_sync::{MappingSyncWorker, SyncStrategy};
use fc_rpc::EthTask;
use fc_rpc_core::types::{
	FeeHistoryCache, 
	FilterPool
};
use futures::prelude::*;
use sc_cli::SubstrateCli;
use sc_client_api::BlockchainEvents;
use sc_executor::NativeElseWasmExecutor;
use sc_network::NetworkService;
use sc_network_common::{
	protocol::event::Event, 
	service::{
		NetworkBlock,
		NetworkEventStream
	}
};
use sc_consensus::ImportQueue;
use sc_service::{ BasePath, config::Configuration, error::Error as ServiceError, RpcHandlers, TaskManager};
use sc_telemetry::{Telemetry, TelemetryHandle , TelemetryWorker, TelemetryWorkerHandle};
use sp_keystore::SyncCryptoStorePtr;
// use sp_runtime::{
// 	traits::Block as BlockT, 
// };
use std::{
	collections::BTreeMap,
	sync::{Arc, Mutex},
	time::Duration,
};
use substrate_prometheus_endpoint::Registry;

// Cumulus Imports
use cumulus_client_consensus_aura::{AuraConsensus, BuildAuraConsensusParams, SlotProportion};
use cumulus_client_cli::CollatorOptions;
use cumulus_client_consensus_common::{
	ParachainBlockImport as TParachainBlockImport, ParachainConsensus,
};
use cumulus_client_network::BlockAnnounceValidator;
use cumulus_primitives_core::ParaId;
use cumulus_relay_chain_interface::{RelayChainError, RelayChainInterface};

/// The full client type definition.
// pub type FullClient =
// 	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
// type FullBackend = sc_service::TFullBackend<Block>;
// type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
// type FullGrandpaBlockImport =
// 	grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;


/// Native executor type.
pub struct ParachainNativeExecutor;

impl sc_executor::NativeExecutionDispatch for ParachainNativeExecutor {
	type ExtendHostFunctions = frame_benchmarking::benchmarking::HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		cess_node_runtime::api::dispatch(method, data)
	}

	fn native_version() -> sc_executor::NativeVersion {
		cess_node_runtime::native_version()
	}
}

type ParachainExecutor = NativeElseWasmExecutor<ParachainNativeExecutor>;

type ParachainClient = sc_service::TFullClient<Block, RuntimeApi, ParachainExecutor>;

type ParachainBackend = sc_service::TFullBackend<Block>;

type ParachainBlockImport = TParachainBlockImport<Block, Arc<ParachainClient>, ParachainBackend>;

/// The transaction pool type defintion.
pub type TransactionPool = sc_transaction_pool::FullPool<Block, ParachainClient>;

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
	config: &Configuration) 
	-> Result<Arc<fc_db::Backend<Block>>, String> {
	Ok(Arc::new(fc_db::Backend::<Block>::new(
		client,
		&fc_db::DatabaseSettings {
			source: fc_db::DatabaseSource::RocksDb {
				path: frontier_database_dir(&config),
				cache_size: 0,
			},
		})?)
	)
}

/// Creates a new partial node.
pub fn new_partial(
	config: &Configuration,
) -> Result<
	sc_service::PartialComponents<
		ParachainClient,
		ParachainBackend,
		(),
		sc_consensus::DefaultImportQueue<Block, ParachainClient>,
		TransactionPool,
		(
			ParachainBlockImport,
			Option<Telemetry>,
			Option<TelemetryWorkerHandle>
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

	let executor = ParachainExecutor::new(
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

	// let select_chain = sc_consensus::LongestChain::new(backend.clone());
	// let (grandpa_block_import, grandpa_link) = grandpa::block_import(
	// 	client.clone(),
	// 	&(client.clone() as Arc<_>),
	// 	select_chain.clone(),
	// 	telemetry.as_ref().map(|x| x.handle()),
	// )?;

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);
	// let justification_import = grandpa_block_import.clone();

	let block_import = ParachainBlockImport::new(client.clone(), backend.clone());

	let import_queue = build_import_queue(
		client.clone(),
		block_import.clone(),
		config,
		telemetry.as_ref().map(|telemetry| telemetry.handle()),
		&task_manager,
	)?;

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain: (),
		import_queue,
		transaction_pool,
		other: (block_import, telemetry, telemetry_worker_handle),
	})
}

/// Build the import queue for the parachain runtime.
fn build_import_queue(
	client: Arc<ParachainClient>,
	block_import: ParachainBlockImport,
	config: &Configuration,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
) -> Result<sc_consensus::DefaultImportQueue<Block, ParachainClient>, sc_service::Error> {
	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

	cumulus_client_consensus_aura::import_queue::<
		sp_consensus_aura::sr25519::AuthorityPair,
		_,
		_,
		_,
		_,
		_,
	>(cumulus_client_consensus_aura::ImportQueueParams {
		block_import,
		client,
		create_inherent_data_providers: move |_, _| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot =
				sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			Ok((slot, timestamp))
		},
		registry: config.prometheus_registry(),
		spawner: &task_manager.spawn_essential_handle(),
		telemetry,
	})
	.map_err(Into::into)
}

fn build_consensus(
	client: Arc<ParachainClient>,
	block_import: ParachainBlockImport,
	prometheus_registry: Option<&Registry>,
	telemetry: Option<TelemetryHandle>,
	task_manager: &TaskManager,
	relay_chain_interface: Arc<dyn RelayChainInterface>,
	transaction_pool: Arc<sc_transaction_pool::FullPool<Block, ParachainClient>>,
	sync_oracle: Arc<NetworkService<Block, Hash>>,
	keystore: SyncCryptoStorePtr,
	force_authoring: bool,
	para_id: ParaId,
) -> Result<Box<dyn ParachainConsensus<Block>>, sc_service::Error> {
	let slot_duration = cumulus_client_consensus_aura::slot_duration(&*client)?;

	let proposer_factory = sc_basic_authorship::ProposerFactory::with_proof_recording(
		task_manager.spawn_handle(),
		client.clone(),
		transaction_pool,
		prometheus_registry,
		telemetry.clone(),
	);

	let params = BuildAuraConsensusParams {
		proposer_factory,
		create_inherent_data_providers: move |_, (relay_parent, validation_data)| {
			let relay_chain_interface = relay_chain_interface.clone();
			async move {
				let parachain_inherent =
					cumulus_primitives_parachain_inherent::ParachainInherentData::create_at(
						relay_parent,
						&relay_chain_interface,
						&validation_data,
						para_id,
					)
					.await;
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot =
						sp_consensus_aura::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

				let parachain_inherent = parachain_inherent.ok_or_else(|| {
					Box::<dyn std::error::Error + Send + Sync>::from(
						"Failed to create parachain inherent",
					)
				})?;
				Ok((slot, timestamp, parachain_inherent))
			}
		},
		block_import,
		para_client: client,
		backoff_authoring_blocks: Option::<()>::None,
		sync_oracle,
		keystore,
		force_authoring,
		slot_duration,
		// We got around 500ms for proposing
		block_proposal_slot_portion: SlotProportion::new(1f32 / 24f32),
		// And a maximum of 750ms if slots are skipped
		max_block_proposal_slot_portion: Some(SlotProportion::new(1f32 / 16f32)),
		telemetry,
	};

	Ok(AuraConsensus::build::<sp_consensus_aura::sr25519::AuthorityPair, _, _, _, _, _, _>(params))
}

/// Result of [`new_full_base`].
// pub struct NewFullBase {
// 	/// The task manager of the node.
// 	pub task_manager: TaskManager,
// 	/// The client instance of the node.
// 	pub client: Arc<ParachainClient>,
// 	/// The networking service of the node.
// 	pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
// 	/// The transaction pool of the node.
// 	pub transaction_pool: Arc<TransactionPool>,
// 	/// The rpc handlers of the node.
// 	pub rpc_handlers: RpcHandlers,
// }

/// Creates a full service from the configuration.
#[sc_tracing::logging::prefix_logs_with("Parachain")]
pub async fn new_full_base(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	eth_config: EthConfiguration,
	para_id: ParaId,
	disable_hardware_benchmarks: bool,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient>)> {
	let hwbench = if !disable_hardware_benchmarks {
		parachain_config.database.path().map(|database_path| {
			let _ = std::fs::create_dir_all(&database_path);
			sc_sysinfo::gather_hwbench(Some(database_path))
		})
	} else {
		None
	};

	let parachain_config = cumulus_client_service::prepare_node_config(parachain_config);
	
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain: (),
		transaction_pool,
		other: (block_import, mut telemetry, telemetry_worker_handle),
	} = new_partial(&parachain_config)?;
	
	let import_queue_service = import_queue.service();
	let (relay_chain_interface, collator_key) = cumulus_client_service::build_relay_chain_interface(
		polkadot_config,
		&parachain_config,
		telemetry_worker_handle,
		&mut task_manager,
		collator_options.clone(),
		hwbench.clone(),
	)
	.await
	.map_err(|e| match e {
		RelayChainError::ServiceError(polkadot_service::Error::Sub(x)) => x,
		s => s.to_string().into(),
	})?;

	let block_announce_validator =
		BlockAnnounceValidator::new(relay_chain_interface.clone(), para_id);


	let auth_disc_publish_non_global_ips = parachain_config.network.allow_non_globals_in_dht;
	// let grandpa_protocol_name = grandpa::protocol_standard_name(
	// 	&client.block_hash(0).ok().flatten().expect("Genesis block exists; qed"),
	// 	&parachain_config.chain_spec,
	// );

	// parachain_config
	// .network
	// .extra_sets
	// .push(grandpa::grandpa_peers_set_config(grandpa_protocol_name.clone()));
	
	// let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
	// 	backend.clone(),
	// 	import_setup.1.shared_authority_set().clone(),
	// 	Vec::default(),
	// ));

	let (network, system_rpc_tx, tx_handler_controller, start_network) =
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

	if parachain_config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&parachain_config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let force_authoring = parachain_config.force_authoring;
	let validator = parachain_config.role.is_authority();
	
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
		// let (_, _, rrsc_link) = &block_import;

		// let justification_stream = grandpa_link.justification_stream();
		// let shared_authority_set = grandpa_link.shared_authority_set().clone();
		// let shared_voter_state = grandpa::SharedVoterState::empty();
		// let shared_voter_state2 = shared_voter_state.clone();

		// let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
		// 	backend.clone(),
		// 	Some(shared_authority_set.clone()),
		// );

		// let rrsc_config = rrsc_link.config().clone();
		// let shared_epoch_changes = rrsc_link.epoch_changes().clone();

		let client = client.clone();
		let pool = transaction_pool.clone();
		// let select_chain = select_chain.clone();
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
		let rpc_builder = move |deny_unsafe, subscription_executor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				// select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				// rrsc: node_rpc::RRSCDeps {
				// 	rrsc_config: rrsc_config.clone(),
				// 	shared_epoch_changes: shared_epoch_changes.clone(),
				// 	keystore: keystore.clone(),
				// },
				// grandpa: node_rpc::GrandpaDeps {
				// 	shared_voter_state: shared_voter_state.clone(),
				// 	shared_authority_set: shared_authority_set.clone(),
				// 	justification_stream: justification_stream.clone(),
				// 	subscription_executor,
				// 	finality_provider: finality_proof_provider.clone(),
				// },
				graph: pool.pool().clone(),
				converter: Some(TransactionConverter),
				is_authority,
				enable_dev_signer: enable_dev_signer.clone(),
				network: network_clone.clone(),
				filter_pool: filter_pool.clone(),
				frontier_backend: frontier_backend.clone(),
				max_past_logs: max_past_logs.clone(),
				fee_history_limit: fee_history_limit.clone(),
				fee_history_cache: fee_history_cache.clone(),
				block_data_cache: block_data_cache.clone(),
				overrides: overrides.clone(),
				execute_gas_limit_multiplier: execute_gas_limit_multiplier.clone(),
				subscription_executor,
			};

			node_rpc::create_full(deps, rpc_backend.clone()).map_err(Into::into)
		};

		rpc_builder
	};

	// let shared_voter_state = rpc_setup;

	let role = parachain_config.role.clone();
	let force_authoring = parachain_config.force_authoring;
	// let backoff_authoring_blocks =
	// 	Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
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
		EthTask::fee_history_task(
			client.clone(),
			overrides,
			fee_history_cache,
			fee_history_limit,
		),
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

	// let (_, _, rrsc_link) = block_import;

	// (with_startup_data)(&block_import, &rrsc_link);

	if let sc_service::config::Role::Authority { .. } = &role {
		// let proposer = sc_basic_authorship::ProposerFactory::with_proof_recording(
		// 	task_manager.spawn_handle(),
		// 	client.clone(),
		// 	transaction_pool.clone(),
		// 	prometheus_registry.as_ref(),
		// 	telemetry.as_ref().map(|x| x.handle()),
		// );

		let client_clone = client.clone();
		// let slot_duration = rrsc_link.config().slot_duration();
		// let rrsc_config = cessc_consensus_rrsc::RRSCParams {
		// 	keystore: keystore_container.sync_keystore(),
		// 	client: client.clone(),
		// 	select_chain: (),
		// 	env: proposer,
		// 	block_import,
		// 	sync_oracle: network.clone(),
		// 	justification_sync_link: network.clone(),
		// 	create_inherent_data_providers: move |parent, ()| {
		// 		let client_clone = client_clone.clone();
		// 		async move {
		// 			let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
		// 				&*client_clone,
		// 				parent,
		// 			)?;

		// 			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

		// 			let slot =
		// 				cessp_consensus_rrsc::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
		// 					*timestamp,
		// 					slot_duration,
		// 				);

		// 			let storage_proof =
		// 				sp_transaction_storage_proof::registration::new_data_provider(
		// 					&*client_clone,
		// 					&parent,
		// 				)?;

		// 			Ok((slot, timestamp, uncles, storage_proof))
		// 		}
		// 	},
		// 	force_authoring,
		// 	backoff_authoring_blocks,
		// 	rrsc_link,
		// 	block_proposal_slot_portion: SlotProportion::new(0.5),
		// 	max_block_proposal_slot_portion: None,
		// 	telemetry: telemetry.as_ref().map(|x| x.handle()),
		// };

		// let rrsc = cessc_consensus_rrsc::start_rrsc(rrsc_config)?;
		// task_manager.spawn_essential_handle().spawn_blocking(
		// 	"rrsc-proposer",
		// 	Some("block-authoring"),
		// 	rrsc,
		// );
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

	// let config = grandpa::Config {
	// 	// FIXME #1578 make this available through chainspec
	// 	gossip_duration: std::time::Duration::from_millis(333),
	// 	justification_period: 512,
	// 	name: Some(name),
	// 	observer_enabled: false,
	// 	keystore,
	// 	local_role: role,
	// 	telemetry: telemetry.as_ref().map(|x| x.handle()),
	// 	protocol_name: grandpa_protocol_name,
	// };

	// if enable_grandpa {
	// 	// start the full GRANDPA voter
	// 	// NOTE: non-authorities could run the GRANDPA observer protocol, but at
	// 	// this point the full voter should provide better guarantees of block
	// 	// and vote data availability than the observer. The observer has not
	// 	// been tested extensively yet and having most nodes in a network run it
	// 	// could lead to finality stalls.
	// 	let grandpa_config = grandpa::GrandpaParams {
	// 		config,
	// 		link: grandpa_link,
	// 		network: network.clone(),
	// 		telemetry: telemetry.as_ref().map(|x| x.handle()),
	// 		voting_rule: grandpa::VotingRulesBuilder::default().build(),
	// 		prometheus_registry,
	// 		shared_voter_state,
	// 	};

	// 	// the GRANDPA voter task is considered infallible, i.e.
	// 	// if it fails we take down the service with it.
	// 	task_manager.spawn_essential_handle().spawn_blocking(
	// 		"grandpa-voter",
	// 		None,
	// 		grandpa::run_grandpa_voter(grandpa_config)?,
	// 	);
	// }

	let announce_block = {
		let network = network.clone();
		Arc::new(move |hash, data| network.announce_block(hash, data))
	};

	let relay_chain_slot_duration = Duration::from_secs(6);

	if validator {
		let parachain_consensus = build_consensus(
			client.clone(),
			block_import,
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|t| t.handle()),
			&task_manager,
			relay_chain_interface.clone(),
			transaction_pool,
			network,
			keystore_container.sync_keystore(),
			force_authoring,
			para_id,
		)?;

		let spawner = task_manager.spawn_handle();
		let params = cumulus_client_service::StartCollatorParams {
			para_id,
			block_status: client.clone(),
			announce_block,
			client: client.clone(),
			task_manager: &mut task_manager,
			relay_chain_interface,
			spawner,
			parachain_consensus,
			import_queue: import_queue_service,
			collator_key: collator_key.expect("Command line arguments do not allow this. qed"),
			relay_chain_slot_duration,
		};

		cumulus_client_service::start_collator(params).await?;
	} else {
		let params = cumulus_client_service::StartFullNodeParams {
			client: client.clone(),
			announce_block,
			task_manager: &mut task_manager,
			para_id,
			relay_chain_interface,
			relay_chain_slot_duration,
			import_queue: import_queue_service,
		};

		cumulus_client_service::start_full_node(params)?;
	}

	start_network.start_network();
	Ok((task_manager, client))
}

/// Builds a new service for a full client.
pub async fn new_full(
	parachain_config: Configuration,
	polkadot_config: Configuration,
	collator_options: CollatorOptions,
	eth_config: EthConfiguration,
	para_id: ParaId,
	disable_hardware_benchmarks: bool,
) -> sc_service::error::Result<(TaskManager, Arc<ParachainClient>)> {
	new_full_base(
		parachain_config, 
		polkadot_config, 
		collator_options, 
		eth_config,
		para_id,
		disable_hardware_benchmarks
	).await
}
