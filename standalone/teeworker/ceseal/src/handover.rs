use crate::pal_gramine::GraminePlatform;
use anyhow::{Context, Result};
use cestory::RpcService;
use cestory_api::{
    ceseal_client::CesealClient, crpc::ceseal_api_server::CesealApi, ecall_args::InitArgs,
};
use tonic::{transport::Channel, Request};
use tracing::info;

pub(crate) async fn handover_from(url: &str, args: InitArgs) -> Result<()> {
    let this = RpcService::new(GraminePlatform);
    this.lock_ceseal(true, false)
        .expect("Failed to lock Ceseal")
        .init(args);

    let mut from_ceseal = CesealClient::<Channel>::connect(url.to_string()).await?;
    info!("Requesting for challenge");
    let challenge = from_ceseal
        .handover_create_challenge(())
        .await
        .context("Failed to create challenge")?
        .into_inner();
    info!("Challenge received");
    let response = this
        .handover_accept_challenge(Request::new(challenge))
        .await
        .context("Failed to accept challenge")?
        .into_inner();
    info!("Requesting for key");
    let encrypted_key = from_ceseal
        .handover_start(response)
        .await
        .context("Failed to start handover")?
        .into_inner();
    info!("Key received");
    this.handover_receive(Request::new(encrypted_key))
        .await
        .context("Failed to receive handover result")?;
    Ok(())
}
