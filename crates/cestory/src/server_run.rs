use super::{
    expert, expert::CesealExpertStub, podr2, pois, pubkeys, types::CesealProperties, Ceseal, CesealSafeBox, RpcService,
};
use anyhow::{anyhow, Context as _, Result};
use cestory_api::{crpc::ceseal_api_server::CesealApiServer, ecall_args::InitArgs};
use serde::{de::DeserializeOwned, Serialize};
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};
use tonic::transport::Server;

pub async fn run_ceseal_server<Platform>(
    init_args: InitArgs,
    platform: Platform,
    inner_listener_addr: SocketAddr,
) -> Result<()>
where
    Platform: pal::Platform + Serialize + DeserializeOwned,
{
    let (expert_cmd_tx, expert_cmd_rx) = mpsc::channel(16);
    let cestory = if init_args.enable_checkpoint {
        match Ceseal::restore_from_checkpoint(&platform, &init_args) {
            Ok(Some(factory)) => {
                info!("Loaded checkpoint");
                CesealSafeBox::new_with(factory, Some(expert_cmd_tx))
            },
            Ok(None) => {
                info!("No checkpoint found");
                CesealSafeBox::new(platform, Some(expert_cmd_tx))
            },
            Err(err) => {
                anyhow::bail!("Failed to load checkpoint: {:?}", err);
            },
        }
    } else {
        info!("Checkpoint disabled.");
        CesealSafeBox::new(platform, Some(expert_cmd_tx))
    };

    cestory.lock(true, true).expect("Failed to lock Ceseal").init(init_args);
    info!("Enclave init OK");

    tokio::spawn(expert::run(cestory.clone(), expert_cmd_rx));

    let ceseal_service = RpcService::new_with(cestory);
    info!("Ceseal internal server will listening on {}", inner_listener_addr);
    Server::builder()
        .add_service(CesealApiServer::new(ceseal_service))
        .serve(inner_listener_addr)
        .await
        .expect("Ceseal server catch panic");

    info!("Ceseal server quited success");
    Ok(())
}

pub(crate) fn spawn_external_server<Platform>(
    ceseal: &mut Ceseal<Platform>,
    ceseal_props: CesealProperties,
    shutdown_rx: oneshot::Receiver<()>,
    stopped_tx: oneshot::Sender<()>,
) -> Result<()> {
    const MAX_ENCODED_MSG_SIZE: usize = 104857600; // 100MiB
    const MAX_DECODED_MSG_SIZE: usize = MAX_ENCODED_MSG_SIZE;

    let listener_addr = {
        let ip = ceseal.args.ip_address.as_ref().map_or("0.0.0.0", String::as_str);
        let port = ceseal.args.public_port.unwrap_or(19999);
        format!("{ip}:{port}").parse().unwrap()
    };

    let expert_cmd_sender = ceseal
        .expert_cmd_sender
        .clone()
        .ok_or(anyhow!("the ceseal.expert_cmd_sender must be set"))?;

    let chain_storage = ceseal
        .runtime_state
        .as_ref()
        .ok_or(anyhow!("ceseal runtime state uninitializtion"))?
        .chain_storage
        .read();
    let pois_param = chain_storage
        .get_pois_expender_param()
        .map(|(k, n, d)| (k as i64, n as i64, d as i64)) //TODO: the defined type (k,n,d) is mismatched between pois lib and chain runtime
        .ok_or(anyhow!("the pois expender param not config on chain"))?;
    info!("pois expender param on chain: {pois_param:?}");

    {
        use rsa::pkcs1::EncodeRsaPublicKey;
        debug!(
            "Successfully load podr2 key public key is: {:?}",
            hex::encode(&ceseal_props.master_key.rsa_public_key().to_pkcs1_der().unwrap().as_bytes())
        );
    }

    tokio::spawn(async move {
        let worker_role = ceseal_props.role.clone();
        let ceseal_expert = CesealExpertStub::new(ceseal_props, expert_cmd_sender);
        let podr2_srv = podr2::new_podr2_api_server(ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let podr2v_srv = podr2::new_podr2_verifier_api_server(ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let pois_srv = pois::new_pois_certifier_api_server(pois_param.clone(), ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let poisv_srv = pois::new_pois_verifier_api_server(pois_param, ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let pubkeys = pubkeys::new_pubkeys_provider_server(ceseal_expert);

        let mut server = Server::builder();
        let router = match worker_role {
            ces_types::WorkerRole::Full => server
                .add_service(pubkeys)
                .add_service(podr2_srv)
                .add_service(podr2v_srv)
                .add_service(pois_srv)
                .add_service(poisv_srv),
            ces_types::WorkerRole::Verifier =>
                server.add_service(pubkeys).add_service(podr2v_srv).add_service(poisv_srv),
            ces_types::WorkerRole::Marker => server.add_service(pubkeys).add_service(podr2_srv).add_service(pois_srv),
        };
        info!("the external server will listening on {} run with {:?} role", listener_addr, worker_role);
        let result = router
            .serve_with_shutdown(listener_addr, async {
                shutdown_rx.await.ok();
                info!("external server starts shutting down");
            })
            .await
            .context("start external server failed")?;
        stopped_tx.send(()).ok();
        Ok::<(), anyhow::Error>(result)
    });
    Ok(())
}
