use crate::light_validation::{storage_proof::StorageProof, LightValidation};
use cestory_api::storage_sync::{BlockValidator, Error as SyncError, Result};
use std::string::ToString;

pub use storage_ext::ChainStorage;

impl BlockValidator for LightValidation<chain::Runtime> {
    fn submit_finalized_headers(
        &mut self,
        bridge_id: u64,
        header: chain::Header,
        ancestry_proof: Vec<chain::Header>,
        grandpa_proof: Vec<u8>,
        authority_set_change: Option<cestory_api::blocks::AuthoritySetChange>,
    ) -> Result<()> {
        self.submit_finalized_headers(
            bridge_id,
            header,
            ancestry_proof,
            grandpa_proof,
            authority_set_change,
        )
        .map_err(|e| SyncError::HeaderValidateFailed(e.to_string()))
    }

    fn validate_storage_proof(
        &self,
        state_root: chain::Hash,
        proof: StorageProof,
        items: &[(&[u8], &[u8])],
    ) -> Result<()> {
        self.validate_storage_proof(state_root, proof, items)
            .map_err(|e| SyncError::StorageProofFailed(e.to_string()))
    }
}

mod storage_ext {
    use crate::{chain, light_validation::utils::storage_prefix};
    use chain::{pallet_mq, pallet_registry};
    use log::error;
    use parity_scale_codec::{Decode, Error};
    use ces_mq::{Message, MessageOrigin};
    use ces_trie_storage::TrieStorage;    
    use serde::{Deserialize, Serialize};
    use sp_state_machine::{Ext, OverlayedChanges};

    #[derive(Serialize, Deserialize, Default)]
    pub struct ChainStorage {
        trie_storage: TrieStorage<crate::RuntimeHasher>,
    }

    impl Clone for ChainStorage {
        fn clone(&self) -> Self {
            Self {
                trie_storage: self.trie_storage.snapshot(),
            }
        }
    }

    impl From<TrieStorage<crate::RuntimeHasher>> for ChainStorage {
        fn from(value: TrieStorage<crate::RuntimeHasher>) -> Self {
            Self {
                trie_storage: value,
            }
        }
    }

    impl ChainStorage {
        fn get_raw(&self, key: impl AsRef<[u8]>) -> Option<Vec<u8>> {
            self.trie_storage.get(key)
        }
        fn get_decoded_result<T: Decode>(&self, key: impl AsRef<[u8]>) -> Result<Option<T>, Error> {
            self.get_raw(key)
                .map(|v| match Decode::decode(&mut &v[..]) {
                    Ok(decoded) => Ok(decoded),
                    Err(e) => {
                        error!("Decode storage value failed: {}", e);
                        Err(e)
                    }
                })
                .transpose()
        }
    }

    impl ChainStorage {
        pub fn from_pairs(
            pairs: impl Iterator<Item = (impl AsRef<[u8]>, impl AsRef<[u8]>)>,
        ) -> Self {
            let mut me = Self::default();
            me.load(pairs);
            me
        }

        pub fn snapshot(&self) -> Self {
            Self {
                trie_storage: self.trie_storage.snapshot(),
            }
        }

        pub fn load(&mut self, pairs: impl Iterator<Item = (impl AsRef<[u8]>, impl AsRef<[u8]>)>) {
            self.trie_storage.load(pairs);
        }

        pub fn root(&self) -> &sp_core::H256 {
            self.trie_storage.root()
        }

        pub fn inner(&self) -> &TrieStorage<crate::RuntimeHasher> {
            &self.trie_storage
        }

        pub fn inner_mut(&mut self) -> &mut TrieStorage<crate::RuntimeHasher> {
            &mut self.trie_storage
        }

        pub fn execute_with<R>(&self, f: impl FnOnce() -> R) -> R {
            let backend = self.trie_storage.as_trie_backend();
            let mut overlay = OverlayedChanges::default();
            let mut ext = Ext::new(&mut overlay, backend, None);
            sp_externalities::set_and_run_with_externalities(&mut ext, f)
        }

        pub fn mq_messages(&self) -> Result<Vec<Message>, Error> {
            for key in ["OutboundMessagesV2", "OutboundMessages"] {
                let messages: Vec<Message> = self
                    .get_decoded_result(storage_prefix("CesMq", key))
                    .map(|v| v.unwrap_or_default())?;
                if !messages.is_empty() {
                    info!("Got {} messages from {key}", messages.len());
                    return Ok(messages);
                }
            }
            Ok(vec![])
        }

        pub fn timestamp_now(&self) -> chain::Moment {
            self.execute_with(chain::Timestamp::now)
        }

        /// Get the next mq sequnce number for given sender. Default to 0 if no message sent.
        pub fn mq_sequence(&self, sender: &MessageOrigin) -> u64 {
            self.execute_with(|| pallet_mq::OffchainIngress::<chain::Runtime>::get(sender))
                .unwrap_or(0)
        }

        /// Return `None` if given ceseal hash is not allowed on-chain
        pub(crate) fn get_ceseal_bin_added_at(
            &self,
            runtime_hash: &[u8],
        ) -> Option<chain::BlockNumber> {
            self.execute_with(|| {
                pallet_registry::CesealBinAddedAt::<chain::Runtime>::get(runtime_hash)
            })
        }

        pub(crate) fn keyfairys(&self) -> Vec<ces_types::WorkerPublicKey> {
            self.execute_with(pallet_registry::Keyfairies::<chain::Runtime>::get)
        }

        pub(crate) fn is_worker_registered(&self, worker: &ces_types::WorkerPublicKey) -> bool {
            self.execute_with(|| pallet_registry::Workers::<chain::Runtime>::get(worker))
                .is_some()
        }

        pub(crate) fn minimum_ceseal_version(&self) -> (u32, u32, u32) {
            self.execute_with(pallet_registry::MinimumCesealVersion::<chain::Runtime>::get)
        }

        pub(crate) fn is_ceseal_bin_in_whitelist(&self, measurement: &[u8]) -> bool {
            let list = self.execute_with(pallet_registry::CesealBinAllowList::<chain::Runtime>::get);
            for hash in list.iter() {
                if hash.starts_with(measurement) {
                    return true;
                }
            }
            false
        }
    }
}
