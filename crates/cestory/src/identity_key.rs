use crate::{new_sr25519_key, pal::Sealing};
use anyhow::Result;
use ces_crypto::{
    ecdh::EcdhKey,
    sr25519::{Persistence as Persist, Sr25519SecretKey, KDF},
};
use parity_scale_codec::{Decode, Encode};
use sp_core::{
    sr25519::{self, Signature},
    Pair,
};
use std::{fmt, path::PathBuf, str::FromStr};

pub type PublicKey = sr25519::Public;

#[derive(Clone)]
pub struct IdentityKey {
    pub key_pair: sr25519::Pair,
    pub trusted_sk: bool,
    pub dev_mode: bool,
}

impl fmt::Debug for IdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityKey")
            .field("key_pair", &self.key_pair.dump_secret_key())
            .field("trusted_sk", &self.trusted_sk)
            .field("dev_mode", &self.dev_mode)
            .finish()
    }
}

impl IdentityKey {
    pub fn build<P: Sealing>(platform: &P, sealing_path: &str, debug_set_key: Option<Vec<u8>>) -> Result<Self> {
        let id_key_path = PathBuf::from_str(sealing_path)?;
        let id_key = if let Some(ref raw_key) = debug_set_key {
            let key_pair = sr25519::Pair::from_seed_slice(&raw_key)?;
            let id_key = IdentityKey { key_pair, trusted_sk: false, dev_mode: true };
            persistence::save_identity(&id_key, platform, &id_key_path)?;
            id_key
        } else {
            use persistence::*;
            match load_identity(platform, &id_key_path)? {
                IdKeyLoadResult::Loaded(ik) => ik,
                IdKeyLoadResult::NotFound => {
                    let id_key = IdentityKey { key_pair: new_sr25519_key(), trusted_sk: true, dev_mode: false };
                    save_identity(&id_key, platform, &id_key_path)?;
                    id_key
                },
            }
        };
        Ok(id_key)
    }

    pub fn key_pair(&self) -> &sr25519::Pair {
        &self.key_pair
    }

    pub fn public_key(&self) -> PublicKey {
        self.key_pair.public()
    }

    pub fn ecdh_key(&self) -> EcdhKey {
        self.key_pair.derive_ecdh_key().expect("Unable to derive ecdh key")
    }

    pub fn ecdh_public_key(&self) -> PublicKey {
        PublicKey::from_raw(self.ecdh_key().public())
    }

    pub fn sign(&self, message: &[u8]) -> Signature {
        self.key_pair.sign(message)
    }

    pub fn dump_secret_key(&self) -> Sr25519SecretKey {
        self.key_pair.dump_secret_key()
    }
}

mod persistence {
    use super::*;
    use anyhow::anyhow;
    use std::path::Path;

    const RUNTIME_SEALED_DATA_FILE: &str = "id_key.seal";

    pub enum IdKeyLoadResult {
        Loaded(IdentityKey),
        NotFound,
    }

    #[derive(Encode, Decode, Clone, Debug)]
    pub struct PersistIdentityKey {
        sk: Sr25519SecretKey,
        trusted_sk: bool,
        dev_mode: bool,
    }

    impl From<IdentityKey> for PersistIdentityKey {
        fn from(id_key: IdentityKey) -> Self {
            PersistIdentityKey {
                sk: id_key.dump_secret_key(),
                trusted_sk: id_key.trusted_sk,
                dev_mode: id_key.dev_mode,
            }
        }
    }

    impl From<PersistIdentityKey> for IdentityKey {
        fn from(pik: PersistIdentityKey) -> Self {
            IdentityKey {
                key_pair: sr25519::Pair::restore_from_secret_key(&pik.sk),
                trusted_sk: pik.trusted_sk,
                dev_mode: pik.dev_mode,
            }
        }
    }

    pub fn load_identity<S: Sealing>(sealer: &S, sealing_path: &Path) -> Result<IdKeyLoadResult> {
        let filepath = sealing_path.join(RUNTIME_SEALED_DATA_FILE);
        let data = sealer
            .unseal_data(filepath)
            .map_err(|e| anyhow!("unseal identity key error: {:?}", e))?;
        if let Some(data) = data {
            let pik: PersistIdentityKey = Decode::decode(&mut &data[..])?;
            return Ok(IdKeyLoadResult::Loaded(pik.into()));
        }
        Ok(IdKeyLoadResult::NotFound)
    }

    pub fn save_identity<S: Sealing>(id_key: &IdentityKey, sealer: &S, sealing_path: &Path) -> Result<()> {
        let pik: PersistIdentityKey = id_key.clone().into();
        let encoded_vec = pik.encode();
        let filepath = sealing_path.join(RUNTIME_SEALED_DATA_FILE);
        sealer
            .seal_data(filepath, &encoded_vec)
            .map_err(|_| anyhow!("failed to seal identity key"))?;
        Ok(())
    }
}
