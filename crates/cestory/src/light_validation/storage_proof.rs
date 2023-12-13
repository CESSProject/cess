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

//! Logic for checking Substrate storage proofs.

use anyhow::Result;
use hash_db::{HashDB, Hasher, EMPTY_PREFIX};
use sp_trie::trie_types::TrieDBBuilder;
use sp_trie::{trie_types::TrieDB, MemoryDB, Trie};

use super::Error;

pub(crate) type StorageProof = Vec<Vec<u8>>;

/// This struct is used to read storage values from a subset of a Merklized database. The "proof"
/// is a subset of the nodes in the Merkle structure of the database, so that it provides
/// authentication against a known Merkle root as well as the values in the database themselves.
pub struct StorageProofChecker<H>
where
    H: Hasher,
{
    root: H::Out,
    db: MemoryDB<H>,
}

impl<H> StorageProofChecker<H>
where
    H: Hasher,
{
    /// Constructs a new storage proof checker.
    ///
    /// This returns an error if the given proof is invalid with respect to the given root.
    pub fn new(root: H::Out, proof: StorageProof) -> Result<Self> {
        let mut db = MemoryDB::default();
        for item in proof {
            db.insert(EMPTY_PREFIX, &item);
        }
        let checker = StorageProofChecker { root, db };
        // Return error if trie would be invalid.
        let _ = checker.trie();
        Ok(checker)
    }

    /// Reads a value from the available subset of storage. If the value cannot be read due to an
    /// incomplete or otherwise invalid proof, this returns an error.
    pub fn read_value(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.trie()
            .get(key)
            .map(|value| value.map(|value| value.to_vec()))
            .map_err(|_| anyhow::Error::msg(Error::StorageValueUnavailable))
    }

    fn trie(&self) -> TrieDB<H> {
        TrieDBBuilder::new(&self.db, &self.root).build()
    }
}