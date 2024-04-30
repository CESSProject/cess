use super::master_key::CesealMasterKey;
use ces_crypto::{
    aead, key_share,
    rsa::RsaDer,
    sr25519::{Persistence as persist, KDF},
    SecretKey,
};
use ces_mq::{traits::MessageChannel, Sr25519Signer};
use ces_types::{
    messaging::{EncryptedKey, MasterKeyDistribution},
    EcdhPublicKey, WorkerPublicKey,
};
use serde::{Deserialize, Serialize};
use sp_core::{hashing, sr25519};
use std::convert::TryInto;
use tracing::info;

const MASTER_KEY_SHARING_SALT: &[u8] = b"master_key_sharing";

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub(crate) struct Keyfairy<MsgChan> {
    /// The current master key in use
    #[codec(skip)]
    master_key: CesealMasterKey,
    egress: MsgChan, // TODO: syncing the egress state while migrating.
    iv_seq: u64,
}

impl<MsgChan> Keyfairy<MsgChan>
where
    MsgChan: MessageChannel<Signer = Sr25519Signer> + Clone,
{
    pub fn new(master_key: CesealMasterKey, egress: MsgChan) -> Self {
        Self { master_key, egress, iv_seq: 0 }
    }

    fn generate_iv(&mut self, block_number: chain::BlockNumber) -> aead::IV {
        let derived_key = self
            .master_key
            .sr25519_keypair()
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

    pub fn master_key(&self) -> &CesealMasterKey {
        &self.master_key
    }

    pub fn master_pubkey(&self) -> sr25519::Public {
        self.master_key.sr25519_public_key()
    }

    pub fn share_master_key(
        &mut self,
        pubkey: &WorkerPublicKey,
        ecdh_pubkey: &EcdhPublicKey,
        block_number: chain::BlockNumber,
    ) {
        info!("Keyfairy: try dispatch master key");
        let rsa_secret_der = self.master_key.dump_rsa_secret_der();
        let encrypted_key = self.encrypt_key_to(&[MASTER_KEY_SHARING_SALT], ecdh_pubkey, &rsa_secret_der, block_number);
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
            self.master_key.sr25519_keypair(),
            key_derive_info,
            &ecdh_pubkey.0,
            &SecretKey::Rsa(secret_key.to_vec()),
            &iv,
        )
        .expect("should never fail with valid master key; qed.");
        EncryptedKey { ecdh_pubkey: sr25519::Public(ecdh_pubkey), encrypted_key, iv }
    }
}
