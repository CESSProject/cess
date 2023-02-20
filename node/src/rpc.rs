//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use crate::primitives as node_primitives;
use fc_rpc::{
	EthBlockDataCacheTask, OverrideHandle, RuntimeApiStorageOverride, SchemaV1Override,
	SchemaV2Override, SchemaV3Override, StorageOverride,
};
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use fp_storage::EthereumStorageSchema;
use grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use node_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Index};
use jsonrpsee::RpcModule;
use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
	client::BlockchainEvents,
};
use cessc_consensus_rrsc::{RRSCConfiguration, Epoch};
use sc_consensus_epochs::SharedEpochChanges;
use sc_network::NetworkService;
use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::{ChainApi, Pool};
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use cessp_consensus_rrsc::RRSCApi;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::BlakeTwo256;
use std::{collections::BTreeMap, sync::Arc};

/// Extra dependencies for RRSC.
pub struct RRSCDeps {
	/// RRSC protocol config.
	pub rrsc_config: RRSCConfiguration,
	/// RRSC pending epoch changes.
	pub shared_epoch_changes: SharedEpochChanges<Block, Epoch>,
	/// The keystore that manages the keys of the node.
	pub keystore: SyncCryptoStorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B, /*A: ChainApi*/> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// RRSC specific dependencies.
	pub rrsc: RRSCDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	// /// Graph pool instance.
	// pub graph: Arc<Pool<A>>,
	// /// The Node authority flag
	// pub is_authority: bool,
	// /// Whether to enable dev signer
	// pub enable_dev_signer: bool,
	// /// Network service
	// pub network: Arc<NetworkService<Block, Hash>>,
	// /// EthFilterApi pool.
	// pub filter_pool: Option<FilterPool>,
	// /// Backend.
	// pub frontier_backend: Arc<fc_db::Backend<Block>>,
	// /// Maximum number of logs in a query.
	// pub max_past_logs: u32,
	// /// Maximum fee history cache size.
	// pub fee_history_limit: u64,
	// /// Fee history cache.
	// pub fee_history_cache: FeeHistoryCache,
	// /// Ethereum data access overrides.
	// pub overrides: Arc<OverrideHandle<Block>>,
	// /// Cache for Ethereum block data.
	// pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
}

pub fn overrides_handle<C, BE>(client: Arc<C>) -> Arc<OverrideHandle<Block>>
where
	C: ProvideRuntimeApi<Block> + StorageProvider<Block, BE> + AuxStore,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError>,
	C: Send + Sync + 'static,
	C::Api: sp_api::ApiExt<Block>
		+ fp_rpc::EthereumRuntimeRPCApi<Block>
		+ fp_rpc::ConvertTransactionRuntimeApi<Block>,
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
{
	let mut overrides_map = BTreeMap::new();
	overrides_map.insert(
		EthereumStorageSchema::V1,
		Box::new(SchemaV1Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);
	overrides_map.insert(
		EthereumStorageSchema::V2,
		Box::new(SchemaV2Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);
	overrides_map.insert(
		EthereumStorageSchema::V3,
		Box::new(SchemaV3Override::new(client.clone()))
			as Box<dyn StorageOverride<_> + Send + Sync>,
	);

	Arc::new(OverrideHandle {
		schemas: overrides_map,
		fallback: Box::new(RuntimeApiStorageOverride::new(client.clone())),
	})
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, SC, B, BE,/* A*/>(
	deps: FullDeps<C, P, SC, B, /* A*/>,
	backend: Arc<B>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
	C: ProvideRuntimeApi<Block>
		+ StorageProvider<Block, BE>
		+ sc_client_api::BlockBackend<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Sync
		+ Send
		+ 'static,
	C: BlockchainEvents<Block>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_mmr_rpc::MmrRuntimeApi<Block, <Block as sp_runtime::traits::Block>::Hash, BlockNumber>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: RRSCApi<Block>,
	C::Api: BlockBuilder<Block>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	P: TransactionPool<Block = Block> + 'static,
	SC: SelectChain<Block> + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashFor<Block>>,
	// A: ChainApi<Block = Block> + 'static,
{
	use fc_rpc::{
		Eth, EthApiServer, EthDevSigner, EthFilter, EthFilterApiServer, EthPubSub,
		EthPubSubApiServer, EthSigner, Net, NetApiServer, Web3,	Web3ApiServer,
	};
	use pallet_mmr_rpc::{Mmr, MmrApiServer};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use cessc_consensus_rrsc_rpc::{ RRSC, RRSCApiServer };
	use sc_finality_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_rpc::dev::{Dev, DevApiServer};
	use cessc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	use substrate_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		select_chain,
		chain_spec,
		deny_unsafe,
		rrsc,
		grandpa,
		// graph,
		// is_authority,
		// enable_dev_signer,
		// network,
		// filter_pool,
		// frontier_backend,
		// max_past_logs,
		// fee_history_limit,
		// fee_history_cache,
		// overrides,
		// block_data_cache,
		// execute_gas_limit_multiplier,
	} = deps;
	let RRSCDeps { keystore, rrsc_config, shared_epoch_changes } = rrsc;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	io.merge(System::new(
		client.clone(),
		pool,
		deny_unsafe).into_rpc()
	)?;
	// Making synchronous calls in light client freezes the browser currently,
	// more context: https://github.com/paritytech/substrate/pull/3480
	// These RPCs should use an asynchronous caller instead.
	// io.merge(Contracts::new(client.clone()).into_rpc())?;
	io.merge(Mmr::new(client.clone()).into_rpc())?;
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		RRSC::new(
			client.clone(),
			shared_epoch_changes.clone(),
			keystore,
			rrsc_config,
			select_chain,
			deny_unsafe,
		)
		.into_rpc(),
	)?;
	io.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;
	io.merge(StateMigration::new(client.clone(), backend, deny_unsafe).into_rpc())?;
	io.merge(
		SyncState::new(chain_spec, client.clone(), shared_authority_set, shared_epoch_changes)?
				.into_rpc(),
	)?;
	io.merge(Dev::new(client.clone(), deny_unsafe).into_rpc())?;

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `io.extend_with(YourRpcTrait::to_delegate(YourRpcStruct::new(ReferenceToClient, ...)));`

	// let mut signers = Vec::new();
	// if enable_dev_signer {
	// 	signers.push(Box::new(EthDevSigner::new()) as Box<dyn EthSigner>);
	// }

	// io.merge(
	// 	Eth::new(
	// 		client.clone(),
	// 		pool.clone(),
	// 		graph,
	// 		Some(cess_node_runtime::TransactionConverter),
	// 		network.clone(),
	// 		signers,
	// 		overrides.clone(),
	// 		frontier_backend.clone(),
	// 		is_authority,
	// 		block_data_cache.clone(),
	// 		fee_history_cache,
	// 		fee_history_limit,
	// 		execute_gas_limit_multiplier
	// 	).into_rpc()
	// )?;

	// if let Some(filter_pool) = filter_pool {
	// 	io.merge(
	// 		EthFilter::new(
	// 			client.clone(),
	// 			frontier_backend,
	// 			filter_pool.clone(),
	// 			500 as usize, // max stored filters
	// 			max_past_logs,
	// 			block_data_cache.clone()
	// 		).into_rpc(),
	// 	)?;
	// }

	// io.merge(
	// 	Net::new(
	// 		client.clone(),
	// 		network.clone(),
	// 		// Whether to format the `peer_count` response as Hex (default) or not.
	// 		true,
	// 	).into_rpc()
	// )?;

	io.merge(Web3::new(client.clone()).into_rpc())?;

	Ok(io)
}
