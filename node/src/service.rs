// This file is part of Substrate.
pub use crate::{
	executor::ExecutorDispatch,
	eth::{
		db_config_dir, spawn_frontier_tasks, new_frontier_partial,
		EthConfiguration, BackendType, FrontierBackend, FrontierBlockImport, FrontierPartialComponents, EthCompatRuntimeApiCollection
	},
	cli::Sealing,
	client::{BaseRuntimeApiCollection, FullBackend, FullClient, RuntimeApiCollection, CESSNodeRuntimeExecutor},
};
use crate::{
	primitives as node_primitives, 
	rpc as node_rpc
};
use sc_rpc::SubscriptionTaskExecutor;
use futures::prelude::*;
use node_primitives::Block;
use sc_client_api::{BlockBackend, StateBackendFor};
use cessc_consensus_rrsc::{self, SlotProportion};
use sc_executor::{NativeExecutionDispatch};
use sc_network::{event::Event, NetworkEventStream};
use sc_network_common::sync::warp::WarpSyncParams;
use sc_service::{ 
	Configuration, error::Error as ServiceError, 
	TaskManager, PartialComponents,
};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_runtime::{
	traits::{BlakeTwo256}, 
};
use sp_api::{ConstructRuntimeApi};
use std::{
	sync::{Arc},
	time::Duration,
	path::Path,
};
use cess_node_runtime::{TransactionConverter};
use sc_consensus::BasicQueue;
use sp_trie::PrefixedMemoryDB;

type BasicImportQueue<Client> = sc_consensus::DefaultImportQueue<Block, Client>;
type FullPool<Client> = sc_transaction_pool::FullPool<Block, Client>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

type GrandpaLinkHalf<Client> = grandpa::LinkHalf<Block, Client, FullSelectChain>;
type GrandpaBlockImport<Client> =
	grandpa::GrandpaBlockImport<FullBackend, Block, Client, FullSelectChain>;
// type BoxBlockImport<Client> = sc_consensus::BoxBlockImport<Block, TransactionFor<Client, Block>>;
/// The transaction pool type defintion.
// pub type TransactionPool = sc_transaction_pool::FullPool<Block, FullClient<cess_node_runtime::RuntimeApi, CESSNodeRuntimeExecutor>>;

pub type Client = FullClient<cess_node_runtime::RuntimeApi, CESSNodeRuntimeExecutor>;

pub fn new_partial<RuntimeApi, Executor>(
	config: &Configuration,
	eth_config: &EthConfiguration,
) -> Result<
	PartialComponents<
		FullClient<RuntimeApi, Executor>,
		FullBackend,
		FullSelectChain,
		BasicImportQueue<FullClient<RuntimeApi, Executor>>,
		FullPool<FullClient<RuntimeApi, Executor>>,
		(
			cessc_consensus_rrsc::RRSCLink<Block>,
			cessc_consensus_rrsc::RRSCWorkerHandle<Block>,
			Option<Telemetry>,
			cessc_consensus_rrsc::RRSCBlockImport<Block, FullClient<RuntimeApi, Executor>, GrandpaBlockImport<FullClient<RuntimeApi, Executor>>>,
			GrandpaLinkHalf<FullClient<RuntimeApi, Executor>>,
			FrontierBackend,
			Arc<fc_rpc::OverrideHandle<Block>>,
		),
	>,
	ServiceError,
>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi: RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>
		+ EthCompatRuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
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

	let executor = sc_service::new_native_or_wasm_executor(config);

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, _>(
			config,
			telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
			executor,
		)?;
	let client = Arc::new(client);

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
		telemetry
	});

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(),
		&client,
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();

	let overrides = crate::rpc::overrides_handle(client.clone());

	let (block_import, rrsc_link) = cessc_consensus_rrsc::block_import(
		cessc_consensus_rrsc::configuration(&*client)?,
		grandpa_block_import.clone(),
		client.clone(),
	)?;

	let slot_duration = rrsc_link.config().slot_duration();
	let (import_queue, rrsc_worker_handle) = cessc_consensus_rrsc::import_queue(
		rrsc_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot = 
				cessp_consensus_rrsc::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			Ok((slot, timestamp))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		telemetry.as_ref().map(|x| x.handle()),
	).map_err::<ServiceError, _>(Into::into)?;

	let frontier_backend = match eth_config.frontier_backend_type {
		BackendType::KeyValue => FrontierBackend::KeyValue(fc_db::kv::Backend::open(
			Arc::clone(&client),
			&config.database,
			&db_config_dir(config),
		)?),
		BackendType::Sql => {
			let db_path = db_config_dir(config).join("sql");
			std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
			let backend = futures::executor::block_on(fc_db::sql::Backend::new(
				fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
					path: Path::new("sqlite:///")
						.join(db_path)
						.join("frontier.db3")
						.to_str()
						.unwrap(),
					create_if_missing: true,
					thread_count: eth_config.frontier_sql_backend_thread_count,
					cache_size: eth_config.frontier_sql_backend_cache_size,
				}),
				eth_config.frontier_sql_backend_pool_size,
				std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
				overrides.clone(),
			))
			.unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
			FrontierBackend::Sql(backend)
		}
	};

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_essential_handle(),
		client.clone(),
	);

	Ok(PartialComponents {
		client,
		backend,
		keystore_container,
		task_manager,
		select_chain,
		import_queue,
		transaction_pool,
		other: (
			rrsc_link,
			rrsc_worker_handle,
			telemetry,
			block_import,
			grandpa_link,
			frontier_backend,
			overrides,
		),
	})
}

/// Builds a new service for a full client.
pub async fn new_full<RuntimeApi, Executor>(
	mut config: Configuration,
	eth_config: EthConfiguration,
	sealing: Option<Sealing>,
	with_startup_data: impl FnOnce(
		&cessc_consensus_rrsc::RRSCBlockImport<Block, FullClient<RuntimeApi, Executor>, GrandpaBlockImport<FullClient<RuntimeApi, Executor>>>,
		&cessc_consensus_rrsc::RRSCLink<Block>,
	),
) -> Result<TaskManager, ServiceError>
where
	RuntimeApi: ConstructRuntimeApi<Block, FullClient<RuntimeApi, Executor>>,
	RuntimeApi: Send + Sync + 'static,
	RuntimeApi::RuntimeApi:
		RuntimeApiCollection<StateBackend = StateBackendFor<FullBackend, Block>>,
	Executor: NativeExecutionDispatch + 'static,
{
	let PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (rrsc_link, rrsc_worker_handle, mut telemetry, block_import, grandpa_link, frontier_backend, overrides),
	} = new_partial::<RuntimeApi, Executor>(&config, &eth_config)?;

	let FrontierPartialComponents {
		filter_pool,
		fee_history_cache,
		fee_history_cache_limit,
	} = new_frontier_partial(&eth_config)?;

	let grandpa_protocol_name = grandpa::protocol_standard_name(
		&client.block_hash(0)?.expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	let warp_sync_params = if sealing.is_some() {
		None
	} else {
		config
			.network
			.extra_sets
			.push(grandpa::grandpa_peers_set_config(
				grandpa_protocol_name.clone(),
			));
		let warp_sync: Arc<dyn sc_network::config::WarpSyncProvider<Block>> =
			Arc::new(grandpa::warp_proof::NetworkProvider::new(
				backend.clone(),
				grandpa_link.shared_authority_set().clone(),
				Vec::default(),
			));
		Some(WarpSyncParams::WithProvider(warp_sync))
	};

	let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync_params,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa && sealing.is_none();
	let prometheus_registry = config.prometheus_registry().cloned();
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;

	// Channel for the rpc handler to communicate with the authorship task.
	// let (command_sink, commands_stream) = mpsc::channel::<T>(1000);

	// Sinks for pubsub notifications.
	// Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
	// The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
	// This way we avoid race conditions when using native substrate block import notification stream.
	let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
		fc_mapping_sync::EthereumBlockNotification<Block>,
	> = Default::default();
	let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

	// for ethereum-compatibility rpc.
	config.rpc_id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));
	let eth_rpc_params = crate::rpc::EthDeps {
		client: client.clone(),
		pool: transaction_pool.clone(),
		graph: transaction_pool.pool().clone(),
		converter: Some(TransactionConverter),
		is_authority: config.role.is_authority(),
		enable_dev_signer: eth_config.enable_dev_signer,
		network: network.clone(),
		sync: sync_service.clone(),
		frontier_backend: match frontier_backend.clone() {
			fc_db::Backend::KeyValue(b) => Arc::new(b),
			fc_db::Backend::Sql(b) => Arc::new(b),
		},
		overrides: overrides.clone(),
		block_data_cache: Arc::new(fc_rpc::EthBlockDataCacheTask::new(
			task_manager.spawn_handle(),
			overrides.clone(),
			eth_config.eth_log_block_cache,
			eth_config.eth_statuses_cache,
			prometheus_registry.clone(),
		)),
		filter_pool: filter_pool.clone(),
		max_past_logs: eth_config.max_past_logs,
		fee_history_cache: fee_history_cache.clone(),
		fee_history_cache_limit,
		execute_gas_limit_multiplier: eth_config.execute_gas_limit_multiplier,
		forced_parent_hashes: None,
	};

	let (rpc_extensions_builder, _rpc_setup) = {
		let grandpa_link= &grandpa_link;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = grandpa::SharedVoterState::empty();
		let shared_voter_state2 = shared_voter_state.clone();

		let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let client = client.clone();
		let pool = transaction_pool.clone();
		let pubsub_notification_sinks = pubsub_notification_sinks.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.keystore();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_backend = backend.clone();
		let rpc_extensions_builder = move |deny_unsafe, subscription_executor: SubscriptionTaskExecutor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				rrsc: node_rpc::RRSCDeps {
					keystore: keystore.clone(),
					rrsc_worker_handle: rrsc_worker_handle.clone(),
				},
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor: subscription_executor.clone(),
					finality_provider: finality_proof_provider.clone(),
				},
				command_sink: None,
				eth: eth_rpc_params.clone(),
			};

			node_rpc::create_full(deps, subscription_executor, pubsub_notification_sinks.clone(), rpc_backend.clone()).map_err(Into::into)
		};

		(rpc_extensions_builder, shared_voter_state2)
	};

	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());

	let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		client: client.clone(),
		backend: backend.clone(),
		task_manager: &mut task_manager,
		keystore: keystore_container.keystore(),
		transaction_pool: transaction_pool.clone(),
		rpc_builder: Box::new(rpc_extensions_builder),
		network: network.clone(),
		system_rpc_tx,
		tx_handler_controller,
		sync_service: sync_service.clone(),
		telemetry: telemetry.as_mut(),
	})?;

	spawn_frontier_tasks(
		&task_manager,
		client.clone(),
		backend,
		frontier_backend,
		filter_pool,
		overrides,
		fee_history_cache,
		fee_history_cache_limit,
		sync_service.clone(),
		pubsub_notification_sinks,
	)
	.await;

	(with_startup_data)(&block_import, &rrsc_link);

	if let sc_service::config::Role::Authority { .. } = &role {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let client_clone = client.clone();
		let slot_duration = rrsc_link.config().slot_duration();
		let rrsc_config = cessc_consensus_rrsc::RRSCParams {
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

					let slot =
						cessp_consensus_rrsc::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					let storage_proof =
						sp_transaction_storage_proof::registration::new_data_provider(
							&*client_clone,
							&parent,
						)?;

					Ok((slot, timestamp, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			rrsc_link,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let rrsc = cessc_consensus_rrsc::start_rrsc(rrsc_config)?;
		task_manager.spawn_essential_handle().spawn_blocking(
			"rrsc-proposer",
			Some("block-authoring"),
			rrsc,
		);
	}

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

	if enable_grandpa {
		// if the node isn't actively participating in consensus then it doesn't
		// need a keystore, regardless of which protocol we use below.
		let keystore = if role.is_authority() {
			Some(keystore_container.keystore())
		} else {
			None
		};

		let grandpa_config = grandpa::Config {
			// FIXME #1578 make this available through chainspec
			gossip_duration: Duration::from_millis(333),
			justification_period: 512,
			name: Some(name),
			observer_enabled: false,
			keystore,
			local_role: role,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			protocol_name: grandpa_protocol_name,
		};

		// start the full GRANDPA voter
		// NOTE: non-authorities could run the GRANDPA observer protocol, but at
		// this point the full voter should provide better guarantees of block
		// and vote data availability than the observer. The observer has not
		// been tested extensively yet and having most nodes in a network run it
		// could lead to finality stalls.
		let grandpa_voter =
			grandpa::run_grandpa_voter(grandpa::GrandpaParams {
				config: grandpa_config,
				link: grandpa_link,
				network,
				sync: sync_service,
				voting_rule: grandpa::VotingRulesBuilder::default().build(),
				prometheus_registry,
				shared_voter_state: grandpa::SharedVoterState::empty(),
				telemetry: telemetry.as_ref().map(|x| x.handle()),
			})?;

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("grandpa-voter", None, grandpa_voter);
	}

	network_starter.start_network();

	// let database_source = config.database.clone();
	// sc_storage_monitor::StorageMonitorService::try_spawn(
	// 	cli.storage_monitor,
	// 	database_source,
	// 	&task_manager.spawn_essential_handle(),
	// )
	// .map_err(|e| ServiceError::Application(e.into()))?;

	Ok(task_manager)
}

pub async fn build_full(
	config: Configuration,
	eth_config: EthConfiguration,
	sealing: Option<Sealing>,
) -> Result<TaskManager, ServiceError> {
	new_full::<cess_node_runtime::RuntimeApi, CESSNodeRuntimeExecutor>(
		config, eth_config, sealing, |_, _| (),
	)
	.await
}

pub fn new_chain_ops(
	config: &mut Configuration,
	eth_config: &EthConfiguration,
) -> Result<
	(
		Arc<Client>,
		Arc<FullBackend>,
		BasicQueue<Block, PrefixedMemoryDB<BlakeTwo256>>,
		TaskManager,
		FrontierBackend,
	),
	ServiceError,
> {
	config.keystore = sc_service::config::KeystoreConfig::InMemory;
	let PartialComponents {
		client,
		backend,
		import_queue,
		task_manager,
		other,
		..
	} = new_partial::<cess_node_runtime::RuntimeApi, CESSNodeRuntimeExecutor>(
		config,
		eth_config,
	)?;
	Ok((client, backend, import_queue, task_manager, other.5))
}
