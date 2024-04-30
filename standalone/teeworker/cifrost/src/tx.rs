use crate::{
    chain_client,
    types::{CesealClient, SrSigner},
    Args,
};
use anyhow::{anyhow, Context, Result};
use cesxt::{subxt::config::polkadot::PolkadotExtrinsicParamsBuilder as Params, ChainApi};
use log::debug;

pub async fn try_apply_master_key(
    cc: &mut CesealClient,
    chain_api: &ChainApi,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<bool> {
    let apply_info = cc.get_master_key_apply(()).await?.into_inner();
    let Some(payload) = apply_info.encoded_payload else {
        return Ok(false);
    };
    let signature = apply_info
        .signature
        .ok_or_else(|| anyhow!("No master key apply signature"))?;
    debug!("appling master key...");
    apply_master_key(chain_api, payload, signature, signer, args).await
}

async fn apply_master_key(
    chain_api: &ChainApi,
    encoded_payload: Vec<u8>,
    signature: Vec<u8>,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<bool> {
    chain_client::update_signer_nonce(chain_api, signer).await?;
    let latest_block = chain_api.blocks().at_latest().await?;
    let tx_params = Params::new()
        .tip(args.tip)
        .mortal(latest_block.header(), args.longevity)
        .build();
    let tx = cesxt::dynamic::tx::apply_master_key(encoded_payload, signature);
    let tx_progress = chain_api
        .tx()
        .create_signed_with_nonce(&tx, &signer.signer, signer.nonce(), tx_params)?
        .submit_and_watch()
        .await?;
    let tx_in_block = tx_progress
        .wait_for_in_block()
        .await
        .context("tx progress wait for in block")?;
    debug!(
        "call tx apply_master_key, txn: {}, block hash: {}",
        tx_in_block.extrinsic_hash(),
        tx_in_block.block_hash()
    );
    let _ = tx_in_block
        .wait_for_success()
        .await
        .context("tx in block wait for success")?;
    signer.increment_nonce();
    Ok(true)
}
