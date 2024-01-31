use ces_node_rpc_ext_types::GetStorageChangesResponse;
use serde::{Deserialize, Serialize};
use serde_json::to_value as to_json_value;
use subxt::{
    backend::rpc::{rpc_params, RpcClient},
    Config, Error,
};

pub use subxt::ext::sp_core::storage::{StorageData, StorageKey};

pub struct ExtraRpcMethods<T> {
    client: RpcClient,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Clone for ExtraRpcMethods<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            _marker: self._marker,
        }
    }
}

impl<T: Config> ExtraRpcMethods<T> {
    /// Instantiate the extra RPC method interface.
    pub fn new(client: RpcClient) -> Self {
        ExtraRpcMethods {
            client,
            _marker: std::marker::PhantomData,
        }
    }

    /// Query storage changes
    pub async fn get_storage_changes(
        &self,
        from: &T::Hash,
        to: &T::Hash,
    ) -> Result<GetStorageChangesResponse, Error> {
        let params = rpc_params![to_json_value(from)?, to_json_value(to)?];
        self.client
            .request("ces_getStorageChanges", params)
            .await
            .map_err(Into::into)
    }

    /// Returns the keys with prefix, leave empty to get all the keys
    pub async fn storage_pairs(
        &self,
        prefix: StorageKey,
        hash: Option<T::Hash>,
    ) -> Result<Vec<(StorageKey, StorageData)>, Error> {
        let params = rpc_params![to_json_value(prefix)?, to_json_value(hash)?];
        let data = self.client.request("state_getPairs", params).await?;
        Ok(data)
    }

    /// Fetch block syncing status
    pub async fn system_sync_state(&self) -> Result<SyncState, Error> {
        self.client.request("system_syncState", rpc_params![]).await
    }

    pub async fn get_mq_next_sequence(&self, message_origin_hex: &str) -> Result<u64, Error> {
        let seq: u64 = self
            .client
            .request(
                "ces_getMqNextSequence",
                rpc_params![to_json_value(message_origin_hex)?],
            )
            .await?;
        Ok(seq)
    }
}

impl<T: Config> ExtraRpcMethods<T>
where
    T::AccountId: Serialize,
{
    /// Reads the next nonce of an account, considering the pending extrinsics in the txpool
    pub async fn account_nonce(&self, account: &T::AccountId) -> Result<u64, Error> {
        let params = rpc_params![to_json_value(account)?];
        self.client.request("system_accountNextIndex", params).await
    }
}

/// System sync state for a Substrate-based runtime
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncState {
    /// Height of the block at which syncing started.
    pub starting_block: u64,
    /// Height of the current best block of the node.
    pub current_block: u64,
    /// Height of the highest block learned from the network. Missing if no block is known yet.
    #[serde(default = "Default::default")]
    pub highest_block: Option<u64>,
}
