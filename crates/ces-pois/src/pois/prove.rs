use serde::{Deserialize, Serialize};

use crate::{acc::multi_level_acc::WitnessNode, expanders::NodeType};

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Commits {
    pub file_indexs: Vec<i64>,
    pub roots: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MhtProof {
    pub index: NodeType,
    pub label: Vec<u8>,
    pub paths: Vec<Vec<u8>>,
    pub locs: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CommitProof {
    pub node: MhtProof,
    pub parents: Vec<MhtProof>,
    pub elders: Vec<MhtProof>,
}

#[derive(Debug, Default)]
pub struct AccProof {
    pub indexs: Vec<i64>,
    pub labels: Vec<Vec<u8>>,
    pub wit_chains: Option<Box<WitnessNode>>,
    pub acc_path: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SpaceProof {
    pub left: i64,
    pub right: i64,
    pub proofs: Vec<Vec<MhtProof>>,
    pub roots: Vec<Vec<u8>>,
    pub wit_chains: Vec<WitnessNode>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DeletionProof {
    pub roots: Vec<Vec<u8>>,
    pub wit_chain: WitnessNode,
    pub acc_path: Vec<Vec<u8>>,
}