use crate::{
    error::Error,
    types::{
        Args, BlockNumber, BlockSyncState, CesealClient, Header, RaOption, RunningFlags,
        SrSigner, SyncOperation,
    },
};
use anyhow::{anyhow, Context, Result};
use ces_types::{AttestationProvider, WorkerEndpointPayload};
use cestory_api::{
    blocks::{BlockHeaderWithChanges, GenesisBlockInfo, HeaderToSync},
    crpc::{self, InitRuntimeResponse},
};
use cesxt::{
    connect as subxt_connect,
    dynamic::storage_key,
    sp_core::{crypto::Pair, sr25519},
    ChainApi,
};
use clap::Parser;
use log::{error, info, warn};
use msg_sync::{Error as MsgSyncError, Receiver, Sender};
use parity_scale_codec::Decode;
use sc_consensus_grandpa::FinalityProof;
use sp_core::crypto::AccountId32;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use tonic::Request;

pub use authority::get_authority_with_proof_at;
pub use cesxt::subxt;

mod authority;
mod block_subscribe;
mod error;
mod msg_sync;
mod prefetcher;
mod tx;

pub mod chain_client;
pub mod types;

async fn sync_headers(cc: &mut CesealClient, api: &ChainApi, from: BlockNumber) -> Result<()> {
    let first_header = chain_client::get_header_at(api, Some(from)).await?;
    let mut headers = vec![HeaderToSync {
        header: first_header.0.clone(),
        justification: None,
    }];

    let encoded_finality_proof = prove_finality_at(api, from).await?;
    let finality_proof: FinalityProof<Header> =
        Decode::decode(&mut encoded_finality_proof.as_slice())?;
    headers.extend(finality_proof.unknown_headers.iter().map(|h| HeaderToSync {
        header: h.clone(),
        justification: None,
    }));

    let last_header = headers
        .last_mut()
        .expect("Already filled at least one header");
    let last_number = last_header.header.number;
    last_header.justification = Some(finality_proof.justification);

    info!(
        "sending a batch of {} headers (last: {})",
        headers.len(),
        last_number
    );
    let relay_synced_to = req_sync_header(cc, headers).await?;
    info!("  ..sync_header: {:?}", relay_synced_to);

    Ok(())
}

pub async fn prove_finality_at(api: &ChainApi, block_number: u32) -> Result<Vec<u8>> {
    let pos = cesxt::rpc::BlockNumberOrHex::Number(block_number.into());
    let proof = api.extra_rpc().prove_finality(pos).await?;
    Ok(proof.0)
}

pub async fn batch_sync_storage_changes(
    cc: &mut CesealClient,
    chain_api: &ChainApi,
    from: BlockNumber,
    to: BlockNumber,
    batch_size: BlockNumber,
) -> Result<()> {
    info!(
        "batch syncing from {from} to {to} ({} blocks)",
        to as i64 - from as i64 + 1
    );

    let mut fetcher = prefetcher::PrefetchClient::new();

    for from in (from..=to).step_by(batch_size as _) {
        let to = to.min(from.saturating_add(batch_size - 1));
        let storage_changes = fetcher.fetch_storage_changes(chain_api, from, to).await?;
        let r = req_dispatch_block(cc, storage_changes).await?;
        log::debug!("  ..dispatch_block: {:?}", r);
    }
    Ok(())
}

async fn try_load_handover_proof(pr: &mut CesealClient, chain_api: &ChainApi) -> Result<()> {
    let info = pr.get_info(()).await?.into_inner();
    if info.safe_mode_level < 2 {
        return Ok(());
    }
    if info.blocknum == 0 {
        return Ok(());
    }
    let current_block = info.blocknum - 1;
    let hash = chain_client::get_header_hash(chain_api, Some(current_block)).await?;
    let proof = chain_client::read_proofs(
        chain_api,
        Some(hash),
        vec![
            &storage_key("TeeWorker", "CesealBinAddedAt")[..],
            &storage_key("TeeWorker", "CesealBinAllowList")[..],
            &storage_key("Timestamp", "Now")[..],
        ],
    )
    .await
    .context("Failed to get handover proof")?;
    info!("Loading handover proof at {current_block}");
    for p in &proof {
        info!("key=0x{}", hex::encode(sp_core::blake2_256(p)));
    }
    pr.load_storage_proof(crpc::StorageProof { proof }).await?;
    Ok(())
}

async fn req_sync_header(
    cc: &mut CesealClient,
    headers: Vec<HeaderToSync>,
) -> Result<crpc::SyncedTo> {
    let resp = cc
        .sync_header(crpc::HeadersToSync::new(headers, None))
        .await?;
    Ok(resp.into_inner())
}

async fn req_dispatch_block(
    pr: &mut CesealClient,
    blocks: Vec<BlockHeaderWithChanges>,
) -> Result<crpc::SyncedTo> {
    let resp = pr.dispatch_blocks(crpc::Blocks::new(blocks)).await?;
    Ok(resp.into_inner())
}

const DEV_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";
#[allow(clippy::too_many_arguments)]
async fn init_runtime(
    chain_api: &ChainApi,
    ceseal_client: &mut CesealClient,
    attestation_provider: Option<AttestationProvider>,
    use_dev_key: bool,
    inject_key: &str,
    operator: Option<AccountId32>,
    start_header: BlockNumber,
) -> Result<InitRuntimeResponse> {
    let genesis_info = fetch_genesis_info(chain_api, start_header).await?;
    let genesis_state = chain_client::fetch_genesis_storage(chain_api).await?;
    let mut debug_set_key = None;
    if !inject_key.is_empty() {
        if inject_key.len() != 64 {
            panic!("inject-key must be 32 bytes hex");
        } else {
            info!("Inject key {}", inject_key);
        }
        debug_set_key = Some(hex::decode(inject_key).expect("Invalid dev key"));
    } else if use_dev_key {
        info!("Inject key {}", DEV_KEY);
        debug_set_key = Some(hex::decode(DEV_KEY).expect("Invalid dev key"));
    }

    let resp = ceseal_client
        .init_runtime(crpc::InitRuntimeRequest::new(
            attestation_provider.is_none(),
            genesis_info,
            debug_set_key,
            genesis_state,
            operator,
            attestation_provider,
        ))
        .await?;
    Ok(resp.into_inner())
}

pub async fn fetch_genesis_info(
    api: &ChainApi,
    genesis_block_number: BlockNumber,
) -> Result<GenesisBlockInfo> {
    let genesis_block = chain_client::get_block_at(api, Some(genesis_block_number))
        .await?
        .0
        .block;
    let set_proof = crate::get_authority_with_proof_at(api, &genesis_block.header).await?;
    Ok(GenesisBlockInfo {
        block_header: genesis_block.header,
        authority_set: set_proof.authority_set,
        proof: set_proof.authority_proof,
    })
}

async fn try_register_worker(
    ceseal_client: &mut CesealClient,
    chain_client: &ChainApi,
    signer: &mut SrSigner,
    operator: Option<AccountId32>,
    args: &Args,
) -> Result<bool> {
    let info = ceseal_client
        .get_runtime_info(crpc::GetRuntimeInfoRequest::new(false, operator))
        .await?
        .into_inner();
    if let Some(attestation) = info.attestation {
        info!("Registering worker...");
        tx::register_worker(
            chain_client,
            info.encoded_runtime_info,
            attestation,
            signer,
            args,
        )
        .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

async fn try_load_chain_state(
    pr: &mut CesealClient,
    chain_api: &ChainApi,
    args: &Args,
) -> Result<()> {
    let info = pr.get_info(()).await?.into_inner();
    info!("info: {info:#?}");
    if !info.can_load_chain_state {
        return Ok(());
    }
    let Some(pubkey) = &info.public_key() else {
        return Err(anyhow!("No public key found for worker"));
    };
    let Ok(pubkey) = hex::decode(pubkey) else {
        return Err(anyhow!("Ceseal returned an invalid pubkey"));
    };
    let (block_number, state) = chain_client::search_suitable_genesis_for_worker(
        chain_api,
        &pubkey,
        args.prefer_genesis_at_block,
    )
    .await
    .context("Failed to search suitable genesis state for worker")?;
    pr.load_chain_state(crpc::ChainState::new(block_number, state))
        .await?;
    Ok(())
}

pub async fn try_update_worker_endpoint(
    cc: &mut CesealClient,
    para_api: &ChainApi,
    signer: &mut SrSigner,
    args: &Args,
) -> Result<bool> {
    let mut info = cc.get_endpoint_info(()).await?.into_inner();
    let encoded_endpoint_payload = match info.encoded_endpoint_payload {
        None => {
            // set endpoint if public_endpoint arg configed
            if let Some(endpoint) = args.public_endpoint.clone() {
                match cc
                    .set_endpoint(Request::new(crpc::SetEndpointRequest::new(endpoint)))
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
            let former: WorkerEndpointPayload =
                Decode::decode(&mut &payload[..]).context("decode payload error")?;
            match args.public_endpoint.clone() {
                Some(endpoint) => {
                    if former.endpoint != Some(endpoint.clone()) || former.endpoint.is_none() {
                        match cc
                            .set_endpoint(Request::new(crpc::SetEndpointRequest::new(endpoint)))
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
        }
    };

    //TODO: Only update on chain if the endpoint changed
    let signature = info
        .signature
        .ok_or_else(|| anyhow!("No endpoint signature"))?;
    info!("Binding worker's endpoint...");
    tx::update_worker_endpoint(para_api, encoded_endpoint_payload, signature, signer, args).await
}

async fn bridge(
    args: &Args,
    flags: &mut RunningFlags,
    err_report: Sender<MsgSyncError>,
) -> Result<()> {
    // Connect to substrate
    let chain_api: ChainApi = subxt_connect(&args.chain_ws_endpoint).await?;
    info!("Connected to chain at: {}", args.chain_ws_endpoint);

    if !args.no_wait {
        // Don't start our worker until the substrate node is synced
        info!("Waiting for chain node to sync blocks...");
        chain_client::wait_until_synced(&chain_api).await?;
        info!("Substrate sync blocks done");
    }

    // Other initialization
    let mut cc = CesealClient::connect(args.internal_endpoint.clone()).await?;
    let pair = <sr25519::Pair as Pair>::from_string(&args.mnemonic, None)
        .expect("Bad privkey derive path");
    let mut signer = SrSigner::new(pair);

    // Try to initialize Ceseal and register on-chain
    let info = cc.get_info(()).await?.into_inner();
    let operator = match args.operator.clone() {
        None => None,
        Some(operator) => {
            let parsed_operator = AccountId32::from_str(&operator)
                .map_err(|e| anyhow!("Failed to parse operator address: {}", e))?;
            Some(parsed_operator)
        }
    };
    if !args.no_init {
        if !info.initialized {
            info!("Ceseal not initialized. Requesting init...");
            let start_header = args.start_header.unwrap_or(0);
            info!("Resolved start header at {}", start_header);
            let runtime_info = init_runtime(
                &chain_api,
                &mut cc,
                args.attestation_provider.into(),
                args.use_dev_key,
                &args.inject_key,
                operator.clone(),
                start_header,
            )
            .await?;
            info!("runtime_info: {:?}", runtime_info);
        } else {
            info!("Ceseal already initialized.");
        }

        if args.fast_sync {
            try_load_chain_state(&mut cc, &chain_api, args).await?;
        }
    }

    if args.no_sync {
        let worker_pubkey = hex::decode(
            info.public_key()
                .ok_or(anyhow!("worker public key must not none"))?,
        )?;
        if !chain_api
            .is_worker_registered_at(&worker_pubkey, None)
            .await?
            && !flags.worker_register_sent
        {
            flags.worker_register_sent =
                try_register_worker(&mut cc, &chain_api, &mut signer, operator.clone(), args)
                    .await?;
        }
        // Try bind worker endpoint
        if args.public_endpoint.is_some() && info.public_key().is_some() {
            // Here the reason we dont directly report errors when `try_update_worker_endpoint` fails is that we want the endpoint can be registered anytime (e.g. days after the cifrost initialization)
            match try_update_worker_endpoint(&mut cc, &chain_api, &mut signer, args).await {
                Ok(registered) => {
                    flags.endpoint_registered = registered;
                }
                Err(e) => {
                    error!("FailedToCallBindWorkerEndpoint: {:?}", e);
                }
            }
        }
        warn!("Block sync disabled.");
        return Ok(());
    }

    block_subscribe::spawn_subscriber(&chain_api, cc.clone()).await?;

    loop {
        // update the latest Ceseal state
        let info = cc.get_info(()).await?.into_inner();
        info!("Ceseal get_info response: {:#?}", info);
        if info.blocknum >= args.to_block {
            info!("Reached target block: {}", args.to_block);
            return Ok(());
        }

        match get_sync_operation(&chain_api, &info).await? {
            SyncOperation::ChainHeader => {
                sync_headers(&mut cc, &chain_api, info.headernum).await?;
            }
            SyncOperation::Block => {
                let next_headernum = info.headernum;
                batch_sync_storage_changes(
                    &mut cc,
                    &chain_api,
                    info.blocknum,
                    next_headernum - 1,
                    args.sync_blocks,
                )
                .await?;
            }
            SyncOperation::ReachedChainTip => {
                if args.load_handover_proof {
                    try_load_handover_proof(&mut cc, &chain_api)
                        .await
                        .context("Failed to load handover proof")?;
                }
                let worker_pubkey = hex::decode(
                    info.public_key()
                        .ok_or(anyhow!("worker public key must not none"))?,
                )?;
                if !chain_api
                    .is_worker_registered_at(&worker_pubkey, None)
                    .await?
                    && !flags.worker_register_sent
                {
                    flags.worker_register_sent = try_register_worker(
                        &mut cc,
                        &chain_api,
                        &mut signer,
                        operator.clone(),
                        args,
                    )
                    .await?;
                }

                if args.public_endpoint.is_some()
                    && !flags.endpoint_registered
                    && info.public_key().is_some()
                {
                    // Here the reason we dont directly report errors when `try_update_worker_endpoint` fails is that we want the endpoint can be registered anytime (e.g. days after the cifrost initialization)
                    match try_update_worker_endpoint(&mut cc, &chain_api, &mut signer, args).await {
                        Ok(registered) => {
                            flags.endpoint_registered = registered;
                        }
                        Err(e) => {
                            error!("FailedToCallBindWorkerEndpoint: {:?}", e);
                        }
                    }
                }

                if chain_api.is_master_key_launched().await?
                    && !info.is_master_key_holded()
                    && !flags.master_key_apply_sent
                {
                    let sent =
                        tx::try_apply_master_key(&mut cc, &chain_api, &mut signer, args).await?;
                    flags.master_key_apply_sent = sent;
                }

                if info.is_master_key_holded() && !info.is_external_server_running() {
                    start_external_server(&mut cc).await?;
                }

                // Now we are idle. Let's try to sync the egress messages.
                if !args.no_msg_submit {
                    msg_sync::maybe_sync_mq_egress(
                        &chain_api,
                        &mut cc,
                        &mut signer,
                        args.tip,
                        args.longevity,
                        args.max_sync_msgs_per_round,
                        err_report.clone(),
                    )
                    .await?;
                }
                flags.restart_failure_count = 0;
                info!("Waiting for new blocks");

                // Launch key handover if required only when the old Ceseal is up-to-date
                if args.next_ceseal_endpoint.is_some() {
                    let mut next_pr =
                        CesealClient::connect(args.next_ceseal_endpoint.clone().unwrap()).await?;
                    handover_worker_key(&mut cc, &mut next_pr).await?;
                }

                if !flags.checkpoint_taked && args.take_checkpoint {
                    cc.take_checkpoint(()).await?;
                    flags.checkpoint_taked = true;
                }

                sleep(Duration::from_millis(args.dev_wait_block_ms)).await;
                continue;
            }
        };
    }
}

async fn get_sync_operation(
    chain_api: &ChainApi,
    info: &crpc::CesealInfo,
) -> Result<SyncOperation> {
    let next_headernum = info.headernum;
    if info.blocknum < next_headernum {
        return Ok(SyncOperation::Block);
    }

    let latest_header = chain_client::get_header_at(chain_api, None).await?.0;
    info!(
        "get_sync_operation: ceseal next header: {}, latest header: {}",
        info.headernum, latest_header.number,
    );
    if latest_header.number > 0 && info.headernum <= latest_header.number {
        Ok(SyncOperation::ChainHeader)
    } else {
        Ok(SyncOperation::ReachedChainTip)
    }
}

fn preprocess_args(args: &mut Args) {
    if args.dev {
        args.use_dev_key = true;
        args.mnemonic = String::from("//Alice");
        args.attestation_provider = RaOption::None;
    }
    if args.longevity > 0 {
        assert!(args.longevity >= 4, "Option --longevity must be 0 or >= 4.");
        assert_eq!(
            args.longevity.count_ones(),
            1,
            "Option --longevity must be power of two."
        );
    }
}

async fn collect_async_errors(
    mut threshold: Option<u64>,
    mut err_receiver: Receiver<MsgSyncError>,
) {
    let threshold_bak = threshold.unwrap_or_default();
    loop {
        match err_receiver.recv().await {
            Some(error) => match error {
                MsgSyncError::BadSignature => {
                    warn!("tx received bad signature, restarting...");
                    return;
                }
                MsgSyncError::OtherRpcError => {
                    if let Some(threshold) = &mut threshold {
                        if *threshold == 0 {
                            warn!("{} tx errors reported, restarting...", threshold_bak);
                            return;
                        }
                        *threshold -= 1;
                    }
                }
            },
            None => {
                warn!("All senders gone, this should never happen!");
                return;
            }
        }
    }
}

pub async fn run() {
    let mut args = Args::parse();
    preprocess_args(&mut args);

    let mut flags = RunningFlags {
        worker_register_sent: false,
        endpoint_registered: false,
        master_key_apply_sent: false,
        restart_failure_count: 0,
        checkpoint_taked: false,
    };

    loop {
        let (sender, receiver) = msg_sync::create_report_channel();
        let threshold = args.restart_on_rpc_error_threshold;
        tokio::select! {
            res = bridge(&args, &mut flags, sender) => {
                if let Err(err) = res {
                    info!("bridge() exited with error: {:?}", err);
                } else {
                    break;
                }
            }
            () = collect_async_errors(threshold, receiver) => ()
        };
        if !args.auto_restart || flags.restart_failure_count > args.max_restart_retries {
            std::process::exit(1);
        }
        flags.restart_failure_count += 1;
        sleep(Duration::from_secs(2)).await;
        info!("Restarting...");
    }
}

/// This function panics intentionally after the worker key handover finishes
async fn handover_worker_key(server: &mut CesealClient, client: &mut CesealClient) -> Result<()> {
    let challenge = server.handover_create_challenge(()).await?.into_inner();
    let response = client
        .handover_accept_challenge(challenge)
        .await?
        .into_inner();
    let encrypted_key = server.handover_start(response).await?.into_inner();
    client.handover_receive(encrypted_key).await?;
    panic!("Worker key handover done, the new Ceseal is ready to go");
}

async fn start_external_server(cc: &mut CesealClient) -> Result<()> {
    use cestory_api::crpc::{ExternalServerCmd, ExternalServerOperation};
    cc.operate_external_server(Request::new(ExternalServerOperation {
        cmd: ExternalServerCmd::Start.into(),
    }))
    .await?;
    Ok(())
}
