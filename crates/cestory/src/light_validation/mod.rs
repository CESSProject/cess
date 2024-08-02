// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

// Ensure we're `no_std` when compiling for Wasm.
// #![cfg_attr(not(feature = "std"), no_std)]

mod error;
mod justification;
pub mod storage_proof;
mod types;

use anyhow::{Context, Result};
use ces_serde_more as more;
use error::JustificationError;
#[cfg(not(test))]
use im::OrdMap as BTreeMap;
use justification::GrandpaJustification;
use log::{error, trace};
use serde::{Deserialize, Serialize};
#[cfg(test)]
// For TypeInfo derivation
use std::collections::BTreeMap;
use std::{fmt, marker::PhantomData};
use storage_proof::{StorageProof, StorageProofChecker};

use finality_grandpa::voter_set::VoterSet;
use num_traits::AsPrimitive;
use parity_scale_codec::{Decode, Encode};
use sp_consensus_grandpa::{AuthorityId, AuthorityWeight, SetId};
use sp_core::H256;
use sp_runtime::{
    traits::{Block as BlockT, Header, NumberFor},
    EncodedJustification,
};

pub use types::{find_scheduled_change, AuthoritySet, BlockHeader};

#[derive(Encode, Decode, Clone, PartialEq, Serialize, Deserialize, ::scale_info::TypeInfo)]
pub struct BridgeInfo<T: Config> {
    #[serde(skip)]
    _marker: PhantomData<T>,
    #[serde(bound(
        serialize = "BlockHeader: ::serde::Serialize",
        deserialize = "BlockHeader: ::serde::de::DeserializeOwned"
    ))]
    last_finalized_block_header: BlockHeader,
    #[serde(with = "more::scale_bytes")]
    current_set: AuthoritySet,
}

impl<T: Config> BridgeInfo<T> {
    pub fn new(block_header: BlockHeader, validator_set: AuthoritySet) -> Self {
        BridgeInfo {
            _marker: Default::default(),
            last_finalized_block_header: block_header,
            current_set: validator_set,
        }
    }
}

type BridgeId = u64;

pub trait Config: frame_system::Config<Hash = H256> {
    type Block: BlockT<Hash = H256, Header = BlockHeader>;
}

impl Config for chain::Runtime {
    type Block = chain::Block;
}

#[derive(Encode, Decode, Clone, Serialize, Deserialize, ::scale_info::TypeInfo)]
pub struct LightValidation<T: Config> {
    num_bridges: BridgeId,
    #[serde(bound(
        serialize = "BlockHeader: ::serde::Serialize",
        deserialize = "BlockHeader: ::serde::de::DeserializeOwned"
    ))]
    tracked_bridges: BTreeMap<BridgeId, BridgeInfo<T>>,
    grandpa_note_stalled: bool,
}

impl<T: Config> LightValidation<T>
where
    NumberFor<<T as frame_system::Config>::Block>: AsPrimitive<usize>,
{
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        LightValidation { num_bridges: 0, tracked_bridges: BTreeMap::new(), grandpa_note_stalled: false }
    }

    pub fn initialize_bridge(
        &mut self,
        block_header: BlockHeader,
        validator_set: AuthoritySet,
        proof: StorageProof,
        grandpa_note_stalled: bool,
    ) -> Result<BridgeId> {
        let state_root = block_header.state_root();

        Self::check_validator_set_proof(state_root, proof, &validator_set.list, validator_set.id)
            .map_err(anyhow::Error::msg)?;

        let bridge_info = BridgeInfo::new(block_header, validator_set);

        let new_bridge_id = self.num_bridges + 1;
        self.tracked_bridges.insert(new_bridge_id, bridge_info);

        self.num_bridges = new_bridge_id;
        self.grandpa_note_stalled = grandpa_note_stalled;
        
        Ok(new_bridge_id)
    }

    /// Submits a sequence of block headers to the light client to validate
    ///
    /// The light client accepts a sequence of block headers, optionally with an authority set change
    /// in the last block. Without the authority set change, it assumes the authority set and the set
    /// id remains the same after submitting the blocks. One submission can have at most one authortiy
    /// set change (change.set_id == last_set_id + 1).
    pub fn submit_finalized_headers(
        &mut self,
        bridge_id: BridgeId,
        header: BlockHeader,
        ancestry_proof: Vec<BlockHeader>,
        grandpa_proof: EncodedJustification,
    ) -> Result<()> {
        let bridge = self
            .tracked_bridges
            .get(&bridge_id)
            .ok_or_else(|| anyhow::Error::msg(Error::NoSuchBridgeExists))?;

        // Check that the new header is a decendent of the old header
        let last_header = &bridge.last_finalized_block_header;
        verify_ancestry(ancestry_proof, last_header.hash(), &header)?;

        let block_hash = header.hash();
        let block_num = *header.number();

        // Check that the header has been finalized
        let voters = &bridge.current_set;
        let voter_set = VoterSet::new(voters.list.clone()).unwrap();
        let voter_set_id = voters.id;

        // We don't really care about the justification, as long as it's valid
        match GrandpaJustification::<<T as frame_system::Config>::Block>::decode_and_verify_finalizes(
            &grandpa_proof,
            (block_hash, block_num.into()),
            voter_set_id,
            &voter_set,
            self.grandpa_note_stalled,
        ) {
            Ok(_) => {},
            Err(JustificationError::JustificationDecode) if self.grandpa_note_stalled => {
                log::debug!("grandpa_note_stalled is true, ignore JustificationDecode error");
            },
            Err(e) => {
                return Err(anyhow::Error::msg(e));
            },
        }

        match self.tracked_bridges.get_mut(&bridge_id) {
            Some(bridge_info) => {
                if let Some(scheduled_change) = find_scheduled_change(&header) {
                    // GRANDPA only includes a `delay` for forced changes, so this isn't valid.
                    if scheduled_change.delay != 0 {
                        return Err(anyhow::Error::msg(Error::UnsupportedScheduledChangeDelay));
                    }

                    // Commit
                    bridge_info.current_set =
                        AuthoritySet { list: scheduled_change.next_authorities, id: voter_set_id + 1 }
                }
                bridge_info.last_finalized_block_header = header;
            },
            _ => panic!("We succesfully got this bridge earlier, therefore it exists; qed"),
        };

        Ok(())
    }

    pub fn validate_storage_proof(
        &self,
        state_root: T::Hash,
        proof: StorageProof,
        items: &[(&[u8], &[u8])], // &[(key, value)]
    ) -> Result<()> {
        let checker = StorageProofChecker::<T::Hashing>::new(state_root, proof)?;
        for (k, v) in items {
            let actual_value = checker
                .read_value(k)?
                .ok_or_else(|| anyhow::Error::msg(Error::StorageValueUnavailable))?;
            if actual_value.as_slice() != *v {
                return Err(anyhow::Error::msg(Error::StorageValueMismatch));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    // InvalidStorageProof,
    // StorageRootMismatch,
    StorageValueUnavailable,
    // InvalidValidatorSetProof,
    ValidatorSetMismatch,
    InvalidAncestryProof,
    NoSuchBridgeExists,
    InvalidFinalityProof,
    // UnknownClientError,
    // HeaderAncestryMismatch,
    // UnexpectedValidatorSetId,
    StorageValueMismatch,
    UnsupportedScheduledChangeDelay,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Error::StorageRootMismatch => write!(f, "storage root mismatch"),
            Error::StorageValueUnavailable => write!(f, "storage value unavailable"),
            Error::ValidatorSetMismatch => write!(f, "validator set mismatch"),
            Error::InvalidAncestryProof => write!(f, "invalid ancestry proof"),
            Error::NoSuchBridgeExists => write!(f, "no such bridge exists"),
            Error::InvalidFinalityProof => write!(f, "invalid finality proof"),
            // Error::HeaderAncestryMismatch => write!(f, "header ancestry mismatch"),
            // Error::UnexpectedValidatorSetId => write!(f, "unexpected validator set id"),
            Error::StorageValueMismatch => write!(f, "storage value mismatch"),
            Error::UnsupportedScheduledChangeDelay => write!(f, "scheduled change should not have delay"),
        }
    }
}

impl From<JustificationError> for Error {
    fn from(e: JustificationError) -> Self {
        match e {
            JustificationError::BadJustification(msg) => {
                error!("InvalidFinalityProof(BadJustification({}))", msg);
                Error::InvalidFinalityProof
            },
            JustificationError::JustificationDecode => {
                error!("InvalidFinalityProof(JustificationDecode)");
                Error::InvalidFinalityProof
            },
        }
    }
}

impl<T: Config> LightValidation<T>
where
    NumberFor<<T as frame_system::Config>::Block>: AsPrimitive<usize>,
{
    fn check_validator_set_proof(
        state_root: &T::Hash,
        proof: StorageProof,
        validator_set: &[(AuthorityId, AuthorityWeight)],
        _set_id: SetId,
    ) -> Result<()> {
        let checker = <StorageProofChecker<T::Hashing>>::new(*state_root, proof)?;

        // By encoding the given set we should have an easy way to compare
        // with the stuff we get out of storage via `read_value`
        let encoded_validator_set = validator_set.encode();

        let alt_key = utils::storage_prefix("Grandpa", "Authorities");
        let old_key = b":grandpa_authorities";

        let matches =
            if let Some(authorities) = checker.read_value(&alt_key).context("Faield to read Grandpa::Authorities")? {
                encoded_validator_set == authorities
            } else {
                let authorities = checker
                    .read_value(old_key)
                    .context("Faield to read :grandpa_authorities")?
                    .ok_or_else(|| anyhow::anyhow!("Missing grandpa authorities"))?;
                encoded_validator_set.get(..) == authorities.get(1..)
            };

        // TODO: check set_id
        // checker.read_value(grandpa::CurrentSetId.key())

        if matches {
            Ok(())
        } else {
            Err(anyhow::Error::msg(Error::ValidatorSetMismatch))
        }
    }
}

// A naive way to check whether a `child` header is a decendent
// of an `ancestor` header. For this it requires a proof which
// is a chain of headers between (but not including) the `child`
// and `ancestor`. This could be updated to use something like
// Log2 Ancestors (#2053) in the future.
fn verify_ancestry<H>(proof: Vec<H>, ancestor_hash: H::Hash, child: &H) -> Result<()>
where
    H: Header<Hash = H256>,
{
    {
        trace!("ancestor_hash: {}", ancestor_hash);
        for h in proof.iter() {
            trace!("block {:?} - hash: {} parent: {}", h.number(), h.hash(), h.parent_hash());
        }
        trace!("child block {:?} - hash: {} parent: {}", child.number(), child.hash(), child.parent_hash());
    }

    let mut parent_hash = child.parent_hash();
    if *parent_hash == ancestor_hash {
        return Ok(());
    }

    // If we find that the header's parent hash matches our ancestor's hash we're done
    for header in proof.iter() {
        // Need to check that blocks are actually related
        if header.hash() != *parent_hash {
            break;
        }

        parent_hash = header.parent_hash();
        if *parent_hash == ancestor_hash {
            return Ok(());
        }
    }

    Err(anyhow::Error::msg(Error::InvalidAncestryProof))
}

impl<T: Config> fmt::Debug for LightValidation<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LightValidationTest {{ num_bridges: {}, tracked_bridges: {:?} }}",
            self.num_bridges, self.tracked_bridges
        )
    }
}

impl<T: Config> fmt::Debug for BridgeInfo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BridgeInfo {{ last_finalized_block_header: {:?}, current_validator_set: {:?}, current_validator_set_id: {} }}",
			self.last_finalized_block_header, self.current_set.list, self.current_set.id)
    }
}

pub mod utils {
    use parity_scale_codec::Encode;

    fn calc_storage_prefix(module: &str, storage_item: &str) -> [u8; 32] {
        let module_hash = sp_core::twox_128(module.as_bytes());
        let storage_hash = sp_core::twox_128(storage_item.as_bytes());
        let mut final_key = [0u8; 32];
        final_key[..16].copy_from_slice(&module_hash);
        final_key[16..].copy_from_slice(&storage_hash);
        final_key
    }

    #[inline(always)]
    pub fn storage_prefix(module: &str, storage_item: &str) -> [u8; 32] {
        use hex_literal::hex;
        match (module, storage_item) {
            // Speed up well-known storage items
            // This let the compiler to optimize `storage_prefix("Grandpa", "Authorities")` to a constant.
            ("Grandpa", "Authorities") => {
                hex!("5f9cc45b7a00c5899361e1c6099678dc5e0621c4869aa60c02be9adcc98a0d1d")
            },
            _ => calc_storage_prefix(module, storage_item),
        }
    }

    /// Calculates the Substrate storage key prefix for a StorageMap
    #[allow(unused)]
    pub fn storage_map_prefix_twox_64_concat(
        module: &[u8],
        storage_item: &[u8],
        key: &(impl Encode + ?Sized),
    ) -> Vec<u8> {
        let mut bytes = sp_core::twox_128(module).to_vec();
        bytes.extend(&sp_core::twox_128(storage_item)[..]);
        let encoded = key.encode();
        bytes.extend(sp_core::twox_64(&encoded));
        bytes.extend(&encoded);
        bytes
    }
}
