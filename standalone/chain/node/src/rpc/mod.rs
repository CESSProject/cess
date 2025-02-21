//! A collection of node-specific RPC methods.
use polkadot_sdk::*;
use sc_consensus_grandpa as grandpa;
use cess_node_primitives::{opaque::Block, AccountId, Balance, BlockNumber, Hash, Nonce};
use sc_consensus_babe::BabeWorkerHandle;
use sc_consensus_babe_rpc::{BabeApiServer, Babe};
use sp_consensus_babe::BabeApi;
use grandpa::{FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState};
use jsonrpsee::RpcModule;
use sc_client_api::{
	backend::{Backend, StorageProvider},
	client::BlockchainEvents,
	AuxStore, UsageProvider,
};
pub use sc_rpc::SubscriptionTaskExecutor;
use sc_service::TransactionPool;
use sc_transaction_pool::ChainApi;
use sp_api::{CallApiAt, ProvideRuntimeApi};
use sp_application_crypto::RuntimeAppPublic;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_beefy::AuthorityIdBound;
use sc_consensus_beefy::communication::notification::{
	BeefyBestBlockStream, BeefyVersionedFinalityProofStream,
};
use sp_inherents::CreateInherentDataProviders;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

mod eth;
pub use self::eth::{create_eth, EthDeps};

/// Extra dependencies for Babe.
pub struct BabeDeps {
	/// A handle to the Babe worker for issuing requests.
	pub babe_worker_handle: BabeWorkerHandle<Block>,
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

/// Dependencies for BEEFY
pub struct BeefyDeps<AuthorityId: AuthorityIdBound> {
	/// Receives notifications about finality proof events from BEEFY.
	pub beefy_finality_proof_stream: BeefyVersionedFinalityProofStream<Block, AuthorityId>,
	/// Receives notifications about best block events from BEEFY.
	pub beefy_best_block_stream: BeefyBestBlockStream<Block>,
	/// Executor to drive the subscription manager in the BEEFY RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B, AuthorityId: AuthorityIdBound> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Babe specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// BEEFY specific dependencies.
	pub beefy: BeefyDeps<AuthorityId>,
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
	type RuntimeStorageOverride = fc_rpc::frontier_backend_client::SystemAccountId20StorageOverride<Block, C, BE>;
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, B, SC, P, A, CT, CIDP, AuthorityId>(
	FullDeps { 
		client, 
		pool, 
		select_chain, 
		chain_spec, 
		babe, 
		grandpa, 
		beefy,
		backend 
	}: FullDeps<C, P, SC, B, AuthorityId>,
	eth_deps: EthDeps<Block, C, P, A, CT, CIDP>,
	subscription_task_executor: SubscriptionTaskExecutor,	
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: CallApiAt<Block> + ProvideRuntimeApi<Block>,
	C: ProvideRuntimeApi<Block> + sc_client_api::BlockBackend<Block> + AuxStore + Sync + Send,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: sp_api::Metadata<Block>,
	C::Api: sp_block_builder::BlockBuilder<Block>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	C::Api: BabeApi<Block>,
	C::Api: ces_pallet_mq_runtime_api::MqApi<Block>,
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
	AuthorityId: AuthorityIdBound,
	<AuthorityId as RuntimeAppPublic>::Signature: Send + Sync,
{
	use ces_node_rpc_ext::{NodeRpcExt, NodeRpcExtApiServer};
	use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_consensus_beefy_rpc::{Beefy, BeefyApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	use substrate_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};

	let mut io = RpcModule::new(());
	let BabeDeps { keystore, babe_worker_handle } = babe;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	io.merge(System::new(client.clone(), pool.clone()).into_rpc())?;
	// Making synchronous calls in light client freezes the browser currently,
	// more context: https://github.com/paritytech/substrate/pull/3480
	// These RPCs should use an asynchronous caller instead.
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain).into_rpc())?;
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

	io.merge(SyncState::new(chain_spec, client.clone(), shared_authority_set, babe_worker_handle)?.into_rpc())?;

	io.merge(StateMigration::new(client.clone(), backend.clone()).into_rpc())?;
	io.merge(NodeRpcExt::new(client, backend, pool).into_rpc())
		.expect("Initialize CESS node RPC ext failed.");

	io.merge(
		Beefy::<Block, AuthorityId>::new(
			beefy.beefy_finality_proof_stream,
			beefy.beefy_best_block_stream,
			beefy.subscription_executor,
		)?
		.into_rpc(),
	)?;

	// Ethereum compatibility RPCs
	let io = create_eth::<_, _, _, _, _, _, _, DefaultEthConfig<C, B>>(
		io,
		eth_deps,
		subscription_task_executor,
	)?;

	Ok(io)
}
