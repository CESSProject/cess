// Substrate
use crate::eth::EthCompatRuntimeApiCollection;
use ces_pallet_mq_runtime_api::MqApi;
use cess_node_primitives::{opaque::Block, AccountId, Balance, Nonce};
use sc_executor::{NativeElseWasmExecutor, NativeExecutionDispatch, NativeVersion};

/// Full backend.
pub type FullBackend = sc_service::TFullBackend<Block>;
/// Full client.
pub type FullClient<RuntimeApi, Executor> =
	sc_service::TFullClient<Block, RuntimeApi, NativeElseWasmExecutor<Executor>>;

pub type Client = FullClient<cess_node_runtime::RuntimeApi, CESSNodeRuntimeExecutor>;

/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions = frame_benchmarking::benchmarking::HostFunctions;
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = ();

pub struct CESSNodeRuntimeExecutor;
impl NativeExecutionDispatch for CESSNodeRuntimeExecutor {
	type ExtendHostFunctions = HostFunctions;

	fn dispatch(method: &str, data: &[u8]) -> Option<Vec<u8>> {
		cess_node_runtime::api::dispatch(method, data)
	}

	fn native_version() -> NativeVersion {
		cess_node_runtime::native_version()
	}
}

/// A set of APIs that every runtimes must implement.
pub trait BaseRuntimeApiCollection:
	sp_api::ApiExt<Block>
	+ sp_api::Metadata<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ sp_offchain::OffchainWorkerApi<Block>
	+ sp_session::SessionKeys<Block>
	+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
{
}

impl<Api> BaseRuntimeApiCollection for Api where
	Api: sp_api::ApiExt<Block>
		+ sp_api::Metadata<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
{
}

/// A set of APIs that template runtime must implement.
pub trait RuntimeApiCollection:
	BaseRuntimeApiCollection
	+ EthCompatRuntimeApiCollection
	+ cessc_consensus_rrsc::RRSCApi<Block>
	+ sp_consensus_grandpa::GrandpaApi<Block>
	+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ MqApi<Block>
{
}

impl<Api> RuntimeApiCollection for Api where
	Api: BaseRuntimeApiCollection
		+ EthCompatRuntimeApiCollection
		+ cessc_consensus_rrsc::RRSCApi<Block>
		+ sp_consensus_grandpa::GrandpaApi<Block>
		+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ MqApi<Block>
{
}
