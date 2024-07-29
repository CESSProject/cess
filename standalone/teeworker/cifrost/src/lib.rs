use crate::{
    error::Error,
    types::{
        Args, Block, BlockNumber, BlockSyncState, CesealClient, Header, RaOption, RunningFlags,
        SrSigner,
    },
};
use anyhow::{anyhow, Context, Result};
use ces_types::{AttestationProvider, WorkerEndpointPayload};
use cestory_api::{
    blocks::{AuthoritySetChange, GenesisBlockInfo, HeaderToSync},
    crpc::{self, InitRuntimeResponse},
};
use cesxt::{
    connect as subxt_connect,
    dynamic::storage_key,
    sp_core::{crypto::Pair, sr25519},
    ChainApi,
};
use clap::Parser;
use log::{debug, error, info, warn};
use msg_sync::{Error as MsgSyncError, Receiver, Sender};
use parity_scale_codec::Decode;
use sp_consensus_grandpa::SetId;
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
        let r = cc
            .dispatch_blocks(crpc::Blocks::new(storage_changes))
            .await?
            .into_inner();
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
    let genesis_info = {
        let genesis_block = chain_client::get_block_at(chain_api, Some(start_header))
            .await?
            .0
            .block;
        let set_proof = get_authority_with_proof_at(chain_api, &genesis_block.header).await?;
        GenesisBlockInfo {
            block_header: genesis_block.header.clone(),
            authority_set: set_proof.authority_set,
            proof: set_proof.authority_proof,
        }
    };
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
        info!("No attestation evidence from ceseal runtime_info!");
        Ok(false)
    }
}

async fn try_update_worker_ra_report(
    ceseal_client: &mut CesealClient,
    chain_client: &ChainApi,
    signer: &mut SrSigner,
    operator: Option<AccountId32>,
    longevity: u64,
    tip: u128,
) -> Result<bool> {
    let info = ceseal_client
        .get_runtime_info(crpc::GetRuntimeInfoRequest::new(false, operator))
        .await?
        .into_inner();
    if let Some(attestation) = info.attestation {
        tx::update_worker_ra_report(
            chain_client,
            info.encoded_runtime_info,
            attestation,
            signer,
            longevity,
            tip,
        )
        .await?;
        Ok(true)
    } else {
        info!("No attestation evidence from ceseal runtime_info!");
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
    let mut signer = SrSigner::new(pair.clone());

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

    info!("Start updating ceseal's ra report task...");
    tokio::spawn(schedule_updates_ra_report(
        cc.clone(),
        chain_api.clone(),
        pair.clone(),
        operator.clone(),
        args.longevity,
        args.tip,
    ));

    // Don't just sync message if we want to wait for some block
    let mut sync_state = BlockSyncState {
        blocks: Vec::new(),
        authory_set_state: None,
    };

    loop {
        // update the latest Ceseal state
        let info = cc.get_info(()).await?.into_inner();
        info!("Ceseal get_info response: {:#?}", info);
        if info.blocknum >= args.to_block {
            info!("Reached target block: {}", args.to_block);
            return Ok(());
        }

        let latest_block = chain_client::get_block_at(&chain_api, None).await?.0.block;
        // remove the blocks not needed in the buffer. info.blocknum is the next required block
        while let Some(b) = sync_state.blocks.first() {
            if b.block.header.number >= info.headernum {
                break;
            }
            sync_state.blocks.remove(0);
        }

        info!(
            "try to sync blocks. next required: (body={}, header={}), finalized tip: {}, buffered: {}",
            info.blocknum, info.headernum, latest_block.header.number, sync_state.blocks.len());

        // fill the sync buffer to catch up the chain tip
        let next_block = match sync_state.blocks.last() {
            Some(b) => b.block.header.number + 1,
            None => info.headernum,
        };

        let (batch_end, more_blocks) = {
            let latest = latest_block.header.number;
            let fetch_limit = next_block + args.fetch_blocks - 1;
            if fetch_limit < latest {
                (fetch_limit, true)
            } else {
                (latest, false)
            }
        };

        for b in next_block..=batch_end {
            let (block, _) = chain_client::get_block_at(&chain_api, Some(b)).await?;

            if block.justifications.is_some() {
                debug!("block with justification at: {}", block.block.header.number);
            }
            sync_state.blocks.push(block);
        }

        // send the blocks to Ceseal in batch
        let synced_blocks = batch_sync_block(
            &chain_api,
            &mut cc,
            &mut sync_state,
            args.sync_blocks,
            &info,
        )
        .await?;

        // check if Ceseal has already reached the chain tip.
        if synced_blocks == 0 && !more_blocks {
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
                flags.worker_register_sent =
                    try_register_worker(&mut cc, &chain_api, &mut signer, operator.clone(), args)
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
                let sent = tx::try_apply_master_key(&mut cc, &chain_api, &mut signer, args).await?;
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

const GRANDPA_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"FRNK";

async fn batch_sync_block(
    chain_api: &ChainApi,
    ceseal_api: &mut CesealClient,
    sync_state: &mut BlockSyncState,
    batch_window: BlockNumber,
    info: &crpc::CesealInfo,
) -> Result<BlockNumber> {
    let block_buf = &mut sync_state.blocks;
    if block_buf.is_empty() {
        return Ok(0);
    }
    let mut next_headernum = info.headernum;
    let mut next_blocknum = info.blocknum;

    let mut synced_blocks: BlockNumber = 0;

    let hdr_synced_to = next_headernum.saturating_sub(1);
    macro_rules! sync_blocks_to {
        ($to: expr) => {
            if next_blocknum <= $to {
                batch_sync_storage_changes(ceseal_api, chain_api, next_blocknum, $to, batch_window)
                    .await?;
                synced_blocks += $to - next_blocknum + 1;
                next_blocknum = $to + 1;
            };
        };
    }

    sync_blocks_to!(hdr_synced_to);

    while !block_buf.is_empty() {
        // Current authority set id
        let last_set = if let Some(set) = sync_state.authory_set_state {
            set
        } else {
            // Construct the authority set from the last block we have synced (the genesis)
            let number = block_buf
                .first()
                .unwrap()
                .block
                .header
                .number
                .saturating_sub(1);
            let hash = chain_api
                .rpc()
                .chain_get_block_hash(Some(number.into()))
                .await?
                .ok_or_else(|| anyhow!("Failed to get block hash for block number {}", number))?;
            let set_id = chain_api.current_set_id(Some(hash)).await?;
            let set = (number, set_id);
            sync_state.authory_set_state = Some(set);
            set
        };
        // Find the next set id change
        let set_id_change_at = bisec_setid_change(chain_api, last_set, block_buf).await?;
        let last_number_in_buff = block_buf.last().unwrap().block.header.number;
        // Search
        // Find the longest batch within the window
        let first_block_number = block_buf.first().unwrap().block.header.number;
        // TODO: fix the potential overflow here
        let end_buffer = block_buf.len() as isize - 1;
        let end_set_id_change = match set_id_change_at {
            Some(change_at) => change_at as isize - first_block_number as isize,
            None => block_buf.len() as isize,
        };
        let header_end = std::cmp::min(end_buffer, end_set_id_change);
        let mut header_idx = header_end;
        while header_idx >= 0 {
            if block_buf[header_idx as usize]
                .justifications
                .as_ref()
                .and_then(|v| v.get(GRANDPA_ENGINE_ID))
                .is_some()
            {
                break;
            }
            header_idx -= 1;
        }
        if header_idx < 0 {
            warn!(
                "Cannot find justification within window (from: {}, to: {})",
                first_block_number,
                block_buf.last().unwrap().block.header.number,
            );
            break;
        }
        // send out the longest batch and remove it from the input buffer
        let block_batch: Vec<Block> = block_buf.drain(..=(header_idx as usize)).collect();
        let header_batch: Vec<HeaderToSync> = block_batch
            .iter()
            .map(|b| HeaderToSync {
                header: b.block.header.clone(),
                justification: b
                    .justifications
                    .clone()
                    .and_then(|v| v.into_justification(GRANDPA_ENGINE_ID)),
            })
            .collect();

        /* print collected headers */
        {
            for h in header_batch.iter() {
                debug!(
                    "Header {} :: {} :: {}",
                    h.header.number,
                    h.header.hash().to_string(),
                    h.header.parent_hash.to_string()
                );
            }
        }

        let last_header = &header_batch.last().unwrap();
        let last_header_number = last_header.header.number;

        let mut authrotiy_change: Option<AuthoritySetChange> = None;
        if let Some(change_at) = set_id_change_at {
            if change_at == last_header_number {
                authrotiy_change =
                    Some(get_authority_with_proof_at(chain_api, &last_header.header).await?);
            }
        }

        info!(
            "sending a batch of {} headers (last: {}, change: {:?})",
            header_batch.len(),
            last_header_number,
            authrotiy_change
                .as_ref()
                .map(|change| &change.authority_set)
        );

        let mut header_batch = header_batch;
        header_batch.retain(|h| h.header.number >= next_headernum);
        // Remove justifications for all headers except the last one to reduce data overhead
        // and improve sync performance.
        if let Some((_last, rest_headers)) = header_batch.split_last_mut() {
            for header in rest_headers {
                header.justification = None;
            }
        }

        let r = ceseal_api
            .sync_header(crpc::HeadersToSync::new(header_batch, authrotiy_change))
            .await?
            .into_inner();
        info!("  ..sync_header: {:?}", r);
        next_headernum = r.synced_to + 1;

        sync_blocks_to!(r.synced_to);

        sync_state.authory_set_state = Some(match set_id_change_at {
            // set_id changed at next block
            Some(change_at) => (change_at + 1, last_set.1 + 1),
            // not changed
            None => (last_number_in_buff, last_set.1),
        });
    }
    Ok(synced_blocks)
}

/// Returns the next set_id change by a binary search on the known blocks
///
/// `known_blocks` must have at least one block with block justification, otherwise raise an error
/// `NoJustificationInRange`. If there's no set_id change in the given blocks, it returns None.
async fn bisec_setid_change(
    chain_api: &ChainApi,
    last_set: (BlockNumber, SetId),
    known_blocks: &Vec<Block>,
) -> Result<Option<BlockNumber>> {
    debug!("bisec_setid_change(last_set: {:?})", last_set);
    if known_blocks.is_empty() {
        return Err(anyhow!(Error::SearchSetIdChangeInEmptyRange));
    }
    let (last_block, last_id) = last_set;
    // Run binary search only on blocks with justification
    let headers: Vec<&Header> = known_blocks
        .iter()
        .filter(|b| b.block.header.number > last_block && b.justifications.is_some())
        .map(|b| &b.block.header)
        .collect();
    let mut l = 0i64;
    let mut r = (headers.len() as i64) - 1;
    while l <= r {
        let mid = (l + r) / 2;
        let hash = headers[mid as usize].hash();
        let set_id = chain_api.current_set_id(Some(hash)).await?;
        // Left: set_id == last_id, Right: set_id > last_id
        if set_id == last_id {
            l = mid + 1;
        } else {
            r = mid - 1;
        }
    }
    // Return the first occurance of bigger set_id; return (last_id + 1) if not found
    let result = if (l as usize) < headers.len() {
        Some(headers[l as usize].number)
    } else {
        None
    };
    debug!("bisec_setid_change result: {:?}", result);
    Ok(result)
}

async fn schedule_updates_ra_report(
    cc: CesealClient,
    chain_api: ChainApi,
    pair: sr25519::Pair,
    operator: Option<AccountId32>,
    longevity: u64,
    tip: u128,
) -> Result<()> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(86400));
    let mut signer = SrSigner::new(pair.clone());
    loop {
        interval.tick().await;
        match try_update_worker_ra_report(
            &mut cc.clone(),
            &chain_api,
            &mut signer,
            operator.clone(),
            longevity,
            tip,
        )
        .await{
            Ok(result) =>{
                info!("Scheduled update ceseal ra report successfully!")
            },
            Err(error) => {
                info!("can't update ceseal ra report because :{:?}",error.to_string())
            },
        };
    }
}
