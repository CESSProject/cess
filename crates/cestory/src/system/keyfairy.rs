use super::RotatedMasterKey;

use ces_crypto::{
    aead, key_share,
    rsa::{Persistence, RsaDer},
    sr25519::{Persistence as persist, KDF},
    SecretKey,
};
use ces_mq::{traits::MessageChannel, Sr25519Signer};
use ces_serde_more as more;
use ces_types::{
    messaging::{EncryptedKey, MasterKeyDistribution},
    EcdhPublicKey, WorkerPublicKey,
};
use log::info;
use serde::{Deserialize, Serialize};
use sp_core::{hashing, sr25519, Pair};
use std::convert::TryInto;

const MASTER_KEY_SHARING_SALT: &[u8] = b"master_key_sharing";

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub(crate) struct Keyfairy<MsgChan> {
    /// The current master key in use
    #[serde(with = "more::key_bytes")]
    #[codec(skip)]
    master_key: sr25519::Pair,
    #[serde(with = "more::rsa_key_bytes")]
    #[codec(skip)]
    rsa_key: rsa::RsaPrivateKey,
    /// This will be switched once when the first master key is uploaded
    master_pubkey_on_chain: bool,
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
        let rsa_key =
            rsa::RsaPrivateKey::restore_from_der(&master_key_history.first().expect("empty master key history").secret)
                .expect("fail convert sr25519 pair from rsa skey when process new event");
        let master_key = crate::get_sr25519_from_rsa_key(rsa_key.clone());

        Self {
            master_key,
            rsa_key,
            master_pubkey_on_chain: false,
            master_key_history,
            egress,
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

    pub fn master_pubkey_uploaded(&mut self, master_pubkey: sr25519::Public) {
        #[cfg(not(feature = "shadow-gk"))]
        assert_eq!(self.master_key.public(), master_pubkey, "local and on-chain master key mismatch");
        self.master_pubkey_on_chain = true;
    }

    pub fn master_pubkey(&self) -> sr25519::Public {
        self.master_key.public()
    }

    /// Return whether the history is really updated
    pub fn set_master_key_history(&mut self, master_key_history: &Vec<RotatedMasterKey>) -> bool {
        if master_key_history.len() <= self.master_key_history.len() {
            return false
        }
        self.master_key_history = master_key_history.clone();
        true
    }

    pub fn share_master_key(
        &mut self,
        pubkey: &WorkerPublicKey,
        ecdh_pubkey: &EcdhPublicKey,
        block_number: chain::BlockNumber,
    ) {
        self.share_latest_master_key(pubkey, ecdh_pubkey, block_number);
    }

    pub fn share_latest_master_key(
        &mut self,
        pubkey: &WorkerPublicKey,
        ecdh_pubkey: &EcdhPublicKey,
        block_number: chain::BlockNumber,
    ) {
        info!("Keyfairy: try dispatch master key");
        let master_key = self.rsa_key.dump_secret_der();
        let encrypted_key = self.encrypt_key_to(&[MASTER_KEY_SHARING_SALT], ecdh_pubkey, &master_key, block_number);
        self.egress.push_message(&MasterKeyDistribution::distribution(
            *pubkey,
            encrypted_key.ecdh_pubkey,
            encrypted_key.encrypted_key,
            encrypted_key.iv,
        ));
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
        secret_key: &RsaDer,
        block_number: chain::BlockNumber,
    ) -> EncryptedKey {
        let iv = self.generate_iv(block_number);
        let (ecdh_pubkey, encrypted_key) = key_share::encrypt_secret_to(
            &self.master_key,
            key_derive_info,
            &ecdh_pubkey.0,
            &SecretKey::Rsa(secret_key.to_vec()),
            &iv,
        )
        .expect("should never fail with valid master key; qed.");
        EncryptedKey { ecdh_pubkey: sr25519::Public(ecdh_pubkey), encrypted_key, iv }
    }

    pub fn rsa_private_key(&self) -> &rsa::RsaPrivateKey {
        &self.rsa_key
    }

    pub fn master_key(&self) -> &sr25519::Pair {
        &self.master_key
    }

    pub fn podr2_key_pair(&self) -> ces_pdp::Keys {
        ces_pdp::gen_keypair_from_private_key(self.rsa_key.clone())
    }
}
