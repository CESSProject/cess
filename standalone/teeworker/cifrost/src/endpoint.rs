use crate::{
    chain_client,
    types::{CesealClient, ParachainApi, SrSigner},
    Args,
};
use anyhow::{anyhow, Context, Result};
use ces_types::WorkerAction;
use cestory_api::crpc::SetEndpointRequest;
use cesxt::subxt::config::polkadot::PolkadotExtrinsicParamsBuilder as Params;
use log::{error, info};
use parity_scale_codec::Decode;
use tonic::Request;

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
    info!("worker's endpoint updated on chain");
    signer.increment_nonce();
    Ok(true)
}

pub async fn try_update_worker_endpoint(
    cc: &mut CesealClient,
    para_api: &ParachainApi,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<bool> {
    let mut info = cc.get_endpoint_info(()).await?.into_inner();
    let encoded_endpoint_payload = match info.encoded_endpoint_payload {
        None => {
            // set endpoint if public_endpoint arg configed
            if let Some(endpoint) = args.public_endpoint.clone() {
                match cc
                    .set_endpoint(Request::new(SetEndpointRequest::new(endpoint)))
                    .await
                {
                    Ok(resp) => {
                        let response = resp.into_inner();
                        info.signature = response.signature;
                        response
                            .encoded_endpoint_payload
                            .ok_or(anyhow!("BUG: can't be None"))?
                    }
                    Err(e) => {
                        error!("call ceseal.set_endpoint() response error: {:?}", e);
                        return Ok(false);
                    }
                }
            } else {
                return Ok(false);
            }
        }
        Some(payload) => {
            // update endpoint if the public_endpoint arg changed
            let former: WorkerAction =
                Decode::decode(&mut &payload[..]).context("decode payload error")?;
            if let WorkerAction::UpdateEndpoint(former) = former {
                match args.public_endpoint.clone() {
                    Some(endpoint) => {
                        if former.endpoint != Some(endpoint.clone()) || former.endpoint.is_none() {
                            match cc
                                .set_endpoint(Request::new(SetEndpointRequest::new(endpoint)))
                                .await
                            {
                                Ok(resp) => resp
                                    .into_inner()
                                    .encoded_endpoint_payload
                                    .ok_or(anyhow!("BUG: can't be None"))?,
                                Err(e) => {
                                    error!("call ceseal.set_endpoint() response error: {:?}", e);
                                    return Ok(false);
                                }
                            }
                        } else {
                            payload
                        }
                    }
                    None => payload,
                }
            } else {
                error!("call ceseal.set_endpoint() payload type error");
                return Ok(false);
            }
        }
    };

    //TODO: Only update on chain if the endpoint changed
    let signature = info
        .signature
        .ok_or_else(|| anyhow!("No endpoint signature"))?;
    info!("Binding worker's endpoint...");
    update_worker_endpoint(para_api, encoded_endpoint_payload, signature, signer, args).await
}
