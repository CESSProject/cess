use crate::types::CesealClient;
use anyhow::{Context, Error, Result};
use cesxt::ChainApi;
use log::{error, info};
use std::time::Duration;
use tokio::time::sleep;

pub async fn spawn_subscriber(chain_client: &ChainApi, ceseal_client: CesealClient) -> Result<()> {
    let mut blocks_sub = chain_client.blocks().subscribe_finalized().await?;
    tokio::spawn(async move {
        while let Some(block) = blocks_sub.next().await {
            let block = block?;
            let block_number = block.header().number;
            let extrinsics = block.extrinsics().await?;
            for ext in extrinsics.iter() {
                let events = ext.events().await?;
                for evt in events.iter() {
                    let evt = evt?;
                    let pallet_name = evt.pallet_name();
                    let event_name = evt.variant_name();
                    if pallet_name == "TeeWorker" && event_name == "CesealBinAdded" {
                        info!("catch event: {pallet_name}_{event_name} on block: #{block_number}");
                        handle_checkpoint_take(ceseal_client.clone(), block_number).await;
                    }
                }
            }
        }
        Ok::<(), Error>(())
    });
    Ok(())
}

async fn handle_checkpoint_take(mut ceseal_client: CesealClient, block_number: u32) {
    info!("handling CesealBinAdded event, waiting ceseal arrive to block: #{block_number}");
    let handler = async move {
        loop {
            //FIXME: Maybe uniform the ceseal state query as stream fetch
            let info = ceseal_client
                .get_info(())
                .await
                .context("call ceseal.get_info() failed")?
                .into_inner();
            if info.blocknum >= block_number {
                ceseal_client
                    .take_checkpoint(())
                    .await
                    .context("call ceseal.take_checkpoint() failed")?;
                info!("ceseal has taked checkpoint at block: #{block_number}");
                break;
            }
            sleep(Duration::from_millis(5000)).await;
        }
        Ok::<(), Error>(())
    };
    tokio::spawn(async move {
        if let Err(error) = handler.await {
            error!("handle_checkpoint_take() failed: {error}");
        }
    });
}
