use crate::pal::Sealing;
use ces_crypto::{
    rsa::{Persistence, RsaDer},
    CryptoError,
};
use ces_serde_more as more;
use rsa::{rand_core::OsRng, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no sealed master key")]
    NoSealedMasterKey,

    #[error("unseal master key error: {0}")]
    UnsealError(String),

    #[error("decode master key error: {0}")]
    DecodeMasterKey(String),

    #[error("broken sealed master key")]
    MasterKeyBroken,

    #[error("RSA private key to PKCS#8-encoding error: {0}")]
    Pkcs8Encode(String),

    #[error("RSA public key PKCS#1-encoding error: {0}")]
    Pkcs1Encode(String),

    #[error(transparent)]
    RsaError(#[from] rsa::errors::Error),

    #[error(transparent)]
    SecretString(#[from] sp_core::crypto::SecretStringError),

    #[error("ces-crypto error: {0:?}")]
    CesCrypto(CryptoError),
}

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub struct CesealMasterKey {
    #[serde(with = "more::rsa_key_bytes")]
    #[codec(skip)]
    pub(crate) rsa_priv_key: RsaPrivateKey,

    #[serde(with = "more::key_bytes")]
    #[codec(skip)]
    pub(crate) sr25519_keypair: sr25519::Pair,
}

fn derive_to_sr25519_key_by(rsa_priv_key: RsaPrivateKey) -> Result<sr25519::Pair, Error> {
    use crypto::{digest::Digest, sha2::Sha256};
    use rsa::pkcs8::EncodePrivateKey;
    let rsa_key_byte = rsa_priv_key
        .to_pkcs8_der()
        .map_err(|e| Error::Pkcs8Encode(e.to_string()))?
        .to_bytes();
    let mut hasher = Sha256::new();
    hasher.input(&rsa_key_byte);
    let mut seed = vec![0u8; hasher.output_bytes()];
    hasher.result(&mut seed);
    let pair = sr25519::Pair::from_seed_slice(&seed).map_err(Error::SecretString)?;
    Ok(pair)
}

fn generate_rsa_private_key(bit_size: usize) -> Result<RsaPrivateKey, Error> {
    let mut rng = OsRng;
    let rsa_priv_key = RsaPrivateKey::new(&mut rng, bit_size).map_err(Error::RsaError)?;
    Ok(rsa_priv_key)
}

impl CesealMasterKey {
    pub fn generate() -> Result<Self, Error> {
        let rsa_priv_key = generate_rsa_private_key(2048)?;
        let sr25519_keypair = derive_to_sr25519_key_by(rsa_priv_key.clone())?;
        Ok(Self { rsa_priv_key, sr25519_keypair })
    }

    pub fn from_sealed_file(
        sealing_path: &str,
        identity_key: &sr25519::Pair,
        sealer: &impl Sealing,
    ) -> Result<Self, Error> {
        let rsa_der = persistence::unseal(sealing_path, identity_key, sealer)?;
        Self::from_rsa_der(&rsa_der)
    }

    pub fn from_rsa_der(rsa_der: &RsaDer) -> Result<Self, Error> {
        let rsa_priv_key = RsaPrivateKey::restore_from_der(rsa_der).map_err(|e| Error::CesCrypto(e))?;
        let sr25519_keypair = derive_to_sr25519_key_by(rsa_priv_key.clone())?;
        Ok(Self { rsa_priv_key, sr25519_keypair })
    }

    pub fn seal(&self, sealing_path: &str, identity_key: &sr25519::Pair, sealer: &impl Sealing) {
        let rsa_der = self.rsa_priv_key.dump_secret_der();
        persistence::seal(sealing_path, rsa_der, identity_key, sealer)
    }

    pub fn rsa_private_key(&self) -> &RsaPrivateKey {
        &self.rsa_priv_key
    }

    pub fn rsa_public_key(&self) -> RsaPublicKey {
        RsaPublicKey::from(&self.rsa_priv_key)
    }

    pub fn rsa_public_key_as_pkcs1_der(&self) -> Result<Vec<u8>, Error> {
        use rsa::pkcs1::EncodeRsaPublicKey;
        Ok(self
            .rsa_public_key()
            .to_pkcs1_der()
            .map_err(|e| Error::Pkcs1Encode(e.to_string()))?
            .into_vec())
    }

    pub fn sr25519_keypair(&self) -> &sr25519::Pair {
        &self.sr25519_keypair
    }

    pub fn sr25519_public_key(&self) -> sr25519::Public {
        self.sr25519_keypair.public()
    }

    pub fn dump_rsa_secret_der(&self) -> RsaDer {
        self.rsa_priv_key.dump_secret_der()
    }
}

pub mod persistence {
    use super::Error;
    use crate::pal::Sealing;
    use ces_crypto::{
        rsa::RsaDer,
        sr25519::{Signature, Signing},
    };
    use ces_types::{wrap_content_to_sign, SignedContentType};
    use parity_scale_codec::{Decode, Encode};
    use sp_core::sr25519;
    use std::path::PathBuf;

    /// Master key filepath
    pub const MASTER_KEY_FILE: &str = "master_key.seal";

    #[derive(Debug, Encode, Decode, Clone)]
    struct PersistentMasterKey {
        pub secret: RsaDer,
        pub signature: Signature,
    }

    #[derive(Debug, Encode, Decode)]
    enum MasterKeySeal {
        V1(PersistentMasterKey),
    }

    pub fn master_key_file_path(sealing_path: &str) -> PathBuf {
        PathBuf::from(sealing_path).join(MASTER_KEY_FILE)
    }

    /// Seal master key seeds with signature to ensure integrity
    pub fn seal(sealing_path: &str, rsa_key_der: RsaDer, identity_key: &sr25519::Pair, sealer: &impl Sealing) {
        let encoded = rsa_key_der.encode();
        let wrapped = wrap_content_to_sign(&encoded, SignedContentType::MasterKeyStore);
        let signature = identity_key.sign_data(&wrapped);

        let data = MasterKeySeal::V1(PersistentMasterKey { secret: rsa_key_der, signature });
        let filepath = master_key_file_path(&sealing_path);
        info!("Seal master key to {}", filepath.as_path().display());
        // TODO: seal with identity key so the newly handovered ceseal do not need to do an extra sync to get master
        // key
        sealer.seal_data(filepath, &data.encode()).expect("Seal master key failed");
    }

    pub fn is_master_key_exists_on_local(sealing_path: &str) -> bool {
        master_key_file_path(sealing_path).exists()
    }

    /// Unseal local master key seeds and verify signature
    pub fn unseal(sealing_path: &str, identity_key: &sr25519::Pair, sealer: &impl Sealing) -> Result<RsaDer, Error> {
        let filepath = master_key_file_path(&sealing_path);
        info!("Unseal master key from {}", filepath.as_path().display());
        let Some(sealed_data) = sealer
            .unseal_data(&filepath)
            .map_err(|e| Error::UnsealError(format!("{:?}", e)))?
        else {
            return Err(Error::NoSealedMasterKey)
        };

        let versioned_data =
            MasterKeySeal::decode(&mut &sealed_data[..]).map_err(|e| Error::DecodeMasterKey(e.to_string()))?;
        // #[allow(clippy::infallible_destructuring_match)]
        let secret = match versioned_data {
            MasterKeySeal::V1(data) => {
                let encoded = data.secret.encode();
                let wrapped = wrap_content_to_sign(&encoded, SignedContentType::MasterKeyStore);
                if !identity_key.verify_data(&data.signature, &wrapped) {
                    return Err(Error::MasterKeyBroken)
                }
                data.secret
            },
        };
        Ok(secret)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    struct MockSealer;
    impl Sealing for MockSealer {
        type SealError = std::io::Error;
        type UnsealError = std::io::Error;

        fn seal_data(&self, path: impl AsRef<std::path::Path>, data: &[u8]) -> Result<(), Self::SealError> {
            std::fs::write(path, data)?;
            Ok(())
        }

        fn unseal_data(&self, path: impl AsRef<std::path::Path>) -> Result<Option<Vec<u8>>, Self::UnsealError> {
            match std::fs::read(path) {
                Err(err) if matches!(err.kind(), std::io::ErrorKind::NotFound) => Ok(None),
                other => other.map(Some),
            }
        }
    }
    struct CleanableFile(std::path::PathBuf);
    impl Drop for CleanableFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn test_seal_unseal() {
        let sealer = MockSealer;
        let rsa_priv_key = generate_rsa_private_key(2048).unwrap();
        let rsa_der = rsa_priv_key.dump_secret_der();
        let identity_key = crate::new_sr25519_key();
        let sealing_path = "./";
        let _f = CleanableFile(persistence::master_key_file_path(sealing_path));
        persistence::seal(sealing_path, rsa_der.clone(), &identity_key, &sealer);
        let unsealed_rsa_der = persistence::unseal(sealing_path, &identity_key, &sealer).unwrap();
        assert_eq!(rsa_der, unsealed_rsa_der);
    }
}
