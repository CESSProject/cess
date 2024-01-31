use scale_encode::EncodeAsType;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use subxt::config::polkadot::{PolkadotExtrinsicParams, PolkadotExtrinsicParamsBuilder};

mod chain_api;
pub mod dynamic;
pub mod rpc;

pub use subxt::ext::sp_core;

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, PartialOrd, Ord, Debug, EncodeAsType)]
pub struct ParaId(pub u32);

pub type StorageProof = Vec<Vec<u8>>;
pub type StorageState = Vec<(Vec<u8>, Vec<u8>)>;
pub type ExtrinsicParams = PolkadotExtrinsicParams<Config>;
pub type ExtrinsicParamsBuilder = PolkadotExtrinsicParamsBuilder<Config>;
pub use subxt::PolkadotConfig as Config;
pub type SubxtOnlineClient = subxt::OnlineClient<Config>;

pub use chain_api::{connect, ChainApi};
pub type ParachainApi = ChainApi;
pub type RelaychainApi = ChainApi;

pub use subxt;
pub type BlockNumber = u32;
pub type Hash = primitive_types::H256;
pub type AccountId = <Config as subxt::Config>::AccountId;

/// A wrapper for subxt::tx::PairSigner to make it compatible with older API.
pub struct PairSigner {
    pub signer: subxt::tx::PairSigner<Config, sp_core::sr25519::Pair>,
    nonce: u64,
}
impl PairSigner {
    pub fn new(pair: sp_core::sr25519::Pair) -> Self {
        Self {
            signer: subxt::tx::PairSigner::new(pair),
            nonce: 0,
        }
    }
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
    pub fn nonce(&self) -> u64 {
        self.nonce
    }
    pub fn set_nonce(&mut self, nonce: u64) {
        self.nonce = nonce;
    }
    pub fn account_id(&self) -> &AccountId {
        self.signer.account_id()
    }
}
