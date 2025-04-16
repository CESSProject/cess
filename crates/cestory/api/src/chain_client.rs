use subxt::{
    backend::StreamOfResults,
    blocks::Block,
    config::{
        substrate::{BlakeTwo256, SubstrateHeader, H256},
        SubstrateExtrinsicParams,
    },
    utils::{AccountId32, MultiAddress, MultiSignature},
    Config,
};

include!(concat!(env!("OUT_DIR"), "/genesis_hash.rs"));

// Generated `runtime` mod
include!(concat!(env!("OUT_DIR"), "/runtime_path.rs"));

pub enum CesRuntimeConfig {}

impl Config for CesRuntimeConfig {
    type Hash = H256;
    type AccountId = AccountId32;
    type Address = MultiAddress<Self::AccountId, u32>;
    type Signature = MultiSignature;
    type Hasher = BlakeTwo256;
    type Header = SubstrateHeader<u32, BlakeTwo256>;
    type ExtrinsicParams = SubstrateExtrinsicParams<Self>;
    type AssetId = u32;
}

pub type CesChainClient = subxt::client::OnlineClient<CesRuntimeConfig>;
pub type BlockNumber = u32;
pub type AccountId = <CesRuntimeConfig as subxt::Config>::AccountId;
pub type Hash = <CesRuntimeConfig as subxt::Config>::Hash;
pub type CesBlock = Block<CesRuntimeConfig, CesChainClient>;
pub type CesBlockStream = StreamOfResults<CesBlock>;
