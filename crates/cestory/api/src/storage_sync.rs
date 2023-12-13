use super::blocks::{
    AuthoritySetChange, BlockHeaderWithChanges, HeaderToSync, RuntimeHasher, StorageProof,
};

use alloc::string::String;
use alloc::vec::Vec;
use chain::Hash;
use derive_more::Display;
use im::Vector as VecDeque;
use serde::{Deserialize, Serialize};

type Storage = ces_trie_storage::TrieStorage<RuntimeHasher>;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Display, Debug)]
pub enum Error {
    /// No header or block data in the request
    EmptyRequest,
    /// No Justification found in the last header
    MissingJustification,
    /// Header validation failed
    #[display("HeaderValidateFailed({_0})")]
    HeaderValidateFailed(String),
    /// Storage proof failed
    #[display("StorageProofFailed({_0})")]
    StorageProofFailed(String),
    /// Some header parent hash mismatches it's parent header
    HeaderHashMismatch,
    /// The end block number mismatch the expecting next block number
    BlockNumberMismatch,
    /// No state root to validate the storage changes
    NoStateRoot,
    /// Invalid storage changes that cause the state root mismatch
    #[display("StateRootMismatch block={block:?} expected={expected:?} actual={actual:?}")]
    StateRootMismatch {
        block: chain::BlockNumber,
        expected: chain::Hash,
        actual: chain::Hash,
    },
    /// Solo/Para mode mismatch
    ChainModeMismatch,
    CannotLoadStateAfterSyncing,
}

impl std::error::Error for Error {}

pub trait BlockValidator {
    fn submit_finalized_headers(
        &mut self,
        bridge_id: u64,
        header: chain::Header,
        ancestry_proof: Vec<chain::Header>,
        grandpa_proof: Vec<u8>,
        auhtority_set_change: Option<AuthoritySetChange>,
    ) -> Result<()>;

    fn validate_storage_proof(
        &self,
        state_root: Hash,
        proof: StorageProof,
        items: &[(&[u8], &[u8])],
    ) -> Result<()>;
}

pub trait StorageSynchronizer {
    /// Return the next block numbers to sync.
    fn counters(&self) -> Counters;

    /// Given chain headers in sequence, validate it and output the state_roots
    fn sync_header(
        &mut self,
        headers: Vec<HeaderToSync>,
        authority_set_change: Option<AuthoritySetChange>,
    ) -> Result<chain::BlockNumber>;

    /// Feed in a block of storage changes
    fn feed_block(
        &mut self,
        block: &BlockHeaderWithChanges,
        storage: &mut Storage,
        drop_proofs: bool,
    ) -> Result<()>;

    /// Assume synced to given block.
    fn assume_at_block(&mut self, block_number: chain::BlockNumber) -> Result<()>;
    fn state_validated(&self) -> bool;
}

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub struct BlockSyncState<Validator> {
    validator: Validator,
    main_bridge: u64,
    /// The next block number of header to be sync.
    header_number_next: chain::BlockNumber,
    /// The next block number of storage to be sync.
    block_number_next: chain::BlockNumber,
    genesis_state_validated: bool,
}

impl<Validator> BlockSyncState<Validator>
where
    Validator: BlockValidator,
{
    pub fn new(
        validator: Validator,
        main_bridge: u64,
        header_number_next: chain::BlockNumber,
        block_number_next: chain::BlockNumber,
    ) -> Self {
        Self {
            validator,
            main_bridge,
            header_number_next,
            block_number_next,
            genesis_state_validated: false,
        }
    }

    /// Given chain headers in sequence, validate it and output the state_roots
    pub fn sync_header(
        &mut self,
        mut headers: Vec<HeaderToSync>,
        authority_set_change: Option<AuthoritySetChange>,
        state_roots: &mut VecDeque<Hash>,
        skip_blocks_below: chain::BlockNumber,
    ) -> Result<chain::BlockNumber> {
        headers.retain(|header| header.header.number >= self.header_number_next);

        let first_header = match headers.first() {
            Some(header) => header,
            None => return Ok(self.header_number_next - 1),
        };
        if first_header.header.number != self.header_number_next {
            return Err(Error::BlockNumberMismatch);
        }

        // Light validation when possible
        let last_header = headers.last().ok_or(Error::EmptyRequest)?;

        {
            // 1. the last header must has justification
            let justification = last_header
                .justification
                .as_ref()
                .ok_or(Error::MissingJustification)?
                .clone();
            let last_header = last_header.header.clone();
            // 2. check header sequence
            for (i, header) in headers.iter().enumerate() {
                if i > 0 && headers[i - 1].header.hash() != header.header.parent_hash {
                    log::error!(
                        "Parent hash of {} mismatch: actual={:?} expected={:?}",
                        header.header.number,
                        headers[i - 1].header.hash(),
                        header.header.parent_hash
                    );
                    return Err(Error::HeaderHashMismatch);
                }
            }
            // 3. generate accenstor proof
            let mut accenstor_proof: Vec<_> = headers[0..headers.len() - 1]
                .iter()
                .map(|h| h.header.clone())
                .collect();
            accenstor_proof.reverse(); // from high to low
                                       // 4. submit to light client
            let bridge_id = self.main_bridge;
            self.validator.submit_finalized_headers(
                bridge_id,
                last_header,
                accenstor_proof,
                justification,
                authority_set_change,
            )?;
        }

        // Save the block hashes for future dispatch
        for header in headers.iter() {
            if header.header.number < skip_blocks_below {
                continue;
            }
            state_roots.push_back(header.header.state_root);
        }

        self.header_number_next = last_header.header.number + 1;

        Ok(last_header.header.number)
    }

    /// Feed a block and apply changes to storage if it's valid.
    pub fn feed_block(
        &mut self,
        block: &BlockHeaderWithChanges,
        state_roots: &mut VecDeque<Hash>,
        storage: &mut Storage,
        drop_proofs: bool,
    ) -> Result<()> {
        if block.block_header.number != self.block_number_next {
            return Err(Error::BlockNumberMismatch);
        }
        if drop_proofs {
            self.block_number_next += 1;
            let root = state_roots.pop_front().ok_or(Error::NoStateRoot)?;
            storage.set_root(root);
            return Ok(());
        }

        if !self.genesis_state_validated {
            let genesis_state_root = state_roots.get(0).ok_or(Error::NoStateRoot)?;
            if storage.root() != genesis_state_root {
                return Err(Error::StateRootMismatch {
                    block: self.block_number_next - 1,
                    expected: *genesis_state_root,
                    actual: *storage.root(),
                });
            }
            _ = state_roots.pop_front();
            self.genesis_state_validated = true;
        }

        let expected_root = state_roots.get(0).ok_or(Error::NoStateRoot)?;

        let changes = &block.storage_changes;

        log::debug!(
            "calc root ({}, {})",
            changes.main_storage_changes.len(),
            changes.child_storage_changes.len()
        );
        let (state_root, transaction) = storage.calc_root_if_changes(
            &changes.main_storage_changes,
            &changes.child_storage_changes,
        );

        if expected_root != &state_root {
            return Err(Error::StateRootMismatch {
                block: block.block_header.number,
                expected: *expected_root,
                actual: state_root,
            });
        }

        log::debug!("apply changes");
        storage.apply_changes(state_root, transaction);
        log::debug!("applied");

        self.block_number_next += 1;
        state_roots.pop_front();
        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct Counters {
    pub next_header_number: chain::BlockNumber,
    pub next_block_number: chain::BlockNumber,
}

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub struct SolochainSynchronizer<Validator> {
    sync_state: BlockSyncState<Validator>,
    #[codec(skip)]
    state_roots: VecDeque<Hash>,
}

impl<Validator: BlockValidator> SolochainSynchronizer<Validator> {
    pub fn new(validator: Validator, main_bridge: u64) -> Self {
        Self {
            sync_state: BlockSyncState::new(validator, main_bridge, 0, 1),
            state_roots: Default::default(),
        }
    }
}

impl<Validator: BlockValidator> StorageSynchronizer for SolochainSynchronizer<Validator> {
    fn counters(&self) -> Counters {
        Counters {
            next_block_number: self.sync_state.block_number_next,
            next_header_number: self.sync_state.header_number_next,
        }
    }

    fn sync_header(
        &mut self,
        headers: Vec<HeaderToSync>,
        authority_set_change: Option<AuthoritySetChange>,
    ) -> Result<chain::BlockNumber> {
        let skip_blocks_below = self.sync_state.block_number_next.checked_sub(1).unwrap();
        self.sync_state.sync_header(
            headers,
            authority_set_change,
            &mut self.state_roots,
            skip_blocks_below,
        )
    }

    fn feed_block(
        &mut self,
        block: &BlockHeaderWithChanges,
        storage: &mut Storage,
        drop_proofs: bool,
    ) -> Result<()> {
        self.sync_state
            .feed_block(block, &mut self.state_roots, storage, drop_proofs)
    }

    fn assume_at_block(&mut self, block_number: chain::BlockNumber) -> Result<()> {
        if self.sync_state.block_number_next > 1 || self.sync_state.header_number_next > 0 {
            return Err(Error::CannotLoadStateAfterSyncing);
        }

        self.sync_state.block_number_next = block_number + 1;
        Ok(())
    }

    fn state_validated(&self) -> bool {
        self.sync_state.genesis_state_validated
    }
}

// We create this new type to help serialize the original dyn StorageSynchronizer.
// Because it it impossible to impl Serialize/Deserialize for dyn StorageSynchronizer.
#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub enum Synchronizer<Validator> {
    Solo(SolochainSynchronizer<Validator>),
}

impl<Validator: BlockValidator> Synchronizer<Validator> {

    pub fn new_solochain(validator: Validator, main_bridge: u64) -> Self {
        Self::Solo(SolochainSynchronizer::new(validator, main_bridge))
    }

    pub fn as_dyn(&self) -> &dyn StorageSynchronizer {
        match self {
            Self::Solo(s) => s,
        }
    }

    pub fn as_dyn_mut(&mut self) -> &mut dyn StorageSynchronizer {
        match self {
            Self::Solo(s) => s,
        }
    }
}

impl<Validator: BlockValidator> StorageSynchronizer for Synchronizer<Validator> {
    fn counters(&self) -> Counters {
        self.as_dyn().counters()
    }

    fn sync_header(
        &mut self,
        headers: Vec<HeaderToSync>,
        authority_set_change: Option<AuthoritySetChange>,
    ) -> Result<chain::BlockNumber> {
        self.as_dyn_mut().sync_header(headers, authority_set_change)
    }

    fn feed_block(
        &mut self,
        block: &BlockHeaderWithChanges,
        storage: &mut Storage,
        drop_proofs: bool,
    ) -> Result<()> {
        self.as_dyn_mut().feed_block(block, storage, drop_proofs)
    }

    fn assume_at_block(&mut self, block_number: chain::BlockNumber) -> Result<()> {
        self.as_dyn_mut().assume_at_block(block_number)
    }

    fn state_validated(&self) -> bool {
        self.as_dyn().state_validated()
    }
}
