use super::RotatedMasterKey;
use crate::types::BlockDispatchContext;
use ces_crypto::{
    aead, key_share,
    sr25519::{Persistence, Sr25519SecretKey, KDF},
};
use ces_mq::{traits::MessageChannel, Sr25519Signer};
use ces_serde_more as more;
use ces_types::{
    messaging::{
        BatchRotateMasterKeyEvent, DispatchMasterKeyHistoryEvent, EncryptedKey, KeyDistribution, RotateMasterKeyEvent,
    },
    wrap_content_to_sign, EcdhPublicKey, SignedContentType, WorkerPublicKey,
};
use log::info;
use pallet_tee_worker::KeyfairyRegistryEvent;
use serde::{Deserialize, Serialize};
use sp_core::{hashing, sr25519, Pair};
use std::{collections::BTreeMap, convert::TryInto};

const MASTER_KEY_SHARING_SALT: &[u8] = b"master_key_sharing";

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub(crate) struct Keyfairy<MsgChan> {
    /// The current master key in use
    #[serde(with = "more::key_bytes")]
    #[codec(skip)]
    master_key: sr25519::Pair,
    /// This will be switched once when the first master key is uploaded
    master_pubkey_on_chain: bool,
    /// Unregistered GK will sync all the GK messages silently
    registered_on_chain: bool,
    #[serde(with = "more::scale_bytes")]
    master_key_history: Vec<RotatedMasterKey>,
    egress: MsgChan, // TODO: syncing the egress state while migrating.
    iv_seq: u64,
}

impl<MsgChan> Keyfairy<MsgChan>
where
    MsgChan: MessageChannel<Signer = Sr25519Signer> + Clone,
{
    pub fn new(master_key_history: Vec<RotatedMasterKey>, egress: MsgChan) -> Self {
        let master_key = sr25519::Pair::restore_from_secret_key(
            &master_key_history.first().expect("empty master key history").secret,
        );
        egress.set_dummy(true);

        Self {
            master_key,
            master_pubkey_on_chain: false,
            registered_on_chain: false,
            master_key_history,
            egress: egress.clone(),
            iv_seq: 0,
        }
    }

    fn generate_iv(&mut self, block_number: chain::BlockNumber) -> aead::IV {
        let derived_key = self
            .master_key
            .derive_sr25519_pair(&[b"iv_generator"])
            .expect("should not fail with valid info");

        let mut buf: Vec<u8> = Vec::new();
        buf.extend(derived_key.dump_secret_key().iter().copied());
        buf.extend(block_number.to_be_bytes().iter().copied());
        buf.extend(self.iv_seq.to_be_bytes().iter().copied());
        self.iv_seq += 1;

        let hash = hashing::blake2_256(buf.as_ref());
        hash[0..12].try_into().expect("should never fail given correct length; qed.")
    }

    pub fn register_on_chain(&mut self) {
        info!("Keyfairy: register on chain");
        self.egress.set_dummy(false);
        self.registered_on_chain = true;
    }

    pub fn unregister_on_chain(&mut self) {
        info!("Keyfairy: unregister on chain");
        self.egress.set_dummy(true);
        self.registered_on_chain = false;
    }

    pub fn registered_on_chain(&self) -> bool {
        self.registered_on_chain
    }

    pub fn master_pubkey_uploaded(&mut self, master_pubkey: sr25519::Public) {
        #[cfg(not(feature = "shadow-gk"))]
        assert_eq!(self.master_key.public(), master_pubkey, "local and on-chain master key mismatch");
        self.master_pubkey_on_chain = true;
    }

    pub fn master_pubkey(&self) -> sr25519::Public {
        self.master_key.public()
    }

    pub fn master_key_history(&self) -> &Vec<RotatedMasterKey> {
        &self.master_key_history
    }

    /// Return whether the history is really updated
    pub fn set_master_key_history(&mut self, master_key_history: &Vec<RotatedMasterKey>) -> bool {
        if master_key_history.len() <= self.master_key_history.len() {
            return false
        }
        self.master_key_history = master_key_history.clone();
        true
    }

    /// Append the rotated key to Keyfairy's master key history, return whether the history is really updated
    pub fn append_master_key(&mut self, rotated_master_key: RotatedMasterKey) -> bool {
        if !self.master_key_history.contains(&rotated_master_key) {
            // the rotation id must be in order
            assert!(
                rotated_master_key.rotation_id == self.master_key_history.len() as u64,
                "Keyfairy Master key history corrupted"
            );
            self.master_key_history.push(rotated_master_key);
            return true
        }
        false
    }

    /// Update the master key and Keyfairy mq, return whether the key switch succeeds
    pub fn switch_master_key(&mut self, rotation_id: u64, block_height: chain::BlockNumber) -> bool {
        let raw_key = self.master_key_history.get(rotation_id as usize);
        if raw_key.is_none() {
            return false
        }

        let raw_key = raw_key.expect("checked; qed.");
        assert!(
            raw_key.rotation_id == rotation_id && raw_key.block_height == block_height,
            "Keyfairy Master key history corrupted"
        );
        let new_master_key = sr25519::Pair::restore_from_secret_key(&raw_key.secret);
        // send the RotatedMasterPubkey event with old master key
        let master_pubkey = new_master_key.public();
        self.egress.push_message(&KeyfairyRegistryEvent::RotatedMasterPubkey {
            rotation_id: raw_key.rotation_id,
            master_pubkey,
        });

        self.master_key = new_master_key;
        self.egress.set_signer(self.master_key.clone().into());
        true
    }

    pub fn share_master_key(
        &mut self,
        pubkey: &WorkerPublicKey,
        ecdh_pubkey: &EcdhPublicKey,
        block_number: chain::BlockNumber,
    ) {
        if self.master_key_history.len() > 1 {
            self.share_master_key_history(pubkey, ecdh_pubkey, block_number);
        } else {
            self.share_latest_master_key(pubkey, ecdh_pubkey, block_number);
        }
    }

    pub fn share_latest_master_key(
        &mut self,
        pubkey: &WorkerPublicKey,
        ecdh_pubkey: &EcdhPublicKey,
        block_number: chain::BlockNumber,
    ) {
        info!("Keyfairy: try dispatch master key");
        let master_key = self.master_key.dump_secret_key();
        let encrypted_key = self.encrypt_key_to(&[MASTER_KEY_SHARING_SALT], ecdh_pubkey, &master_key, block_number);
        self.egress
            .push_message(&KeyDistribution::<chain::BlockNumber>::master_key_distribution(
                *pubkey,
                encrypted_key.ecdh_pubkey,
                encrypted_key.encrypted_key,
                encrypted_key.iv,
            ));
    }

    pub fn share_master_key_history(
        &mut self,
        pubkey: &WorkerPublicKey,
        ecdh_pubkey: &EcdhPublicKey,
        block_number: chain::BlockNumber,
    ) {
        info!("Keyfairy: try dispatch all historical master keys");
        let encrypted_master_key_history = self
            .master_key_history
            .clone()
            .iter()
            .map(|key| {
                (
                    key.rotation_id,
                    key.block_height,
                    self.encrypt_key_to(&[MASTER_KEY_SHARING_SALT], ecdh_pubkey, &key.secret, block_number),
                )
            })
            .collect();
        self.egress
            .push_message(&KeyDistribution::MasterKeyHistory(DispatchMasterKeyHistoryEvent {
                dest: *pubkey,
                encrypted_master_key_history,
            }));
    }

    pub fn process_master_key_rotation_request(
        &mut self,
        block: &BlockDispatchContext,
        event: RotateMasterKeyEvent,
        identity_key: sr25519::Pair,
    ) {
        let new_master_key = crate::new_sr25519_key();
        let secret_key = new_master_key.dump_secret_key();
        let secret_keys: BTreeMap<_, _> = event
            .gk_identities
            .into_iter()
            .map(|gk_identity| {
                let encrypted_key = self.encrypt_key_to(
                    &[MASTER_KEY_SHARING_SALT],
                    &gk_identity.ecdh_pubkey,
                    &secret_key,
                    block.block_number,
                );
                (gk_identity.pubkey, encrypted_key)
            })
            .collect();
        let mut event = BatchRotateMasterKeyEvent {
            rotation_id: event.rotation_id,
            secret_keys,
            sender: identity_key.public(),
            sig: vec![],
        };
        let data_to_sign = event.data_be_signed();
        let data_to_sign = wrap_content_to_sign(&data_to_sign, SignedContentType::MasterKeyRotation);
        event.sig = identity_key.sign(&data_to_sign).0.to_vec();
        self.egress
            .push_message(&KeyDistribution::<chain::BlockNumber>::MasterKeyRotation(event));
    }

    /// Manually encrypt the secret key for sharing
    ///
    /// The encrypted key intends to be shared through public channel in a broadcast way,
    /// so it is possible to share one key to multiple parties in one message.
    ///
    /// For end-to-end secret sharing, use `SecretMessageChannel`.
    fn encrypt_key_to(
        &mut self,
        key_derive_info: &[&[u8]],
        ecdh_pubkey: &EcdhPublicKey,
        secret_key: &Sr25519SecretKey,
        block_number: chain::BlockNumber,
    ) -> EncryptedKey {
        let iv = self.generate_iv(block_number);
        let (ecdh_pubkey, encrypted_key) =
            key_share::encrypt_secret_to(&self.master_key, key_derive_info, &ecdh_pubkey.0, secret_key, &iv)
                .expect("should never fail with valid master key; qed.");
        EncryptedKey { ecdh_pubkey: sr25519::Public(ecdh_pubkey), encrypted_key, iv }
    }
}
