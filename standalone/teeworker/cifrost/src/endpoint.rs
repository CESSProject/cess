use crate::{
    chain_client,
    types::{ParachainApi, CesealClient, SrSigner},
    Args,
};
use anyhow::{anyhow, Result};
use log::{error, info};
use cesxt::subxt::config::polkadot::PolkadotExtrinsicParamsBuilder as Params;

async fn update_worker_endpoint(
    para_api: &ParachainApi,
    encoded_endpoint_payload: Vec<u8>,
    signature: Vec<u8>,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<bool> {
    chain_client::update_signer_nonce(para_api, signer).await?;
    let latest_block = para_api.blocks().at_latest().await?;
    let tx_params = Params::new()
        .tip(args.tip)
        .mortal(latest_block.header(), args.longevity)
        .build();
    let tx = cesxt::dynamic::tx::update_worker_endpoint(encoded_endpoint_payload, signature);
    let ret = para_api
        .tx()
        .create_signed_with_nonce(&tx, &signer.signer, signer.nonce(), tx_params)?
        .submit_and_watch()
        .await;
    if ret.is_err() {
        error!("FailedToCallBindWorkerEndpoint: {:?}", ret);
        return Err(anyhow!("failed to call update_worker_endpoint"));
    }
    signer.increment_nonce();
    Ok(true)
}

pub async fn try_update_worker_endpoint(
    pr: &mut CesealClient,
    para_api: &ParachainApi,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<bool> {
    let info = pr.get_endpoint_info(()).await?.into_inner();
    let encoded_endpoint_payload = match info.encoded_endpoint_payload {
        None => return Ok(false), // Early return if no endpoint payload is available
        Some(payload) => payload,
    };
    let signature = info
        .signature
        .ok_or_else(|| anyhow!("No endpoint signature"))?;
    info!("Binding worker's endpoint...");
    update_worker_endpoint(para_api, encoded_endpoint_payload, signature, signer, args).await
}
