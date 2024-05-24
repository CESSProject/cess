use ces_pallet_mq_runtime_api::MqApi;
use jsonrpsee::{
    core::async_trait,
    proc_macros::rpc,
    types::{ErrorObject, ErrorObjectOwned},
};
use parity_scale_codec::Encode;
use sc_client_api::{
    blockchain::{HeaderBackend, HeaderMetadata},
    Backend, BlockBackend, StorageProvider,
};
use sc_transaction_pool_api::{InPoolTransaction, TransactionPool};
use sp_api::{ApiExt, Core, ProvideRuntimeApi};
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, Header},
};
use sp_state_machine::backend::Backend as StateBackend;
use std::{fmt::Display, marker::PhantomData, result::Result, sync::Arc};

pub use storage_changes::{
    GetStorageChangesResponse, GetStorageChangesResponseWithRoot, MakeInto, StorageChanges,
    StorageChangesWithRoot,
};

mod mq_seq;
mod storage_changes;

/// Base code for all errors.
const RPC_ERR_BASE: i32 = 20000;

/// Top-level error type for the RPC handler.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid sender")]
    InvalidSender,

    #[error("{0}")]
    ApiError(#[from] sp_api::ApiError),

    /// Provided block range couldn't be resolved to a list of blocks.
    #[error("Cannot resolve a block range ['{from}' ... '{to}].")]
    InvalidBlockRange {
        /// Beginning of the block range.
        from: String,
        /// End of the block range.
        to: String,
    },

    /// Aborted due resource limiting such as MAX_NUMBER_OF_BLOCKS.
    #[error("Resource limited, {0}.")]
    ResourceLimited(String),

    /// Error occurred while processing some block.
    #[error("Error occurred while processing the block {0}.")]
    InvalidBlock(String),

    /// The RPC is unavailable.
    #[error("This RPC is unavailable. {0}")]
    Unavailable(String),

    /// Errors that can be formatted as a String
    #[error("{0}")]
    StringError(String),
}

impl Error {
    fn invalid_block<Block: BlockT, E: Display>(id: BlockId<Block>, error: E) -> Self {
        Self::InvalidBlock(format!("{id}: {error}"))
    }
}

impl From<Error> for ErrorObjectOwned {
    fn from(error: Error) -> Self {
        match error {
            Error::InvalidSender => {
                ErrorObject::owned(RPC_ERR_BASE + 1, error.to_string(), None::<()>)
            }
            Error::ApiError(e) => ErrorObject::owned(RPC_ERR_BASE + 2, e.to_string(), None::<()>),
            Error::InvalidBlockRange { .. } => {
                ErrorObject::owned(RPC_ERR_BASE + 3, error.to_string(), None::<()>)
            }
            Error::ResourceLimited(e) => {
                ErrorObject::owned(RPC_ERR_BASE + 4, e.to_string(), None::<()>)
            }
            Error::InvalidBlock(e) => {
                ErrorObject::owned(RPC_ERR_BASE + 5, e.to_string(), None::<()>)
            }
            Error::Unavailable(e) => {
                ErrorObject::owned(RPC_ERR_BASE + 6, e.to_string(), None::<()>)
            }
            Error::StringError(e) => ErrorObject::owned(RPC_ERR_BASE + 7, e, None::<()>),
        }
    }
}

#[rpc(server, namespace = "ces")]
pub trait NodeRpcExtApi<BlockHash> {
    /// Return the storage changes made by each block one by one from `from` to `to`(both
    /// inclusive). To get better performance, the client should limit the amount of requested block
    /// properly. 100 blocks for each call should be OK. REQUESTS FOR TOO LARGE NUMBER OF BLOCKS
    /// WILL BE REJECTED.
    #[method(name = "getStorageChanges")]
    fn get_storage_changes(
        &self,
        from: BlockHash,
        to: BlockHash,
    ) -> Result<GetStorageChangesResponse, Error>;

    /// Same as get_storage_changes but also return the state root of each block.
    #[method(name = "getStorageChangesWithRoot")]
    fn get_storage_changes_with_root(
        &self,
        from: BlockHash,
        to: BlockHash,
    ) -> Result<GetStorageChangesResponseWithRoot, Error>;

    /// Get storage changes made by given block.
    /// Returns `hex_encode(scale_encode(StorageChanges))`
    #[method(name = "getStorageChangesAt")]
    fn get_storage_changes_at(&self, block: BlockHash) -> Result<String, Error>;

    /// Return the next mq sequence number for given sender which take the ready transactions in
    /// count.
    #[method(name = "getMqNextSequence")]
    fn get_mq_seq(&self, sender_hex: String) -> Result<u64, Error>;
}

/// Stuffs for custom RPC
pub struct NodeRpcExt<BE, Block: BlockT, Client, P> {
    client: Arc<Client>,
    backend: Arc<BE>,
    pool: Arc<P>,
    _phantom: PhantomData<Block>,
}

impl<BE, Block: BlockT, Client, P> NodeRpcExt<BE, Block, Client, P>
where
    BE: Backend<Block> + 'static,
    Client: StorageProvider<Block, BE>
        + HeaderBackend<Block>
        + BlockBackend<Block>
        + HeaderMetadata<Block, Error = sp_blockchain::Error>
        + ProvideRuntimeApi<Block>
        + 'static,
    Block: BlockT + 'static,
    Client::Api: sp_api::Metadata<Block> + ApiExt<Block>,
    Client::Api: MqApi<Block>,
    <<Block as BlockT>::Header as Header>::Number: Into<u64>,
    P: TransactionPool + 'static,
{
    pub fn new(client: Arc<Client>, backend: Arc<BE>, pool: Arc<P>) -> Self {
        Self {
            client,
            backend,
            pool,
            _phantom: Default::default(),
        }
    }
}

#[async_trait]
impl<BE: 'static, Block: BlockT, Client: 'static, P> NodeRpcExtApiServer<Block::Hash>
    for NodeRpcExt<BE, Block, Client, P>
where
    BE: Backend<Block>,
    Client: StorageProvider<Block, BE>
        + HeaderBackend<Block>
        + BlockBackend<Block>
        + HeaderMetadata<Block, Error = sp_blockchain::Error>
        + ProvideRuntimeApi<Block>,
    Client::Api: sp_api::Metadata<Block> + ApiExt<Block>,
    Client::Api: MqApi<Block>,
    Block: BlockT + 'static,
    <<Block as BlockT>::Header as Header>::Number: Into<u64>,
    P: TransactionPool + 'static,
{
    fn get_storage_changes(
        &self,
        from: Block::Hash,
        to: Block::Hash,
    ) -> Result<GetStorageChangesResponse, Error> {
        let changes = storage_changes::get_storage_changes(
            self.client.as_ref(),
            self.backend.as_ref(),
            from,
            to,
            false,
        )?;
        Ok(changes.into_iter().map(|c| c.changes).collect())
    }

    fn get_storage_changes_with_root(
        &self,
        from: Block::Hash,
        to: Block::Hash,
    ) -> Result<GetStorageChangesResponseWithRoot, Error> {
        Ok(storage_changes::get_storage_changes(
            self.client.as_ref(),
            self.backend.as_ref(),
            from,
            to,
            true,
        )?)
    }

    fn get_storage_changes_at(&self, block: Block::Hash) -> Result<String, Error> {
        let changes = self.get_storage_changes(block, block)?;
        // get_storage_changes never returns empty vec without error.
        let encoded = changes[0].encode();
        Ok(impl_serde::serialize::to_hex(&encoded, false))
    }

    fn get_mq_seq(&self, sender_hex: String) -> Result<u64, Error> {
        let result = mq_seq::get_mq_seq(&*self.client, &self.pool, sender_hex);
        Ok(result?)
    }
}
