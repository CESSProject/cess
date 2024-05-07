use crate::{
    types::{utils::raw_proof, Block, Hash, Header, SrSigner, StorageKey, NumberOrHex, ConvertTo},
    Error,
};
use anyhow::{Context, Result};
use ces_node_rpc_ext::MakeInto as _;
use ces_trie_storage::ser::StorageChanges;
use ces_types::messaging::MessageOrigin;
use cestory_api::blocks::{AuthoritySet, AuthoritySetChange, StorageProof};
use cesxt::{subxt, BlockNumber, ChainApi};
use parity_scale_codec::{Decode, Encode};
use sp_consensus_grandpa::{AuthorityList, VersionedAuthorityList, GRANDPA_AUTHORITIES_KEY};
use tokio::time::sleep;
use std::time::Duration;
use log::info;

pub use sp_core::{twox_128, twox_64};

/// Gets a storage proof for a single storage item
pub async fn state_get_read_proof(
    api: &ChainApi,
    hash: Option<Hash>,
    storage_key: &[u8],
) -> Result<StorageProof> {
    api.rpc()
        .state_get_read_proof(vec![storage_key], hash)
        .await
        .map(raw_proof)
        .map_err(Into::into)
}

/// Gets a storage proof for a storage items
pub async fn read_proofs(
    api: &ChainApi,
    hash: Option<Hash>,
    storage_keys: impl IntoIterator<Item = &[u8]>,
) -> Result<StorageProof> {
    let mut keys = vec![];
    // Retrieve the actual storage keys in case they are prefixed
    for prefix in storage_keys {
        let full_keys = api.storage_keys(prefix, hash).await?;
        keys.extend(full_keys);
    }
    api.rpc()
        .state_get_read_proof(keys.iter().map(|k| &k[..]), hash)
        .await
        .map(raw_proof)
        .map_err(Into::into)
}

// Storage functions

/// Fetch storage changes made by given block.
pub async fn fetch_storage_changes(
    client: &ChainApi,
    from: &Hash,
    to: &Hash,
) -> Result<Vec<StorageChanges>> {
    let response = client
        .extra_rpc()
        .get_storage_changes(from, to)
        .await?
        .into_iter()
        .map(|changes| StorageChanges {
            // TODO: get rid of this convert
            main_storage_changes: changes.main_storage_changes.into_(),
            child_storage_changes: changes.child_storage_changes.into_(),
        })
        .collect();
    Ok(response)
}

/// Fetch the genesis storage.
pub async fn fetch_genesis_storage(api: &ChainApi) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let hash = Some(api.genesis_hash());
    fetch_genesis_storage_at(api, hash).await
}

async fn fetch_genesis_storage_at(
    api: &ChainApi,
    hash: Option<sp_core::H256>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let response = api
        .extra_rpc()
        .storage_pairs(StorageKey(vec![]), hash)
        .await?;
    let storage = response.into_iter().map(|(k, v)| (k.0, v.0)).collect();
    Ok(storage)
}

/// Fetch best next sequence for given sender considering the txpool
pub async fn mq_next_sequence(api: &ChainApi, sender: &MessageOrigin) -> Result<u64, subxt::Error> {
    let sender_scl = sender.encode();
    let sender_hex = hex::encode(sender_scl);
    api.extra_rpc().get_mq_next_sequence(&sender_hex).await
}

pub fn decode_parachain_heads(head: Vec<u8>) -> Result<Vec<u8>, Error> {
    Decode::decode(&mut head.as_slice()).or(Err(Error::FailedToDecode))
}

/// Updates the nonce from the mempool
pub async fn update_signer_nonce(api: &ChainApi, signer: &mut SrSigner) -> Result<()> {
    let account_id = signer.account_id().clone();
    let nonce = api.extra_rpc().account_nonce(&account_id).await?;
    signer.set_nonce(nonce);
    log::info!("Fetch account {} nonce={}", account_id, nonce);
    Ok(())
}

pub async fn search_suitable_genesis_for_worker(
    api: &ChainApi,
    pubkey: &[u8],
    prefer: Option<BlockNumber>,
) -> Result<(BlockNumber, Vec<(Vec<u8>, Vec<u8>)>)> {
    let ceil = match prefer {
        Some(ceil) => ceil,
        None => {
            let node_state = api
                .extra_rpc()
                .system_sync_state()
                .await
                .context("Failed to get system state")?;
            node_state.current_block as _
        }
    };
    let block = get_worker_unregistered_block(api, pubkey, ceil)
        .await
        .context("Failed to search state for worker")?;
    let block_hash = api
        .rpc()
        .chain_get_block_hash(Some(block.into()))
        .await
        .context("Failed to resolve block number")?
        .ok_or_else(|| anyhow::anyhow!("Block number {block} not found"))?;
    let genesis = fetch_genesis_storage_at(api, Some(block_hash))
        .await
        .context("Failed to fetch genesis storage")?;
    Ok((block, genesis))
}

async fn get_worker_unregistered_block(
    api: &ChainApi,
    worker: &[u8],
    latest_block: u32,
) -> Result<u32> {
    let added_at = api.worker_added_at(worker).await?;
    log::info!("Worker added at={added_at:?}");
    let block = added_at.unwrap_or(latest_block + 1).saturating_sub(1);
    log::info!("Choosing genesis state at {block} ");
    Ok(block)
}

pub async fn wait_until_synced(client: &ChainApi) -> Result<()> {
    loop {
        let state = client.extra_rpc().system_sync_state().await?;
        info!(
            "Checking synced: current={} highest={:?}",
            state.current_block, state.highest_block
        );
        if let Some(highest) = state.highest_block {
            if highest - state.current_block <= 8 {
                return Ok(());
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
}

pub async fn get_header_hash(client: &ChainApi, h: Option<u32>) -> Result<Hash> {
    let pos = h.map(|h| NumberOrHex::Number(h.into()));
    let hash = match pos {
        Some(_) => client
            .rpc()
            .chain_get_block_hash(pos)
            .await?
            .ok_or(Error::BlockHashNotFound)?,
        None => client.rpc().chain_get_finalized_head().await?,
    };
    Ok(hash)
}

pub async fn get_block_at(client: &ChainApi, h: Option<u32>) -> Result<(Block, Hash)> {
    let hash = get_header_hash(client, h).await?;
    let block = client
        .rpc()
        .chain_get_block(Some(hash))
        .await?
        .ok_or(Error::BlockNotFound)?;

    Ok((block.convert_to(), hash))
}

pub async fn get_header_at(client: &ChainApi, h: Option<u32>) -> Result<(Header, Hash)> {
    let hash = get_header_hash(client, h).await?;
    let header = client
        .rpc()
        .chain_get_header(Some(hash))
        .await?
        .ok_or(Error::BlockNotFound)?;

    Ok((header.convert_to(), hash))
}

///Get the authority through the block header hash and pass it to ceseal for verification
pub async fn get_authority_with_proof_at(
    chain_api: &ChainApi,
    hash: Hash,
) -> Result<AuthoritySetChange> {
    // Storage
    let id_key = cesxt::dynamic::storage_key("Grandpa", "CurrentSetId");
    // Authority set
    let value = chain_api
        .rpc()
        .state_get_storage(GRANDPA_AUTHORITIES_KEY, Some(hash))
        .await?
        .expect("No authority key found");
    let list: AuthorityList = VersionedAuthorityList::decode(&mut value.as_slice())
        .expect("Failed to decode VersionedAuthorityList")
        .into();

    // Set id
    let id = chain_api.current_set_id(Some(hash)).await?;
    // Proof
    let proof = crate::chain_client::read_proofs(
        chain_api,
        Some(hash),
        vec![GRANDPA_AUTHORITIES_KEY, &id_key],
    )
    .await?;
    Ok(AuthoritySetChange {
        authority_set: AuthoritySet { list, id },
        authority_proof: proof,
    })
}
