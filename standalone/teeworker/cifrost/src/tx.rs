use crate::{
    chain_client,
    types::{CesealClient, SrSigner},
    Args,
};
use anyhow::{anyhow, Result};
use ces_types::attestation::legacy::Attestation;
use cestory_api::crpc;
use cesxt::{
    subxt::{config::polkadot::PolkadotExtrinsicParamsBuilder as Params, tx::Payload},
    ChainApi,
};
use log::{debug, info};
use parity_scale_codec::Encode;

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

    chain_api
        .tx()
        .sign_and_submit_then_watch(&tx, signer, tx_params)
        .await?
        .wait_for_finalized()
        .await
        .and_then(|tx| {
            debug!(
                "call tx apply_master_key, txn: {:?}, block hash: {:?}",
                tx.extrinsic_hash(),
                tx.block_hash()
            );
            Ok(tx)
        })?
        .wait_for_success()
        .await?;

    signer.increment_nonce();
    Ok(true)
}

pub async fn update_worker_endpoint(
    chain_api: &ChainApi,
    encoded_endpoint_payload: Vec<u8>,
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
    let tx = cesxt::dynamic::tx::update_worker_endpoint(encoded_endpoint_payload, signature);

    chain_api
        .tx()
        .sign_and_submit_then_watch(&tx, signer, tx_params)
        .await?
        .wait_for_finalized_success()
        .await?;

    info!("worker's endpoint updated on chain");
    signer.increment_nonce();
    Ok(true)
}

pub async fn register_worker(
    chain_api: &ChainApi,
    encoded_runtime_info: Vec<u8>,
    attestation: crpc::Attestation,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<()> {
    chain_client::update_signer_nonce(chain_api, signer).await?;
    let latest_block = chain_api.blocks().at_latest().await?;
    let tx_params = Params::new()
        .tip(args.tip)
        .mortal(latest_block.header(), args.longevity)
        .build();
    debug!(
        "tx mortal: (from: {:?}, for_blocks: {:?})",
        latest_block.header().number,
        args.longevity
    );
    let v2 = attestation.payload.is_none();
    debug!("attestation: {attestation:?}, v2: {v2:?}");
    let attestation = match attestation.payload {
        Some(payload) => Attestation::SgxIas {
            ra_report: payload.report.as_bytes().to_vec(),
            signature: payload.signature,
            raw_signing_cert: payload.signing_cert,
        }
        .encode(),
        None => attestation.encoded_report,
    };
    debug!("encoded attestation: {}", hex::encode(&attestation));
    let tx = cesxt::dynamic::tx::register_worker(encoded_runtime_info, attestation, v2);

    let encoded_call_data = tx
        .encode_call_data(&chain_api.metadata())
        .expect("should encoded");
    debug!("register_worker call: 0x{}", hex::encode(encoded_call_data));

    chain_api
        .tx()
        .sign_and_submit_then_watch(&tx, signer, tx_params)
        .await?
        .wait_for_finalized()
        .await
        .and_then(|tx| {
            info!(
                "Tee registration successful in block hash:{:?}, and the transaction hash is :{:?}",
                tx.block_hash(),
                tx.extrinsic_hash(),
            );
            Ok(tx)
        })?
        .wait_for_success()
        .await?;
    
    signer.increment_nonce();
    Ok(())
}


pub async fn update_worker_ra_report(
    chain_api: &ChainApi,
    encoded_runtime_info: Vec<u8>,
    attestation: crpc::Attestation,
    signer: &mut SrSigner,
    longevity: u64,
    tip:u128,
) -> Result<()> {
    chain_client::update_signer_nonce(chain_api, signer).await?;
    let latest_block = chain_api.blocks().at_latest().await?;
    let tx_params = Params::new()
        .tip(tip)
        .mortal(latest_block.header(), longevity)
        .build();
    debug!(
        "tx mortal: (from: {:?}, for_blocks: {:?})",
        latest_block.header().number,
        longevity
    );
    let attestation = match attestation.payload {
        Some(payload) => Attestation::SgxIas {
            ra_report: payload.report.as_bytes().to_vec(),
            signature: payload.signature,
            raw_signing_cert: payload.signing_cert,
        }
        .encode(),
        None => attestation.encoded_report,
    };
    debug!("encoded attestation: {}", hex::encode(&attestation));
    let tx = cesxt::dynamic::tx::refresh_tee_status(encoded_runtime_info, attestation);

    let encoded_call_data = tx
        .encode_call_data(&chain_api.metadata())
        .expect("should encoded");
    debug!("register_worker call: 0x{}", hex::encode(encoded_call_data));

    chain_api
        .tx()
        .sign_and_submit_then_watch(&tx, signer, tx_params)
        .await?
        .wait_for_finalized()
        .await
        .and_then(|tx| {
            info!(
                "Tee update ra report successful in block hash:{:?}, and the transaction hash is :{:?}",
                tx.block_hash(),
                tx.extrinsic_hash(),
            );
            Ok(tx)
        })?
        .wait_for_success()
        .await?;

    signer.increment_nonce();
    Ok(())
}