use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, warn};
use sp_core::crypto::AccountId32;
use std::cmp;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;

use cesxt::{
    connect as subxt_connect,
    dynamic::storage_key,
    sp_core::{crypto::Pair, sr25519},
    ChainApi,
};
use parity_scale_codec::{Decode, Encode};
use sp_consensus_grandpa::{AuthorityList, SetId, VersionedAuthorityList, GRANDPA_AUTHORITIES_KEY};

pub use cesxt::subxt;
use subxt::{config::polkadot::PolkadotExtrinsicParamsBuilder as Params, tx::TxPayload};

mod endpoint;
mod error;
mod msg_sync;
mod notify_client;
mod prefetcher;
mod tx;

pub mod chain_client;
pub mod types;

use crate::{
    error::Error,
    types::{
        Block, BlockNumber, CesealClient, ConvertTo, Hash, Header, NotifyReq, NumberOrHex, SrSigner,
    },
};
use ces_types::{attestation::legacy::Attestation, AttestationProvider};
use cestory_api::{
    blocks::{
        self, AuthoritySet, AuthoritySetChange, BlockHeader, BlockHeaderWithChanges, HeaderToSync,
    },
    crpc::{self, InitRuntimeResponse},
};
use clap::Parser;
use msg_sync::{Error as MsgSyncError, Receiver, Sender};
use notify_client::NotifyClient;

#[derive(Parser, Debug)]
#[command(
    about = "Sync messages between ceseal and the blockchain.",
    version,
    author
)]
pub struct Args {
    #[arg(
        long,
        help = "Dev mode (equivalent to `--use-dev-key --mnemonic='//Alice'`)"
    )]
    dev: bool,

    #[arg(short = 'n', long = "no-init", help = "Should init Ceseal?")]
    no_init: bool,

    #[arg(
        long = "no-sync",
        help = "Don't sync Ceseal. Quit right after initialization."
    )]
    no_sync: bool,

    #[arg(long, help = "Don't write Ceseal egress data back to Substarte.")]
    no_msg_submit: bool,

    #[arg(
        long,
        help = "Inject dev key (0x1) to Ceseal. Cannot be used with remote attestation enabled."
    )]
    use_dev_key: bool,

    #[arg(
        default_value = "",
        long = "inject-key",
        help = "Inject key to Ceseal."
    )]
    inject_key: String,

    #[arg(
        default_value = "ws://localhost:9944",
        long,
        help = "Substrate chain rpc websocket endpoint"
    )]
    chain_ws_endpoint: String,

    #[arg(
        default_value = "http://localhost:8000",
        long,
        help = "The http endpoint to access Ceseal's internal service"
    )]
    internal_endpoint: String,

    #[arg(
        long,
        help = "The http endpoint where Ceseal provides services to the outside world"
    )]
    public_endpoint: Option<String>,

    #[arg(
        long,
        help = "Ceseal http endpoint to handover the key. The handover will only happen when the old Ceseal is synced."
    )]
    next_ceseal_endpoint: Option<String>,

    #[arg(default_value = "", long, help = "notify endpoint")]
    notify_endpoint: String,

    #[arg(
        default_value = "//Alice",
        short = 'm',
        long = "mnemonic",
        help = "Controller SR25519 private key mnemonic, private key seed, or derive path"
    )]
    mnemonic: String,

    #[arg(
        default_value = "1000",
        long = "fetch-blocks",
        help = "The batch size to fetch blocks from Substrate."
    )]
    fetch_blocks: u32,

    #[arg(
        default_value = "4",
        long = "sync-blocks",
        help = "The batch size to sync blocks to Ceseal."
    )]
    sync_blocks: BlockNumber,

    #[arg(
        long = "operator",
        help = "The operator account to set the miner for the worker."
    )]
    operator: Option<String>,

    #[arg(
        long,
        help = "The first parent header to be synced, default to auto-determine"
    )]
    start_header: Option<BlockNumber>,

    #[arg(long, help = "Don't wait the substrate nodes to sync blocks")]
    no_wait: bool,

    #[arg(
        default_value = "5000",
        long,
        help = "(Debug only) Set the wait block duration in ms"
    )]
    dev_wait_block_ms: u64,

    #[arg(
        default_value = "0",
        long,
        help = "The charge transaction payment, unit: balance"
    )]
    tip: u128,

    #[arg(
        default_value = "4",
        long,
        help = "The transaction longevity, should be a power of two between 4 and 65536. unit: block"
    )]
    longevity: u64,

    #[arg(
        default_value = "200",
        long,
        help = "Max number of messages to be submitted per-round"
    )]
    max_sync_msgs_per_round: u64,

    #[arg(long, help = "Auto restart self after an error occurred")]
    auto_restart: bool,

    #[arg(
        default_value = "10",
        long,
        help = "Max auto restart retries if it continiously failing. Only used with --auto-restart"
    )]
    max_restart_retries: u32,

    #[arg(long, help = "Restart if number of rpc errors reaches the threshold")]
    restart_on_rpc_error_threshold: Option<u64>,

    #[arg(long, help = "Stop when synced to given block")]
    #[arg(default_value_t = BlockNumber::MAX)]
    to_block: BlockNumber,

    /// Attestation provider
    #[arg(long, value_enum, default_value_t = RaOption::Ias)]
    attestation_provider: RaOption,

    /// Try to load chain state from the latest block that the worker haven't registered at.
    #[arg(long)]
    fast_sync: bool,

    /// The prefered block to load the genesis state from.
    #[arg(long)]
    prefer_genesis_at_block: Option<BlockNumber>,

    /// Load handover proof after blocks synced.
    #[arg(long)]
    load_handover_proof: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum RaOption {
    None,
    Ias,
}

impl From<RaOption> for Option<AttestationProvider> {
    fn from(other: RaOption) -> Self {
        match other {
            RaOption::None => None,
            RaOption::Ias => Some(AttestationProvider::Ias),
        }
    }
}

struct RunningFlags {
    worker_register_sent: bool,
    endpoint_registered: bool,
    master_key_apply_sent: bool,
    restart_failure_count: u32,
}

pub struct BlockSyncState {
    pub blocks: Vec<Block>,
    /// Tracks the latest known authority set id at a certain block.
    pub authory_set_state: Option<(BlockNumber, SetId)>,
}

pub async fn get_header_hash(client: &ChainApi, h: Option<u32>) -> Result<Hash> {
    let pos = h.map(|h| NumberOrHex::Number(h.into()));
    let hash = match pos {
        Some(_) => client
            .rpc()
            .chain_get_block_hash(pos)
            .await?
            .ok_or(Error::BlockHashNotFound)?,
        None => client.rpc().chain_get_finalized_head().await?,
    };
    Ok(hash)
}

pub async fn get_block_at(client: &ChainApi, h: Option<u32>) -> Result<(Block, Hash)> {
    let hash = get_header_hash(client, h).await?;
    let block = client
        .rpc()
        .chain_get_block(Some(hash))
        .await?
        .ok_or(Error::BlockNotFound)?;

    Ok((block.convert_to(), hash))
}

pub async fn get_header_at(client: &ChainApi, h: Option<u32>) -> Result<(Header, Hash)> {
    let hash = get_header_hash(client, h).await?;
    let header = client
        .rpc()
        .chain_get_header(Some(hash))
        .await?
        .ok_or(Error::BlockNotFound)?;

    Ok((header.convert_to(), hash))
}

pub async fn get_block_without_storage_changes(
    chain_api: &ChainApi,
    h: Option<u32>,
) -> Result<Block> {
    let (block, hash) = get_block_at(chain_api, h).await?;
    info!("get_block: Got block {:?} hash {}", h, hash.to_string());
    Ok(block)
}

pub async fn fetch_storage_changes(
    client: &ChainApi,
    from: BlockNumber,
    to: BlockNumber,
) -> Result<Vec<BlockHeaderWithChanges>> {
    log::info!("fetch_storage_changes ({from}-{to})");
    if to < from {
        return Ok(vec![]);
    }
    let from_hash = get_header_hash(client, Some(from)).await?;
    let to_hash = get_header_hash(client, Some(to)).await?;
    let storage_changes = chain_client::fetch_storage_changes(client, &from_hash, &to_hash)
        .await?
        .into_iter()
        .enumerate()
        .map(|(offset, storage_changes)| {
            BlockHeaderWithChanges {
                // Headers are synced separately. Only the `number` is used in Ceseal while syncing blocks.
                block_header: BlockHeader {
                    number: from + offset as BlockNumber,
                    parent_hash: Default::default(),
                    state_root: Default::default(),
                    extrinsics_root: Default::default(),
                    digest: Default::default(),
                },
                storage_changes,
            }
        })
        .collect();
    Ok(storage_changes)
}

pub async fn batch_sync_storage_changes(
    pr: &mut CesealClient,
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
        let r = req_dispatch_block(pr, storage_changes).await?;
        log::debug!("  ..dispatch_block: {:?}", r);
    }
    Ok(())
}

pub async fn get_authority_with_proof_at(
    chain_api: &ChainApi,
    hash: Hash,
) -> Result<AuthoritySetChange> {
    // Storage
    let id_key = cesxt::dynamic::storage_key("Grandpa", "CurrentSetId");
    // Authority set
    let value = chain_api
        .rpc()
        .state_get_storage(GRANDPA_AUTHORITIES_KEY, Some(hash))
        .await?
        .expect("No authority key found");
    let list: AuthorityList = VersionedAuthorityList::decode(&mut value.as_slice())
        .expect("Failed to decode VersionedAuthorityList")
        .into();

    // Set id
    let id = chain_api.current_set_id(Some(hash)).await?;
    // Proof
    let proof = chain_client::read_proofs(
        chain_api,
        Some(hash),
        vec![GRANDPA_AUTHORITIES_KEY, &id_key],
    )
    .await?;
    Ok(AuthoritySetChange {
        authority_set: AuthoritySet { list, id },
        authority_proof: proof,
    })
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
    let hash = get_header_hash(chain_api, Some(current_block)).await?;
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

async fn req_sync_header(
    pr: &mut CesealClient,
    headers: Vec<HeaderToSync>,
    authority_set_change: Option<AuthoritySetChange>,
) -> Result<crpc::SyncedTo> {
    let resp = pr
        .sync_header(crpc::HeadersToSync::new(headers, authority_set_change))
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

const GRANDPA_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"FRNK";

#[allow(clippy::too_many_arguments)]
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
        let header_end = cmp::min(end_buffer, end_set_id_change);
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
        let last_header_hash = last_header.header.hash();
        let last_header_number = last_header.header.number;

        let mut authrotiy_change: Option<AuthoritySetChange> = None;
        if let Some(change_at) = set_id_change_at {
            if change_at == last_header_number {
                authrotiy_change =
                    Some(get_authority_with_proof_at(chain_api, last_header_hash).await?);
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

        let r = req_sync_header(ceseal_api, header_batch, authrotiy_change).await?;
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
        let genesis_block = get_block_at(chain_api, Some(start_header)).await?.0.block;
        let hash = chain_api
            .rpc()
            .chain_get_block_hash(Some(NumberOrHex::Number(start_header as _)))
            .await?
            .expect("No genesis block?");
        let set_proof = get_authority_with_proof_at(chain_api, hash).await?;
        blocks::GenesisBlockInfo {
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

async fn register_worker(
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

    let ret = chain_api
        .tx()
        .create_signed_with_nonce(&tx, &signer.signer, signer.nonce(), tx_params)?
        .submit_and_watch()
        .await;
    if ret.is_err() {
        error!("FailedToCallRegisterWorker: {:?}", ret);
        return Err(anyhow!(Error::FailedToCallRegisterWorker));
    }
    match ret.unwrap().wait_for_finalized_success().await {
        Ok(e) => {
            info!("Tee registration successful in block hash:{:?}, and the transaction hash is :{:?}",e.block_hash(),e.extrinsic_hash())
        },
        Err(e) => {
            error!("Tee registration transaction has been finalized, but registration failed :{:?}",e.to_string());
            return Err(anyhow!(Error::FailedToCallRegisterWorker));
        },
    };
    signer.increment_nonce();
    Ok(())
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
        register_worker(
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

const DEV_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

async fn wait_until_synced(client: &ChainApi) -> Result<()> {
    loop {
        let state = client.extra_rpc().system_sync_state().await?;
        info!(
            "Checking synced: current={} highest={:?}",
            state.current_block, state.highest_block
        );
        if let Some(highest) = state.highest_block {
            if highest - state.current_block <= 8 {
                return Ok(());
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
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
        wait_until_synced(&chain_api).await?;
        info!("Substrate sync blocks done");
    }

    // Other initialization
    let mut cc = CesealClient::connect(args.internal_endpoint.clone()).await?;
    let pair = <sr25519::Pair as Pair>::from_string(&args.mnemonic, None)
        .expect("Bad privkey derive path");
    let mut signer = SrSigner::new(pair);
    let nc = NotifyClient::new(&args.notify_endpoint);
    let mut ceseal_initialized = false;
    let mut ceseal_new_init = false;
    let mut initial_sync_finished = false;

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
            // STATUS: ceseal_initialized = true
            // STATUS: ceseal_new_init = true
            ceseal_initialized = true;
            ceseal_new_init = true;
            nc.notify(&NotifyReq {
                headernum: info.headernum,
                blocknum: info.blocknum,
                ceseal_initialized,
                ceseal_new_init,
                initial_sync_finished,
            })
            .await
            .ok();
            info!("runtime_info: {:?}", runtime_info);
        } else {
            info!("Ceseal already initialized.");
            // STATUS: ceseal_initialized = true
            // STATUS: ceseal_new_init = false
            ceseal_initialized = true;
            ceseal_new_init = false;
            nc.notify(&NotifyReq {
                headernum: info.headernum,
                blocknum: info.blocknum,
                ceseal_initialized,
                ceseal_new_init,
                initial_sync_finished,
            })
            .await
            .ok();
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
            match endpoint::try_update_worker_endpoint(&mut cc, &chain_api, &mut signer, args).await
            {
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

    // Don't just sync message if we want to wait for some block
    let mut sync_state = BlockSyncState {
        blocks: Vec::new(),
        authory_set_state: None,
    };

    for round in 0u64.. {
        // update the latest Ceseal state
        let info = cc.get_info(()).await?.into_inner();
        info!("Ceseal get_info response (round: {}): {:#?}", round, info);
        if info.blocknum >= args.to_block {
            info!("Reached target block: {}", args.to_block);
            return Ok(());
        }

        // STATUS: header_synced = info.headernum
        // STATUS: block_synced = info.blocknum
        nc.notify(&NotifyReq {
            headernum: info.headernum,
            blocknum: info.blocknum,
            ceseal_initialized,
            ceseal_new_init,
            initial_sync_finished,
        })
        .await
        .ok();

        let next_headernum = info.headernum;
        if info.blocknum < next_headernum {
            info!("blocks fall behind");
            batch_sync_storage_changes(
                &mut cc,
                &chain_api,
                info.blocknum,
                next_headernum - 1,
                args.sync_blocks,
            )
            .await?;
        }

        let latest_block = get_block_at(&chain_api, None).await?.0.block;
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
            let block = get_block_without_storage_changes(&chain_api, Some(b)).await?;

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
                match endpoint::try_update_worker_endpoint(&mut cc, &chain_api, &mut signer, args)
                    .await
                {
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

            // STATUS: initial_sync_finished = true
            initial_sync_finished = true;
            nc.notify(&NotifyReq {
                headernum: info.headernum,
                blocknum: info.blocknum,
                ceseal_initialized,
                ceseal_new_init,
                initial_sync_finished,
            })
            .await
            .ok();

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

            sleep(Duration::from_millis(args.dev_wait_block_ms)).await;
            continue;
        }
    }
    Ok(())
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
    use tonic::Request;
    cc.operate_external_server(Request::new(ExternalServerOperation {
        cmd: ExternalServerCmd::Start.into(),
    }))
    .await?;
    Ok(())
}
