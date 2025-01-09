use crate::{rpc::ExtraRpcMethods, BlockNumber, Config, Hash, SubxtOnlineClient};
use anyhow::{anyhow, Context, Result};
use ces_types::WorkerPublicKey;
use jsonrpsee::{async_client::ClientBuilder, client_transport::ws::WsTransportClientBuilder};
use parity_scale_codec::{Decode, Encode};
use std::{ops::Deref, sync::Arc};
use subxt::{
    backend::{
        legacy::{LegacyBackend, LegacyRpcMethods},
        rpc::RpcClient as SubxtRpcClient,
    },
    dynamic::Value,
    storage::{Storage, StorageKey},
};

pub async fn connect(uri: &str) -> Result<ChainApi> {
    let wsc = ws_client(uri).await?;
    let rpc_client = SubxtRpcClient::new(wsc);
    let rpc_methods = LegacyRpcMethods::<Config>::new(rpc_client.clone());
    let extra_rpc_methods = ExtraRpcMethods::<Config>::new(rpc_client.clone());
    let backend = LegacyBackend::builder().build(rpc_client);
    let client = SubxtOnlineClient::from_backend(Arc::new(backend))
        .await
        .context("Failed to connect to substrate")?;

    let update_client = client.updater();
    tokio::spawn(async move {
        let result = update_client.perform_runtime_updates().await;
        eprintln!("Runtime update failed with result={result:?}");
    });

    Ok(ChainApi {
        client,
        extra_rpc_methods,
        rpc_methods,
    })
}

const MAX_REQUEST_SIZE: u32 = 8 << 20; // 8MiB
const MAX_RESPONSE_SIZE: u32 = 128 << 20; //128MiB

async fn ws_client(url: &str) -> Result<jsonrpsee::async_client::Client> {
    use jsonrpsee::client_transport::ws::Url;
    let url = Url::parse(url).context("Invalid websocket url")?;
    let (sender, receiver) = WsTransportClientBuilder::default()
        .max_request_size(MAX_REQUEST_SIZE)
        .max_response_size(MAX_RESPONSE_SIZE)
        .build(url)
        .await
        .context("Failed to build ws transport")?;
    Ok(ClientBuilder::default().build_with_tokio(sender, receiver))
}

#[derive(Clone)]
pub struct ChainApi {
    client: SubxtOnlineClient,
    extra_rpc_methods: ExtraRpcMethods<Config>,
    rpc_methods: LegacyRpcMethods<Config>,
}

impl Deref for ChainApi {
    type Target = SubxtOnlineClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl ChainApi {
    pub fn extra_rpc(&self) -> &ExtraRpcMethods<Config> {
        &self.extra_rpc_methods
    }

    pub fn rpc(&self) -> &LegacyRpcMethods<Config> {
        &self.rpc_methods
    }

    async fn storage_at(&self, hash: Option<Hash>) -> Result<Storage<Config, SubxtOnlineClient>> {
        let snap = match hash {
            Some(hash) => self.storage().at(hash),
            None => self.storage().at_latest().await?,
        };
        Ok(snap)
    }

    pub fn storage_key(
        &self,
        pallet_name: &str,
        entry_name: &str,
        keys: impl StorageKey,
    ) -> Result<Vec<u8>> {
        let address = subxt::dynamic::storage(pallet_name, entry_name, keys);
        Ok(self.storage().address_bytes(&address)?)
    }

    pub async fn current_set_id(&self, block_hash: Option<Hash>) -> Result<u64> {
        let address = subxt::dynamic::storage("Grandpa", "CurrentSetId", ());
        let set_id = self
            .storage_at(block_hash)
            .await?
            .fetch(&address)
            .await
            .context("Failed to get current set_id")?
            .ok_or_else(|| anyhow!("No set id"))?;
        Ok(set_id
            .to_value()?
            .as_u128()
            .ok_or_else(|| anyhow!("Invalid set id"))? as _)
    }

    pub async fn is_worker_registered_at(
        &self,
        worker: &[u8],
        block_number: Option<BlockNumber>,
    ) -> Result<bool> {
        use subxt::backend::legacy::rpc_methods::NumberOrHex;
        let hash = self
            .rpc_methods
            .chain_get_block_hash(block_number.map(|n| NumberOrHex::Number(n as u64)))
            .await?
            .ok_or_else(|| anyhow!("Block number not found"))?;
        let worker = Value::from_bytes(worker);
        let address = subxt::dynamic::storage("TeeWorker", "Workers", vec![worker]);
        let registered = self
            .storage()
            .at(hash)
            .fetch(&address)
            .await
            .context("Failed to get worker info")?
            .is_some();
        Ok(registered)
    }

    pub async fn worker_added_at(&self, worker: &[u8]) -> Result<Option<BlockNumber>> {
        let worker = Value::from_bytes(worker);
        let address = subxt::dynamic::storage("TeeWorker", "WorkerAddedAt", vec![worker]);
        let Some(block) = self
            .storage()
            .at_latest()
            .await?
            .fetch(&address)
            .await
            .context("Failed to get worker info")?
        else {
            return Ok(None);
        };
        let block_number = block
            .to_value()?
            .as_u128()
            .ok_or_else(|| anyhow!("Invalid block number in WorkerAddedAt"))?;
        Ok(Some(block_number as _))
    }

    pub async fn is_master_key_launched(&self) -> Result<bool> {
        let key: Vec<Value> = vec![];
        let address = subxt::dynamic::storage("TeeWorker", "MasterPubkey", key);
        let launched = self
            .storage()
            .at_latest()
            .await?
            .fetch(&address)
            .await
            .context("Failed to get master public key")?
            .is_some();
        Ok(launched)
    }

    pub async fn is_grandpa_note_stalled(&self) -> Result<bool> {
        let key: Vec<Value> = vec![];
        let address = subxt::dynamic::storage("TeeWorker", "NoteStalled", key);
        let note_stalled = self
            .storage()
            .at_latest()
            .await?
            .fetch(&address)
            .await
            .context("Failed to get NoteStalled")?
            .is_some();
        Ok(note_stalled)
    }

    async fn fetch<K: Encode, V: Decode>(
        &self,
        pallet: &str,
        name: &str,
        key: Option<K>,
    ) -> Result<Option<V>> {
        let mut args = vec![];
        if let Some(key) = key {
            let key = Value::from_bytes(key.encode());
            args.push(key);
        }
        let address = subxt::dynamic::storage(pallet, name, args);
        let Some(data) = self
            .storage()
            .at_latest()
            .await?
            .fetch(&address)
            .await
            .context("Failed to get worker endpoints")?
        else {
            return Ok(None);
        };
        Ok(Some(Decode::decode(&mut &data.encoded()[..])?))
    }

    pub async fn get_endpoint(&self, worker: &WorkerPublicKey) -> Result<String> {
        let result = self.fetch("TeeWorker", "Endpoints", Some(worker)).await?;
        let Some(endpoint) = result else {
            return Err(anyhow!("the worker's endpoint not set yet"));
        };
        Ok(endpoint)
    }

    pub async fn storage_keys(&self, prefix: &[u8], hash: Option<Hash>) -> Result<Vec<Vec<u8>>> {
        let page = 100;
        let mut keys: Vec<Vec<u8>> = vec![];

        loop {
            let start_key = keys.last().map(|k| &k[..]);
            let result = self
                .rpc_methods
                .state_get_keys_paged(prefix, page, start_key, hash)
                .await?;
            if result.is_empty() {
                break;
            }
            result.into_iter().for_each(|key| keys.push(key));
        }
        Ok(keys)
    }
}
