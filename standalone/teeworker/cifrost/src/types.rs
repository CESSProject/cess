use cestory_api::{
    blocks::{BlockHeader, StorageProof},
    ceseal_client,
};
use serde::{Deserialize, Serialize};
use sp_runtime::{generic::SignedBlock as SpSignedBlock, Justifications, OpaqueExtrinsic};

use cesxt::Config;

pub use cesxt::rpc::{self, StorageData, StorageKey};
pub use cesxt::{self, subxt, ParachainApi, RelaychainApi};

use subxt::backend::legacy::rpc_methods::{BlockDetails, BlockJustification};

pub use subxt::backend::legacy::rpc_methods::NumberOrHex;

use parity_scale_codec::{Decode, Encode};

pub type CesealClient = ceseal_client::CesealClient;
pub type SrSigner = cesxt::PairSigner;

pub type SignedBlock<Hdr, Ext> = SpSignedBlock<sp_runtime::generic::Block<Hdr, Ext>>;

pub type BlockNumber = u32;
pub type Hash = sp_core::H256;
pub type Header = sp_runtime::generic::Header<BlockNumber, sp_runtime::traits::BlakeTwo256>;
pub type Block = SignedBlock<Header, OpaqueExtrinsic>;
// API: notify

#[derive(Serialize, Deserialize, Debug)]
pub struct NotifyReq {
    pub headernum: BlockNumber,
    pub blocknum: BlockNumber,
    pub ceseal_initialized: bool,
    pub ceseal_new_init: bool,
    pub initial_sync_finished: bool,
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
