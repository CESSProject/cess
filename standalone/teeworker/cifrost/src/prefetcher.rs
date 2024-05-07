use anyhow::Result;
use cestory_api::blocks::{BlockHeader, BlockHeaderWithChanges};
use cesxt::{BlockNumber, ChainApi};
use tokio::task::JoinHandle;

struct StoragePrefetchState {
    from: BlockNumber,
    to: BlockNumber,
    handle: JoinHandle<Result<Vec<BlockHeaderWithChanges>>>,
}

pub struct PrefetchClient {
    prefetching_storage_changes: Option<StoragePrefetchState>,
}

impl PrefetchClient {
    pub fn new() -> Self {
        Self {
            prefetching_storage_changes: None,
        }
    }

    pub async fn fetch_storage_changes(
        &mut self,
        client: &ChainApi,
        from: BlockNumber,
        to: BlockNumber,
    ) -> Result<Vec<BlockHeaderWithChanges>> {
        let count = to + 1 - from;
        let result = if let Some(state) = self.prefetching_storage_changes.take() {
            if state.from == from && state.to == to {
                log::info!("use prefetched storage changes ({from}-{to})",);
                state.handle.await?.ok()
            } else {
                log::info!(
                    "cancelling the prefetch ({}-{}), requesting ({from}-{to})",
                    state.from,
                    state.to,
                );
                state.handle.abort();
                None
            }
        } else {
            None
        };

        let result = if let Some(result) = result {
            result
        } else {
            fetch_storage_changes(client, from, to).await?
        };
        let next_from = from + count;
        let next_to = next_from + count - 1;
        let client = client.clone();
        self.prefetching_storage_changes = Some(StoragePrefetchState {
            from: next_from,
            to: next_to,
            handle: tokio::spawn(async move {
                log::info!("prefetching ({next_from}-{next_to})");
                fetch_storage_changes(&client, next_from, next_to).await
            }),
        });
        Ok(result)
    }
}

pub async fn fetch_storage_changes(
    client: &ChainApi,
    from: BlockNumber,
    to: BlockNumber,
) -> Result<Vec<BlockHeaderWithChanges>> {
    log::info!("fetch_storage_changes ({from}-{to})");
    if to < from {
        return Ok(vec![]);
    }
    let from_hash = crate::chain_client::get_header_hash(client, Some(from)).await?;
    let to_hash = crate::chain_client::get_header_hash(client, Some(to)).await?;
    let storage_changes = crate::chain_client::fetch_storage_changes(client, &from_hash, &to_hash)
        .await?
        .into_iter()
        .enumerate()
        .map(|(offset, storage_changes)| {
            BlockHeaderWithChanges {
                // Headers are synced separately. Only the `number` is used in Ceseal while syncing blocks.
                block_header: BlockHeader {
                    number: from + offset as BlockNumber,
                    parent_hash: Default::default(),
                    state_root: Default::default(),
                    extrinsics_root: Default::default(),
                    digest: Default::default(),
                },
                storage_changes,
            }
        })
        .collect();
    Ok(storage_changes)
}
