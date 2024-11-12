use polkadot_sdk::*;
use codec::Codec;
// Substrate
use sc_executor::WasmExecutor;
use sp_runtime::traits::{Block as BlockT, MaybeDisplay};

use crate::eth::EthCompatRuntimeApiCollection;
use ces_pallet_mq_runtime_api::MqApi;

/// Full backend.
pub(crate) type FullBackend<B> = sc_service::TFullBackend<B>;
/// Full client.
pub(crate) type FullClient<B, RA, HF> = sc_service::TFullClient<B, RA, WasmExecutor<HF>>;

/// A set of APIs that every runtimes must implement.
#[allow(dead_code)]
pub trait BaseRuntimeApiCollection<Block: BlockT>:
	sp_api::ApiExt<Block>
	+ sp_api::Metadata<Block>
	+ sp_block_builder::BlockBuilder<Block>
	+ sp_offchain::OffchainWorkerApi<Block>
	+ sp_session::SessionKeys<Block>
	+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
{
}

impl<Block, Api> BaseRuntimeApiCollection<Block> for Api
where
	Block: BlockT,
	Api: sp_api::ApiExt<Block>
		+ sp_api::Metadata<Block>
		+ sp_block_builder::BlockBuilder<Block>
		+ sp_offchain::OffchainWorkerApi<Block>
		+ sp_session::SessionKeys<Block>
		+ sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block>
{
}

/// A set of APIs that template runtime must implement.
#[allow(dead_code)]
pub trait RuntimeApiCollection<
	Block: BlockT,
	AccountId: Codec,
	Nonce: Codec,
	Balance: Codec + MaybeDisplay,
>:
	BaseRuntimeApiCollection<Block>
	+ EthCompatRuntimeApiCollection<Block>
	+ cessc_consensus_rrsc::RRSCApi<Block>
	+ sp_consensus_grandpa::GrandpaApi<Block>
	+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
	+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
	+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
	+ MqApi<Block>
{
}

impl<Block, AccountId, Nonce, Balance, Api>
	RuntimeApiCollection<Block, AccountId, Nonce, Balance> for Api
where
	Block: BlockT,
	AccountId: Codec,
	Nonce: Codec,
	Balance: Codec + MaybeDisplay,
	Api: BaseRuntimeApiCollection<Block>
		+ EthCompatRuntimeApiCollection<Block>
		+ cessc_consensus_rrsc::RRSCApi<Block>
		+ sp_consensus_grandpa::GrandpaApi<Block>
		+ sp_authority_discovery::AuthorityDiscoveryApi<Block>
		+ frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce>
		+ pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance>
		+ MqApi<Block>
{
}
