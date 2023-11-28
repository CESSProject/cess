//! A collection of node-specific RPC methods.

use std::sync::Arc;

use jsonrpsee::RpcModule;
// Substrate
use cessc_consensus_rrsc::RRSCWorkerHandle;
use cessc_consensus_rrsc_rpc::{RRSCApiServer, RRSC};
use cessp_consensus_rrsc::RRSCApi;
use grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_client_api::{
	backend::{Backend, StorageProvider},
	client::BlockchainEvents,
	AuxStore, UsageProvider,
};
use sc_rpc::SubscriptionTaskExecutor;
use sc_rpc_api::DenyUnsafe;
use sc_service::TransactionPool;
use sc_transaction_pool::ChainApi;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block as BlockT;
// Runtime
use cess_node_runtime::{opaque::Block, AccountId, Balance, BlockNumber, Hash, Nonce};

mod eth;
pub use self::eth::{create_eth, overrides_handle, EthDeps};

/// Extra dependencies for RRSC.
pub struct RRSCDeps {
	/// A handle to the RRSC worker for issuing requests.
	pub rrsc_worker_handle: RRSCWorkerHandle<Block>,
	/// The keystore that manages the keys of the node.
	pub keystore: KeystorePtr,
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
pub struct FullDeps<C, P, SC, B> {
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
	/// The backend used by the node.
	pub backend: Arc<B>,
}

pub struct DefaultEthConfig<C, BE>(std::marker::PhantomData<(C, BE)>);

impl<C, BE> fc_rpc::EthConfig<Block, C> for DefaultEthConfig<C, BE>
where
	C: StorageProvider<Block, BE> + Sync + Send + 'static,
	BE: Backend<Block> + 'static,
{
	type EstimateGasAdapter = ();
	type RuntimeStorageOverride =
		fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, B, SC, P, A, CT, CIDP>(
	FullDeps {
		client,
		pool,
		select_chain,
		chain_spec,
		deny_unsafe,
		rrsc,
		grandpa,
		backend,
	}: FullDeps<C, P, SC, B>,
	eth_deps: EthDeps<Block, C, P, A, CT, CIDP>,
	subscription_task_executor: SubscriptionTaskExecutor,
	pubsub_notification_sinks: Arc<
		fc_mapping_sync::EthereumBlockNotificationSinks<
			fc_mapping_sync::EthereumBlockNotification<Block>,
		>,
	>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: CallApiAt<Block> + ProvideRuntimeApi<Block>,
	C: ProvideRuntimeApi<Block> + sc_client_api::BlockBackend<Block> + AuxStore + Sync + Send,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: sp_block_builder::BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C::Api: RRSCApi<Block>,
	C: BlockchainEvents<Block> + 'static,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: AuxStore + UsageProvider<Block> + StorageProvider<Block, B>,
	SC: SelectChain<Block> + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<Block>>,
	P: TransactionPool<Block = Block> + 'static,
	A: ChainApi<Block = Block> + 'static,
	CIDP: CreateInherentDataProviders<Block, ()> + Send + 'static,
	CT: fp_rpc::ConvertTransaction<<Block as BlockT>::Extrinsic> + Send + Sync + 'static,
{
	use cessc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_rpc::dev::{Dev, DevApiServer};
	use sc_rpc_spec_v2::chain_spec::{ChainSpec, ChainSpecApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	use substrate_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};

	let mut io = RpcModule::new(());
	let RRSCDeps { keystore, rrsc_worker_handle } = rrsc;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	let chain_name = chain_spec.name().to_string();
	let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
	let properties = chain_spec.properties();
	io.merge(ChainSpec::new(chain_name, genesis_hash, properties).into_rpc())?;

	io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
	// Making synchronous calls in light client freezes the browser currently,
	// more context: https://github.com/paritytech/substrate/pull/3480
	// These RPCs should use an asynchronous caller instead.
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		RRSC::new(client.clone(), rrsc_worker_handle.clone(), keystore, select_chain, deny_unsafe)
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

	io.merge(
		SyncState::new(chain_spec, client.clone(), shared_authority_set, rrsc_worker_handle)?
			.into_rpc(),
	)?;

	io.merge(StateMigration::new(client.clone(), backend, deny_unsafe).into_rpc())?;
	io.merge(Dev::new(client, deny_unsafe).into_rpc())?;

	// Ethereum compatibility RPCs
	let io = create_eth::<_, _, _, _, _, _, _, DefaultEthConfig<C, B>>(
		io,
		eth_deps,
		subscription_task_executor,
		pubsub_notification_sinks,
	)?;

	Ok(io)
}
