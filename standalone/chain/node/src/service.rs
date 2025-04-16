//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use polkadot_sdk::{
	sc_consensus_beefy as beefy, sc_consensus_grandpa as grandpa,
	sp_consensus_beefy as beefy_primitives, *,
};

use std::{path::Path, sync::Arc};
use futures::prelude::*;
// Substrate
use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use sc_client_api::{Backend, BlockBackend};
use sc_consensus::BasicQueue;
use sc_consensus_babe::{self, SlotProportion};
use sc_network::{event::Event, service::traits::NetworkService, NetworkBackend, NetworkEventStream};
use sc_network_sync::{strategy::warp::WarpSyncConfig, SyncingService};
use sc_service::{error::Error as ServiceError, Configuration, PartialComponents, RpcHandlers, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sc_transaction_pool::{BasicPool, FullChainApi};
use sc_transaction_pool_api::OffchainTransactionPoolFactory;
use sp_core::U256;
use sp_runtime::traits::Block as BlockT;
// Runtime
use cess_node_primitives::opaque::Block;
use cess_node_runtime::{RuntimeApi, TransactionConverter};

use crate::{
	client::{FullBackend as FullBackendT, FullClient as FullClientT},
	eth::{self, db_config_dir, EthConfiguration, FrontierBackend, FrontierPartialComponents, StorageOverrideHandler},
	rpc::{self as node_rpc, EthDeps},
};

/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = (sp_io::SubstrateHostFunctions, frame_benchmarking::benchmarking::HostFunctions);
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = sp_io::SubstrateHostFunctions;

pub type FullClient = FullClientT<Block, RuntimeApi, HostFunctions>;
type FullBackend = FullBackendT<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport = grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

type BeefyAuthorityId = beefy_primitives::ecdsa_crypto::AuthorityId;
type FullBeefyBlockImport = beefy::import::BeefyBlockImport<
	Block,
	FullBackend,
	FullClient,
	FullGrandpaBlockImport,
	BeefyAuthorityId,
>;

type BasicImportQueue = sc_consensus::DefaultImportQueue<Block>;
type FullGrandpaLinkHalf = grandpa::LinkHalf<Block, FullClient, FullSelectChain>;
type FullBeefyVoterLinks = beefy::BeefyVoterLinks<Block, BeefyAuthorityId>;
type FullBeefyRpcLinks = beefy::BeefyRPCLinks<Block, BeefyAuthorityId>;

/// The transaction pool type definition.
pub type TransactionPool = BasicPool<FullChainApi<FullClient, Block>, Block>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

type BabeBlockImport = sc_consensus_babe::BabeBlockImport<Block, FullClient, FullBeefyBlockImport>;
type BabeWorkerHandle = sc_consensus_babe::BabeWorkerHandle<Block>;
type BabeLink = sc_consensus_babe::BabeLink<Block>;

pub fn new_partial(
	config: &Configuration,
) -> Result<
	PartialComponents<
		FullClient,
		FullBackend,
		FullSelectChain,
		BasicImportQueue,
		TransactionPool,
		(
			BabeBlockImport,
			BabeWorkerHandle,
			BabeLink,
			FullGrandpaLinkHalf,
			FullBeefyVoterLinks,
			FullBeefyRpcLinks,
			Option<Telemetry>,
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

	let executor = sc_service::new_wasm_executor(&config.executor);

	let (client, backend, keystore_container, task_manager) = sc_service::new_full_parts::<Block, RuntimeApi, _>(
		config,
		telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
		executor,
	)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager.spawn_handle().spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	// FIXME: The `config.transaction_pool.options` field is private, so for now use its default value
	let transaction_pool = Arc::from(BasicPool::new_full(
		Default::default(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	));

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(),
		GRANDPA_JUSTIFICATION_PERIOD,
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (beefy_block_import, beefy_voter_links, beefy_rpc_links) =
		beefy::beefy_block_import_and_links(
			grandpa_block_import,
			backend.clone(),
			client.clone(),
			config.prometheus_registry().cloned(),
		);

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::configuration(&*client)?,
		beefy_block_import,
		client.clone(),
	)?;

	let slot_duration = babe_link.config().slot_duration();
	let (import_queue, babe_worker_handle) = {
		let param = sc_consensus_babe::ImportQueueParams {
			link: babe_link.clone(),
			block_import: block_import.clone(),
			justification_import: Some(Box::new(justification_import)),
			client: client.clone(),
			select_chain: select_chain.clone(),
			create_inherent_data_providers: move |_, ()| async move {
				let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

				let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

				Ok((slot, timestamp))
			},
			spawner: &task_manager.spawn_essential_handle(),
			registry: config.prometheus_registry(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		};
		sc_consensus_babe::import_queue(param).map_err::<ServiceError, _>(Into::into)?
	};

	Ok(PartialComponents {
		client,
		backend,
		keystore_container,
		task_manager,
		select_chain,
		import_queue,
		transaction_pool,
		other: (
			block_import,
			babe_worker_handle,
			babe_link,
			grandpa_link,
			beefy_voter_links,
			beefy_rpc_links,
			telemetry,
		),
	})
}

/// Result of [`new_full_base`].
#[allow(dead_code)]
pub struct NewFullBase {
	/// The task manager of the node.
	pub task_manager: TaskManager,
	/// The client instance of the node.
	pub client: Arc<FullClient>,
	/// The networking service of the node.
	pub network: Arc<dyn NetworkService>,
	/// The syncing service of the node.
	pub sync: Arc<SyncingService<Block>>,
	/// The transaction pool of the node.
	pub transaction_pool: Arc<TransactionPool>,
	/// The rpc handlers of the node.
	pub rpc_handlers: RpcHandlers,
}

/// Creates a full service from the configuration.
pub fn new_full_base<N: NetworkBackend<Block, <Block as BlockT>::Hash>>(
	mut config: Configuration,
	eth_config: EthConfiguration,
	disable_hardware_benchmarks: bool,
) -> Result<NewFullBase, ServiceError> {
	let is_offchain_indexing_enabled = config.offchain_worker.indexing_enabled;
	let role = config.role;
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks = Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	let enable_offchain_worker = config.offchain_worker.enabled;

	let hwbench = (!disable_hardware_benchmarks)
		.then_some(config.database.path().map(|database_path| {
			let _ = std::fs::create_dir_all(&database_path);
			sc_sysinfo::gather_hwbench(Some(database_path), &SUBSTRATE_REFERENCE_HARDWARE)
		}))
		.flatten();

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (
			block_import, 
			babe_worker_handle,
			babe_link,
			grandpa_link,
			beefy_voter_links,
			beefy_rpc_links,
			mut telemetry, 
		),
	} = new_partial(&config)?;

	let metrics = N::register_notification_metrics(config.prometheus_config.as_ref().map(|cfg| &cfg.registry));

	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	let auth_disc_public_addresses = config.network.public_addresses.clone();

	let mut net_config = sc_network::config::FullNetworkConfiguration::<_, _, N>::new(
		&config.network,
		config.prometheus_config.as_ref().map(|cfg| cfg.registry.clone()),
	);

	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let peer_store_handle = net_config.peer_store_handle();

	let grandpa_protocol_name = grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);
	let (grandpa_protocol_config, grandpa_notification_service) = grandpa::grandpa_peers_set_config::<_, N>(
		grandpa_protocol_name.clone(),
		metrics.clone(),
		Arc::clone(&peer_store_handle),
	);
	net_config.add_notification_protocol(grandpa_protocol_config);

	let beefy_gossip_proto_name =
		beefy::gossip_protocol_name(&genesis_hash, config.chain_spec.fork_id());
	// `beefy_on_demand_justifications_handler` is given to `beefy-gadget` task to be run,
	// while `beefy_req_resp_cfg` is added to `config.network.request_response_protocols`.
	let (beefy_on_demand_justifications_handler, beefy_req_resp_cfg) =
		beefy::communication::request_response::BeefyJustifsRequestHandler::new::<_, N>(
			&genesis_hash,
			config.chain_spec.fork_id(),
			client.clone(),
			prometheus_registry.clone(),
		);

	let (beefy_notification_config, beefy_notification_service) =
		beefy::communication::beefy_peers_set_config::<_, N>(
			beefy_gossip_proto_name.clone(),
			metrics.clone(),
			Arc::clone(&peer_store_handle),
		);

	net_config.add_notification_protocol(beefy_notification_config);
	net_config.add_request_response_protocol(beefy_req_resp_cfg);

	let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		grandpa_link.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			net_config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_config: Some(WarpSyncConfig::WithProvider(warp_sync)),
			block_relay: None,
			metrics,
		})?;

	let (
		FrontierPartialComponents { filter_pool, fee_history_cache, fee_history_cache_limit },
		eth_block_data_cache,
		eth_block_notification_sink,
		storage_override,
		eth_inherent_data_providers,
		frontier_backend,
	) = new_frontier_and_rpc_dep_partial(
		&mut config,
		&eth_config,
		babe_link.config().slot_duration(),
		client.clone(),
		backend.clone(),
		&mut task_manager,
		sync_service.clone(),
		prometheus_registry.clone(),
	)?;

	let shared_voter_state = grandpa::SharedVoterState::empty();
	let rpc_builder = {
		let client = client.clone();
		let backend = backend.clone();
		let transaction_pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let network = network.clone();
		let sync_service = sync_service.clone();
		let shared_voter_state = shared_voter_state.clone();
		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let is_authority = config.role.is_authority();
		let finality_proof_provider =
			grandpa::FinalityProofProvider::new_for_service(backend.clone(), Some(shared_authority_set.clone()));
		let keystore = keystore_container.keystore();
		let chain_spec = config.chain_spec.cloned_box();
		let enable_dev_signer = eth_config.enable_dev_signer;
		let execute_gas_limit_multiplier = eth_config.execute_gas_limit_multiplier;
		let max_past_logs = eth_config.max_past_logs;

		let rpc_builder = move |subscription_executor: node_rpc::SubscriptionTaskExecutor| {
			let eth_rpc_deps = FullEthDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				graph: transaction_pool.pool().clone(),
				converter: Some(TransactionConverter),
				is_authority,
				enable_dev_signer,
				network: network.clone(),
				sync: sync_service.clone(),
				frontier_backend: frontier_backend.clone(),
				storage_override: storage_override.clone(),
				block_data_cache: eth_block_data_cache.clone(),
				filter_pool: filter_pool.clone(),
				max_past_logs,
				fee_history_cache: fee_history_cache.clone(),
				fee_history_cache_limit,
				execute_gas_limit_multiplier,
				forced_parent_hashes: None,
				pending_create_inherent_data_providers: eth_inherent_data_providers.clone(),
				pubsub_notification_sinks: eth_block_notification_sink.clone(),
			};
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: transaction_pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				babe: node_rpc::BabeDeps { keystore: keystore.clone(), babe_worker_handle: babe_worker_handle.clone() },
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor: subscription_executor.clone(),
					finality_provider: finality_proof_provider.clone(),
				},
				beefy: node_rpc::BeefyDeps::<BeefyAuthorityId> {
					beefy_finality_proof_stream: beefy_rpc_links
						.from_voter_justif_stream
						.clone(),
					beefy_best_block_stream: beefy_rpc_links
						.from_voter_best_beefy_stream
						.clone(),
					subscription_executor: subscription_executor.clone(),
				},
				backend: backend.clone(),
			};

			node_rpc::create_full(deps, eth_rpc_deps, subscription_executor).map_err(Into::into)
		};
		rpc_builder
	};

	let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		backend: backend.clone(),
		client: client.clone(),
		keystore: keystore_container.keystore(),
		network: network.clone(),
		rpc_builder: Box::new(rpc_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut(),
	})?;

	if let Some(hwbench) = hwbench {
		sc_sysinfo::print_hwbench(&hwbench);
		match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench, false) {
			Err(err) if role.is_authority() => {
				log::warn!("⚠️  The hardware does not meet the minimal requirements {} for role 'Authority'.", err);
			},
			_ => {},
		}

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
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: sync_service.clone(),
			justification_sync_link: sync_service.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
						*timestamp,
						slot_duration,
					);

					let storage_proof =
						sp_transaction_storage_proof::registration::new_data_provider(&*client_clone, &parent)?;

					Ok((slot, timestamp, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("babe-proposer", Some("block-authoring"), babe);
	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role = sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream = network.event_stream("authority-discovery").filter_map(|e| async move {
			match e {
				Event::Dht(e) => Some(e),
				_ => None,
			}
		});
		let (authority_discovery_worker, _service) = sc_authority_discovery::new_worker_and_service_with_config(
			sc_authority_discovery::WorkerConfig {
				publish_non_global_ips: auth_disc_publish_non_global_ips,
				public_addresses: auth_disc_public_addresses,
				..Default::default()
			},
			client.clone(),
			Arc::new(network.clone()),
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
	let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

	// beefy is enabled if its notification service exists
	let network_params = beefy::BeefyNetworkParams {
		network: Arc::new(network.clone()),
		sync: sync_service.clone(),
		gossip_protocol_name: beefy_gossip_proto_name,
		justifications_protocol_name: beefy_on_demand_justifications_handler.protocol_name(),
		notification_service: beefy_notification_service,
		_phantom: core::marker::PhantomData::<Block>,
	};
	let beefy_params = beefy::BeefyParams {
		client: client.clone(),
		backend: backend.clone(),
		payload_provider: sp_consensus_beefy::mmr::MmrRootProvider::new(client.clone()),
		runtime: client.clone(),
		key_store: keystore.clone(),
		network_params,
		min_block_delta: 8,
		prometheus_registry: prometheus_registry.clone(),
		links: beefy_voter_links,
		on_demand_justifications_handler: beefy_on_demand_justifications_handler,
		is_authority: role.is_authority(),
	};

	let beefy_gadget = beefy::start_beefy_gadget::<_, _, _, _, _, _, _, _>(beefy_params);
	// BEEFY is part of consensus, if it fails we'll bring the node down with it to make sure it
	// is noticed.
	task_manager
		.spawn_essential_handle()
		.spawn_blocking("beefy-gadget", None, beefy_gadget);
	// When offchain indexing is enabled, MMR gadget should also run.
	if is_offchain_indexing_enabled {
		task_manager.spawn_essential_handle().spawn_blocking(
			"mmr-gadget",
			None,
			mmr_gadget::MmrGadget::start(
				client.clone(),
				backend.clone(),
				sp_mmr_primitives::INDEXING_PREFIX.to_vec(),
			),
		);
	}

	let grandpa_config = grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(333),
		justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
		name: Some(name),
		observer_enabled: false,
		keystore,
		local_role: role,
		telemetry: telemetry.as_ref().map(|x| x.handle()),
		protocol_name: grandpa_protocol_name,
	};

	if enable_grandpa {
		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_params = grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network: network.clone(),
			sync: Arc::new(sync_service.clone()),
			notification_service: grandpa_notification_service,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry: prometheus_registry.clone(),
			shared_voter_state,
			offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool.clone()),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			grandpa::run_grandpa_voter(grandpa_params)?,
		);
	}

	if enable_offchain_worker {
		let offchain_workers = sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
			runtime_api_provider: client.clone(),
			is_validator: role.is_authority(),
			keystore: Some(keystore_container.keystore()),
			offchain_db: backend.offchain_storage(),
			transaction_pool: Some(OffchainTransactionPoolFactory::new(transaction_pool.clone())),
			network_provider: Arc::new(network.clone()),
			enable_http_requests: true,
			custom_extensions: |_| vec![],
		})?;
		task_manager.spawn_handle().spawn(
			"offchain-workers-runner",
			"offchain-worker",
			offchain_workers.run(client.clone(), task_manager.spawn_handle()).boxed(),
		);
	}

	network_starter.start_network();
	Ok(NewFullBase { task_manager, client, network, sync: sync_service, transaction_pool, rpc_handlers })
}

#[derive(Clone)]
struct EthPendingCreateInherentDataProvider {
	slot_duration: sp_consensus_slots::SlotDuration,
	target_gas_price: u64,
}

#[async_trait::async_trait]
impl sp_inherents::CreateInherentDataProviders<Block, ()> for EthPendingCreateInherentDataProvider {
	type InherentDataProviders = (
		sp_consensus_babe::inherents::InherentDataProvider,
		sp_timestamp::InherentDataProvider,
		fp_dynamic_fee::InherentDataProvider,
	);
	async fn create_inherent_data_providers(
		&self,
		_parent: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		let current = sp_timestamp::InherentDataProvider::from_system_time();
		let next_slot = current.timestamp().as_millis() + self.slot_duration.as_millis();
		let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
		let slot = sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
			*timestamp,
			self.slot_duration,
		);
		let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(self.target_gas_price));
		Ok((slot, timestamp, dynamic_fee))
	}
}

type FullEthDeps = EthDeps<
	Block,
	FullClient,
	TransactionPool,
	FullChainApi<FullClient, Block>,
	TransactionConverter,
	EthPendingCreateInherentDataProvider,
>;
type FullEthBlockNotificationSinks =
	fc_mapping_sync::EthereumBlockNotificationSinks<fc_mapping_sync::EthereumBlockNotification<Block>>;
type FullStorageOverrideHandler = StorageOverrideHandler<Block, FullClient, FullBackend>;
type FullFrontierBackend = FrontierBackend<Block, FullClient>;

fn new_frontier_and_rpc_dep_partial(
	config: &mut Configuration,
	eth_config: &EthConfiguration,
	slot_duration: sp_consensus_slots::SlotDuration,
	client: Arc<FullClient>,
	backend: Arc<FullBackend>,
	task_manager: &TaskManager,
	sync_service: Arc<SyncingService<Block>>,
	prometheus_registry: Option<substrate_prometheus_endpoint::Registry>,
) -> Result<
	(
		FrontierPartialComponents,
		Arc<fc_rpc::EthBlockDataCacheTask<Block>>,
		Arc<FullEthBlockNotificationSinks>,
		Arc<FullStorageOverrideHandler>,
		EthPendingCreateInherentDataProvider,
		Arc<FullFrontierBackend>,
	),
	ServiceError,
> {
	// for ethereum-compatibility rpc.
	config.rpc.id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));

	let storage_override = Arc::new(StorageOverrideHandler::<Block, _, _>::new(client.clone()));
	let frontier_backend =
		Arc::new(fc_db::kv::Backend::open(Arc::clone(&client), &config.database, &db_config_dir(config))?);
	let FrontierPartialComponents { filter_pool, fee_history_cache, fee_history_cache_limit } =
		eth::new_frontier_partial(&eth_config)?;

	// Sinks for pubsub notifications.
	// Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
	// The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the
	// subscriber on receiving a message through this channel. This way we avoid race conditions when using native
	// substrate block import notification stream.
	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<Block>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	let pending_create_inherent_data_providers =
		EthPendingCreateInherentDataProvider { slot_duration, target_gas_price: eth_config.target_gas_price };
	let block_data_cache = Arc::new(fc_rpc::EthBlockDataCacheTask::new(
		task_manager.spawn_handle(),
		storage_override.clone(),
		eth_config.eth_log_block_cache,
		eth_config.eth_statuses_cache,
		prometheus_registry.clone(),
	));

	eth::spawn_frontier_tasks(
		task_manager,
		client,
		backend,
		frontier_backend.clone(),
		filter_pool.clone(),
		storage_override.clone(),
		fee_history_cache.clone(),
		fee_history_cache_limit.clone(),
		sync_service,
		pubsub_notification_sinks.clone(),
	);

	Ok((
		FrontierPartialComponents { filter_pool, fee_history_cache, fee_history_cache_limit },
		block_data_cache,
		pubsub_notification_sinks,
		storage_override,
		pending_create_inherent_data_providers,
		frontier_backend,
	))
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration, cli: crate::cli::Cli) -> Result<TaskManager, ServiceError> {
	let database_path = config.database.path().map(Path::to_path_buf);
	let eth_config = cli.eth;
	let task_manager = match config.network.network_backend.unwrap_or_default() {
		sc_network::config::NetworkBackendType::Libp2p => {
			let task_manager =
				new_full_base::<sc_network::NetworkWorker<_, _>>(config, eth_config, cli.no_hardware_benchmarks)
					.map(|NewFullBase { task_manager, .. }| task_manager)?;
			task_manager
		},
		sc_network::config::NetworkBackendType::Litep2p => {
			let task_manager =
				new_full_base::<sc_network::Litep2pNetworkBackend>(config, eth_config, cli.no_hardware_benchmarks)
					.map(|NewFullBase { task_manager, .. }| task_manager)?;
			task_manager
		},
	};

	if let Some(database_path) = database_path {
		sc_storage_monitor::StorageMonitorService::try_spawn(
			cli.storage_monitor,
			database_path,
			&task_manager.spawn_essential_handle(),
		)
		.map_err(|e| ServiceError::Application(e.into()))?;
	}

	Ok(task_manager)
}

pub fn new_chain_ops(
	config: &mut Configuration,
) -> Result<
	(Arc<FullClient>, Arc<FullBackend>, BasicQueue<Block>, TaskManager, Arc<FrontierBackend<Block, FullClient>>),
	ServiceError,
> {
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	let PartialComponents { client, backend, import_queue, task_manager, .. } = new_partial(&config)?;
	let frontier_backend =
		Arc::new(fc_db::kv::Backend::open(Arc::clone(&client), &config.database, &db_config_dir(config))?);
	Ok((client, backend, import_queue, task_manager, frontier_backend))
}
