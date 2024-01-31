use crate::CryptoError;
use rsa::{
    Pkcs1v15Sign, pkcs8::{DecodePrivateKey,EncodePrivateKey}, PublicKey,
};
use alloc::vec::Vec;

pub const SIGNATURE_BYTES: usize = 256;
pub const RSA_BIT_SIZE: usize = 2048;

pub type RsaDer = Vec<u8>;
pub type Signature = [u8; SIGNATURE_BYTES];

pub trait Signing {
    fn sign_data(&self, data: &[u8]) -> Result<Signature,CryptoError>;

    fn verify_data(&self, sig: &Signature, data: &[u8]) -> bool;
}

pub trait Persistence {
    fn dump_seed(&self) -> RsaDer;

    fn dump_secret_der(&self) -> RsaDer;

    fn restore_from_der(seed: &RsaDer) -> Result<rsa::RsaPrivateKey,CryptoError>;
}

impl Signing for rsa::RsaPrivateKey {
    fn sign_data(&self, data: &[u8]) -> Result<Signature,CryptoError> {
        self.sign(Pkcs1v15Sign::new_raw(), data).map_err(|_|CryptoError::RsaSigningError)?.try_into().map_err(|_|CryptoError::RsaSigningError)
    }
    
    fn verify_data(&self, sig: &Signature, hashed: &[u8]) -> bool {
        self.verify(Pkcs1v15Sign::new_raw(), hashed, sig).is_ok()
    }
}

impl Persistence for rsa::RsaPrivateKey {
    fn dump_seed(&self) -> RsaDer {
        panic!("No available seed for rsa pair");
    }

    fn dump_secret_der(&self) -> RsaDer {
        self.to_pkcs8_der().unwrap().as_bytes().to_vec()
    }

    fn restore_from_der(seed: &RsaDer) -> Result<rsa::RsaPrivateKey,CryptoError> {
        rsa::RsaPrivateKey::from_pkcs8_der(&seed).map_err(|_|CryptoError::RsaInvalidDer)
    }
}