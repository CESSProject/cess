use crate::pal_gramine::GraminePlatform;
use anyhow::{Context, Result};
use cestory::RpcService;
use cestory_api::{
    ecall_args::InitArgs, crpc::ceseal_api_server::CesealApi,
    ceseal_client::new_ceseal_client,
};
use tracing::info;

pub(crate) async fn handover_from(url: &str, args: InitArgs) -> Result<()> {
    let mut this = RpcService::new(GraminePlatform);
    this.lock_ceseal(true, false).expect("Failed to lock Ceseal").init(args);

    let from_ceseal = new_ceseal_client(url.into());
    info!("Requesting for challenge");
    let challenge = from_ceseal
        .handover_create_challenge(())
        .await
        .context("Failed to create challenge")?;
    info!("Challenge received");
    let response = this
        .handover_accept_challenge(challenge)
        .await
        .context("Failed to accept challenge")?;
    info!("Requesting for key");
    let encrypted_key = from_ceseal
        .handover_start(response)
        .await
        .context("Failed to start handover")?;
    info!("Key received");
    this.handover_receive(encrypted_key)
        .await
        .context("Failed to receive handover result")?;
    Ok(())
}
