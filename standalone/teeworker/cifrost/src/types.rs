use cestory_api::{
    blocks::{BlockHeader, StorageProof},
    ceseal_client,
};
use cesxt::Config;
use clap::Parser;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::{generic::SignedBlock as SpSignedBlock, Justifications, OpaqueExtrinsic};
use subxt::backend::legacy::rpc_methods::{BlockDetails, BlockJustification};
use tonic::transport::Channel;

use ces_types::AttestationProvider;
pub use cesxt::rpc::{self, StorageData, StorageKey};
pub use cesxt::{self, subxt};
use sp_consensus_grandpa::SetId;
pub use subxt::backend::legacy::rpc_methods::NumberOrHex;

pub type CesealClient = ceseal_client::CesealClient<Channel>;
pub type SrSigner = cesxt::PairSigner;

pub type SignedBlock<Hdr, Ext> = SpSignedBlock<sp_runtime::generic::Block<Hdr, Ext>>;

pub type BlockNumber = u32;
pub type Hash = sp_core::H256;
pub type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub type Block = SignedBlock<Header, OpaqueExtrinsic>;
pub type UnsigedBlock = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

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
    pub dev: bool,

    #[arg(short = 'n', long = "no-init", help = "Should init Ceseal?")]
    pub no_init: bool,

    #[arg(
        long = "no-sync",
        help = "Don't sync Ceseal. Quit right after initialization."
    )]
    pub no_sync: bool,

    #[arg(long, help = "Don't write Ceseal egress data back to Substarte.")]
    pub no_msg_submit: bool,

    #[arg(
        long,
        help = "Inject dev key (0x1) to Ceseal. Cannot be used with remote attestation enabled."
    )]
    pub use_dev_key: bool,

    #[arg(
        default_value = "",
        long = "inject-key",
        help = "Inject key to Ceseal."
    )]
    pub inject_key: String,

    #[arg(
        default_value = "ws://localhost:9944",
        long,
        help = "Substrate chain rpc websocket endpoint"
    )]
    pub chain_ws_endpoint: String,

    #[arg(
        default_value = "http://localhost:8000",
        long,
        help = "The http endpoint to access Ceseal's internal service"
    )]
    pub internal_endpoint: String,

    #[arg(
        long,
        help = "The http endpoint where Ceseal provides services to the outside world"
    )]
    pub public_endpoint: Option<String>,

    #[arg(
        long,
        help = "Ceseal http endpoint to handover the key. The handover will only happen when the old Ceseal is synced."
    )]
    pub next_ceseal_endpoint: Option<String>,

    #[arg(
        default_value = "//Alice",
        short = 'm',
        long = "mnemonic",
        help = "Controller SR25519 private key mnemonic, private key seed, or derive path"
    )]
    pub mnemonic: String,

    #[arg(
        default_value = "1000",
        long = "fetch-blocks",
        help = "The batch size to fetch blocks from Substrate."
    )]
    pub fetch_blocks: u32,

    #[arg(
        default_value = "4",
        long = "sync-blocks",
        help = "The batch size to sync blocks to Ceseal."
    )]
    pub sync_blocks: BlockNumber,

    #[arg(
        long = "operator",
        help = "The operator account to set the miner for the worker."
    )]
    pub operator: Option<String>,

    #[arg(
        long,
        help = "The first parent header to be synced, default to auto-determine"
    )]
    pub start_header: Option<BlockNumber>,

    #[arg(long, help = "Don't wait the substrate nodes to sync blocks")]
    pub no_wait: bool,

    #[arg(
        default_value = "5000",
        long,
        help = "(Debug only) Set the wait block duration in ms"
    )]
    pub dev_wait_block_ms: u64,

    #[arg(
        default_value = "0",
        long,
        help = "The charge transaction payment, unit: balance"
    )]
    pub tip: u128,

    #[arg(
        default_value = "4",
        long,
        help = "The transaction longevity, should be a power of two between 4 and 65536. unit: block"
    )]
    pub longevity: u64,

    #[arg(
        default_value = "200",
        long,
        help = "Max number of messages to be submitted per-round"
    )]
    pub max_sync_msgs_per_round: u64,

    #[arg(long, help = "Auto restart self after an error occurred")]
    pub auto_restart: bool,

    #[arg(
        default_value = "10",
        long,
        help = "Max auto restart retries if it continiously failing. Only used with --auto-restart"
    )]
    pub max_restart_retries: u32,

    #[arg(long, help = "Restart if number of rpc errors reaches the threshold")]
    pub restart_on_rpc_error_threshold: Option<u64>,

    #[arg(long, help = "Stop when synced to given block")]
    #[arg(default_value_t = BlockNumber::MAX)]
    pub to_block: BlockNumber,

    /// Attestation provider
    #[arg(long, value_enum, default_value_t = RaOption::Ias)]
    pub attestation_provider: RaOption,

    /// Try to load chain state from the latest block that the worker haven't registered at.
    #[arg(long)]
    pub fast_sync: bool,

    /// The prefered block to load the genesis state from.
    #[arg(long)]
    pub prefer_genesis_at_block: Option<BlockNumber>,

    /// Load handover proof after blocks synced.
    #[arg(long)]
    pub load_handover_proof: bool,

    /// Take checkpoint from Ceseal after completing the first block synchronization in Cifrost
    #[arg(long)]
    pub take_checkpoint: bool,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum RaOption {
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

pub struct RunningFlags {
    pub worker_register_sent: bool,
    pub endpoint_registered: bool,
    pub master_key_apply_sent: bool,
    pub restart_failure_count: u32,
    pub checkpoint_taked: bool,
}

pub struct BlockSyncState {
    pub blocks: Vec<Block>,
    /// Tracks the latest known authority set id at a certain block.
    pub authory_set_state: Option<(BlockNumber, SetId)>,
}

pub mod utils {
    use super::StorageProof;
    use cesxt::subxt::backend::legacy::rpc_methods::ReadProof;
    pub fn raw_proof<T>(read_proof: ReadProof<T>) -> StorageProof {
        read_proof.proof.into_iter().map(|p| p.0).collect()
    }
}

pub trait ConvertTo<T> {
    fn convert_to(&self) -> T;
}

fn recode<F: Encode, T: Decode>(f: &F) -> Result<T, parity_scale_codec::Error> {
    Decode::decode(&mut &f.encode()[..])
}

impl<H> ConvertTo<BlockHeader> for H
where
    H: subxt::config::Header,
{
    fn convert_to(&self) -> BlockHeader {
        recode(self).expect("Failed to convert subxt header to block header")
    }
}

impl ConvertTo<Block> for BlockDetails<Config> {
    fn convert_to(&self) -> Block {
        Block {
            block: sp_runtime::generic::Block {
                header: self.block.header.convert_to(),
                extrinsics: vec![],
            },
            justifications: self.justifications.as_ref().map(|x| x.convert_to()),
        }
    }
}

impl ConvertTo<Justifications> for Vec<BlockJustification> {
    fn convert_to(&self) -> Justifications {
        recode(self).expect("Failed to convert BlockDetails to Block")
    }
}