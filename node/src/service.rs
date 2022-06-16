//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

pub use crate::executor::ExecutorDispatch;
use crate::primitives as node_primitives;
use crate::rpc as node_rpc;
use cess_node_runtime::RuntimeApi;
use futures::prelude::*;
use node_primitives::Block;
use sc_client_api::{BlockBackend, ExecutorProvider};
use sc_consensus_babe::{self, SlotProportion};
use sc_executor::NativeElseWasmExecutor;
use sc_network::{Event, NetworkService};
use sc_service::{config::Configuration, error::Error as ServiceError, RpcHandlers, TaskManager};
use sc_telemetry::{Telemetry, TelemetryWorker};
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

// Our native executor instance.
pub type FullClient =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<ExecutorDispatch>>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type FullGrandpaBlockImport =
	grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>;

/// The transaction pool type defintion.
pub type TransactionPool = sc_transaction_pool::FullPool<Block, FullClient>;

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

/// Create a transaction using the given `call`.
///
/// The transaction will be signed by `sender`. If `nonce` is `None` it will be fetched from the
/// state of the best block.
///
/// Note: Should only be used for tests.

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
			impl Fn(
				node_rpc::DenyUnsafe,
				sc_rpc::SubscriptionTaskExecutor,
			) -> Result<node_rpc::IoHandler, sc_service::Error>,
			(
				sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
				grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
				sc_consensus_babe::BabeLink<Block>,
			),
			grandpa::SharedVoterState,
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

	let telemetry = telemetry.map(|(worker, telemetry)| {
		task_manager
			.spawn_handle()
			.spawn("telemetry", None, worker.run());
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

	let (grandpa_block_import, grandpa_link) = grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
		telemetry.as_ref().map(|x| x.handle()),
	)?;
	let justification_import = grandpa_block_import.clone();

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get(&*client)?,
		grandpa_block_import,
		client.clone(),
	)?;

	let slot_duration = babe_link.config().slot_duration();
	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		move |_, ()| async move {
			let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

			let slot =
				sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
					*timestamp,
					slot_duration,
				);

			let uncles =
				sp_authorship::InherentDataProvider::<<Block as BlockT>::Header>::check_inherents();

			Ok((timestamp, slot, uncles))
		},
		&task_manager.spawn_essential_handle(),
		config.prometheus_registry(),
		sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
		telemetry.as_ref().map(|x| x.handle()),
	)?;

	let import_setup = (block_import, grandpa_link, babe_link);

	let (rpc_extensions_builder, rpc_setup) = {
		let (_, grandpa_link, babe_link) = &import_setup;

		let justification_stream = grandpa_link.justification_stream();
		let shared_authority_set = grandpa_link.shared_authority_set().clone();
		let shared_voter_state = grandpa::SharedVoterState::empty();
		let rpc_setup = shared_voter_state.clone();

		let finality_proof_provider = grandpa::FinalityProofProvider::new_for_service(
			backend.clone(),
			Some(shared_authority_set.clone()),
		);

		let babe_config = babe_link.config().clone();
		let shared_epoch_changes = babe_link.epoch_changes().clone();

		let client = client.clone();
		let pool = transaction_pool.clone();
		let select_chain = select_chain.clone();
		let keystore = keystore_container.sync_keystore();
		let chain_spec = config.chain_spec.cloned_box();

		let rpc_backend = backend.clone();
		let rpc_extensions_builder = move |deny_unsafe, subscription_executor| {
			let deps = node_rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				select_chain: select_chain.clone(),
				chain_spec: chain_spec.cloned_box(),
				deny_unsafe,
				babe: node_rpc::BabeDeps {
					babe_config: babe_config.clone(),
					shared_epoch_changes: shared_epoch_changes.clone(),
					keystore: keystore.clone(),
				},
				grandpa: node_rpc::GrandpaDeps {
					shared_voter_state: shared_voter_state.clone(),
					shared_authority_set: shared_authority_set.clone(),
					justification_stream: justification_stream.clone(),
					subscription_executor,
					finality_provider: finality_proof_provider.clone(),
				},
			};

			node_rpc::create_full(deps, rpc_backend.clone()).map_err(Into::into)
		};

		(rpc_extensions_builder, rpc_setup)
	};

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		other: (rpc_extensions_builder, import_setup, rpc_setup, telemetry),
	})
}

/// Result of [`new_full_base`].
pub struct NewFullBase {
	/// The task manager of the node.
	pub task_manager: TaskManager,
	/// The client instance of the node.
	pub client: Arc<FullClient>,
	/// The networking service of the node.
	pub network: Arc<NetworkService<Block, <Block as BlockT>::Hash>>,
	/// The transaction pool of the node.
	pub transaction_pool: Arc<TransactionPool>,
	/// The rpc handlers of the node.
	pub rpc_handlers: RpcHandlers,
}

/// Creates a full service from the configuration.
pub fn new_full_base(
	mut config: Configuration,
	with_startup_data: impl FnOnce(
		&sc_consensus_babe::BabeBlockImport<Block, FullClient, FullGrandpaBlockImport>,
		&sc_consensus_babe::BabeLink<Block>,
	),
) -> Result<NewFullBase, ServiceError> {
	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		import_queue,
		keystore_container,
		select_chain,
		transaction_pool,
		other: (rpc_extensions_builder, import_setup, rpc_setup, mut telemetry),
	} = new_partial(&config)?;

	let shared_voter_state = rpc_setup;
	let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
	let grandpa_protocol_name = grandpa::protocol_standard_name(
		&client
			.block_hash(0)
			.ok()
			.flatten()
			.expect("Genesis block exists; qed"),
		&config.chain_spec,
	);

	config
		.network
		.extra_sets
		.push(grandpa::grandpa_peers_set_config(
			grandpa_protocol_name.clone(),
		));
	let warp_sync = Arc::new(grandpa::warp_proof::NetworkProvider::new(
		backend.clone(),
		import_setup.1.shared_authority_set().clone(),
		Vec::default(),
	));

	let (network, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			block_announce_validator_builder: None,
			warp_sync: Some(warp_sync),
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
	let backoff_authoring_blocks =
		Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();

	let rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		config,
		backend,
		client: client.clone(),
		keystore: keystore_container.sync_keystore(),
		network: network.clone(),
		rpc_extensions_builder: Box::new(rpc_extensions_builder),
		transaction_pool: transaction_pool.clone(),
		task_manager: &mut task_manager,
		system_rpc_tx,
		telemetry: telemetry.as_mut(),
	})?;

	let (block_import, grandpa_link, babe_link) = import_setup;

	(with_startup_data)(&block_import, &babe_link);

	if let sc_service::config::Role::Authority { .. } = &role {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool.clone(),
			prometheus_registry.as_ref(),
			telemetry.as_ref().map(|x| x.handle()),
		);

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let client_clone = client.clone();
		let slot_duration = babe_link.config().slot_duration();
		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.sync_keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: network.clone(),
			justification_sync_link: network.clone(),
			create_inherent_data_providers: move |parent, ()| {
				let client_clone = client_clone.clone();
				async move {
					let uncles = sc_consensus_uncles::create_uncles_inherent_data_provider(
						&*client_clone,
						parent,
					)?;

					let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

					let slot =
						sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
							*timestamp,
							slot_duration,
						);

					let storage_proof =
						sp_transaction_storage_proof::registration::new_data_provider(
							&*client_clone,
							&parent,
						)?;

					Ok((timestamp, slot, uncles, storage_proof))
				}
			},
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			can_author_with,
			block_proposal_slot_portion: SlotProportion::new(0.5),
			max_block_proposal_slot_portion: None,
			telemetry: telemetry.as_ref().map(|x| x.handle()),
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager.spawn_essential_handle().spawn_blocking(
			"babe-proposer",
			Some("block-authoring"),
			babe,
		);
	}

	// Spawn authority discovery module.
	if role.is_authority() {
		let authority_discovery_role =
			sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore());
		let dht_event_stream =
			network
				.event_stream("authority-discovery")
				.filter_map(|e| async move {
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
	let keystore = if role.is_authority() {
		Some(keystore_container.sync_keystore())
	} else {
		None
	};

	let config = grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: std::time::Duration::from_millis(333),
		justification_period: 512,
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
		let grandpa_config = grandpa::GrandpaParams {
			config,
			link: grandpa_link,
			network: network.clone(),
			telemetry: telemetry.as_ref().map(|x| x.handle()),
			voting_rule: grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state,
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			None,
			grandpa::run_grandpa_voter(grandpa_config)?,
		);
	}

	network_starter.start_network();
	Ok(NewFullBase {
		task_manager,
		client,
		network,
		transaction_pool,
		rpc_handlers,
	})
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration) -> Result<TaskManager, ServiceError> {
	new_full_base(config, |_, _| ()).map(|NewFullBase { task_manager, .. }| task_manager)
}
