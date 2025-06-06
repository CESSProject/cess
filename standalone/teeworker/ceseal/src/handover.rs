use anyhow::{Context, Result};
use cestory::{handover, Config};
use cestory_api::handover::handover_server_api_client::HandoverServerApiClient;
use cestory_pal::Platform;
use tonic::transport::Channel;
use tracing::info;

pub(crate) async fn handover_from<P: Platform>(
    config: Config,
    platform: P,
    from_url: &str,
) -> Result<()> {
    let mut handover_client = handover::new_handover_client(config, platform);
    let mut handover_server =
        HandoverServerApiClient::<Channel>::connect(from_url.to_string()).await?;
    info!("Requesting for challenge");
    let challenge = handover_server
        .create_challenge(())
        .await
        .context("Failed to create challenge")?
        .into_inner();
    info!("Challenge received");
    let response = handover_client
        .accept_challenge(challenge)
        .await
        .context("Failed to accept challenge")?;
    info!("Requesting for key");
    let encrypted_key = handover_server
        .start(response)
        .await
        .context("Failed to start handover")?
        .into_inner();
    info!("Key received");
    handover_client
        .receive(encrypted_key)
        .context("Failed to receive handover result")?;
    Ok(())
}
