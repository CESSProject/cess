#![no_std]
#![forbid(unsafe_code)]
#![forbid(missing_docs)]
#![allow(clippy::result_unit_err)]

//! Verify BLS signatures
//!
//! This verifies BLS signatures in a manner which is compatible with
//! the Internet Computer.

use core::fmt;
use core::ops::Neg;

use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, Scalar};
use pairing::group::{Curve, Group};
// use rand::RngCore;

lazy_static::lazy_static! {
    static ref G2PREPARED_NEG_G : G2Prepared = G2Affine::generator().neg().into();
}

const BLS_SIGNATURE_DOMAIN_SEP: [u8; 43] = *b"BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_";

fn hash_to_g1(msg: &[u8]) -> G1Affine {
    <G1Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(
        msg,
        &BLS_SIGNATURE_DOMAIN_SEP,
    )
    .to_affine()
}

/// A BLS12-381 public key usable for signature verification
#[derive(Clone, Eq, PartialEq)]
pub struct PublicKey {
    pk: G2Affine,
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PublicKey({})", hex::encode(self.serialize()))
    }
}

/// An error indicating an encoded public key was not valid
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidPublicKey {
    /// The public key encoding was not the correct length
    WrongLength,
    /// The public key was not a valid BLS12-381 G2 point
    InvalidPoint,
}

impl PublicKey {
    /// The length of the binary encoding of this type
    pub const BYTES: usize = 96;

    fn new(pk: G2Affine) -> Self {
        Self { pk }
    }

    /// Return the serialization of this BLS12-381 public key
    pub fn serialize(&self) -> [u8; Self::BYTES] {
        self.pk.to_compressed()
    }

    /// Deserialize a BLS12-381 public key
    pub fn deserialize(bytes: &[u8]) -> Result<Self, InvalidPublicKey> {
        let bytes: Result<[u8; Self::BYTES], _> = bytes.try_into();

        match bytes {
            Err(_) => Err(InvalidPublicKey::WrongLength),
            Ok(b) => {
                let pk = G2Affine::from_compressed(&b);
                if bool::from(pk.is_some()) {
                    Ok(Self::new(pk.unwrap()))
                } else {
                    Err(InvalidPublicKey::InvalidPoint)
                }
            }
        }
    }

    /// Verify a BLS signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), ()> {
        let msg = hash_to_g1(message);
        let g2_gen: &G2Prepared = &G2PREPARED_NEG_G;
        let pk = G2Prepared::from(self.pk);

        let sig_g2 = (&signature.sig, g2_gen);
        let msg_pk = (&msg, &pk);

        let x = multi_miller_loop(&[sig_g2, msg_pk]).final_exponentiation();

        if bool::from(x.is_identity()) {
            Ok(())
        } else {
            Err(())
        }
    }
}

/// An error indicating a signature could not be deserialized
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidSignature {
    /// The signature encoding is the wrong length
    WrongLength,
    /// The signature encoding is not a valid BLS12-381 G1 point
    InvalidPoint,
}

/// A type expressing a BLS12-381 signature
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Signature {
    sig: G1Affine,
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", hex::encode(self.serialize()))
    }
}

impl Signature {
    /// The length of the binary encoding of this type
    pub const BYTES: usize = 48;

    fn new(sig: G1Affine) -> Self {
        Self { sig }
    }

    /// Serialize the BLS signature
    pub fn serialize(&self) -> [u8; Self::BYTES] {
        self.sig.to_compressed()
    }

    /// Deserialize a BLS signature
    pub fn deserialize(bytes: &[u8]) -> Result<Self, InvalidSignature> {
        let bytes: Result<[u8; Self::BYTES], _> = bytes.try_into();

        match bytes {
            Err(_) => Err(InvalidSignature::WrongLength),
            Ok(b) => {
                let sig = G1Affine::from_compressed(&b);
                if bool::from(sig.is_some()) {
                    Ok(Self::new(sig.unwrap()))
                } else {
                    Err(InvalidSignature::InvalidPoint)
                }
            }
        }
    }
}

/// An error indicating a private key could not be deserialized
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum InvalidPrivateKey {
    /// The secret key encoding was the wrong length and not possibly valid
    WrongLength,
    /// The secret key was outside the valid range of BLS12-381 keys
    OutOfRange,
}

/// A type expressing a BLS12-381 private key
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct PrivateKey {
    sk: Scalar,
}

impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateKey(REDACTED)")
    }
}

impl PrivateKey {
    /// The length of the serialized encoding of this type
    pub const BYTES: usize = 32;

    fn new(sk: Scalar) -> Self {
        Self { sk }
    }

    /// Create a new random secret key
    pub fn random() -> Self {
        // let mut rng = rand::thread_rng();
        loop {
            let mut buf = [0u8; 32];
            // rng.fill_bytes(&mut buf);
            // note: Handle the `Result` below
            getrandom::getrandom(&mut buf);
            let s: Option<Scalar> = Scalar::from_bytes(&buf).into();

            if let Some(s) = s {
                return Self::new(s);
            }
        }
    }

    /// Serialize a private key to the binary big-ending encoding
    pub fn serialize(&self) -> [u8; Self::BYTES] {
        let mut bytes = self.sk.to_bytes();
        bytes.reverse();
        bytes
    }

    /// Deserialize a secret key
    pub fn deserialize(bytes: &[u8]) -> Result<Self, InvalidPrivateKey> {
        let bytes: Result<[u8; Self::BYTES], _> = bytes.try_into();

        match bytes {
            Err(_) => Err(InvalidPrivateKey::WrongLength),
            Ok(mut b) => {
                b.reverse();
                let sk = Scalar::from_bytes(&b);
                if bool::from(sk.is_some()) {
                    Ok(Self::new(sk.unwrap()))
                } else {
                    Err(InvalidPrivateKey::OutOfRange)
                }
            }
        }
    }

    /// Return the public key associated with this secret key
    pub fn public_key(&self) -> PublicKey {
        PublicKey::new(G2Affine::from(G2Affine::generator() * self.sk))
    }

    /// Sign a message using this private key
    ///
    /// The message can be of arbitrary length
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = hash_to_g1(message) * self.sk;
        Signature::new(sig.to_affine())
    }
}

/// Verify a BLS signature
///
/// The signature must be exactly 48 bytes (compressed G1 element)
/// The key must be exactly 96 bytes (compressed G2 element)
pub fn verify_bls_signature(sig: &[u8], msg: &[u8], key: &[u8]) -> Result<(), ()> {
    let sig = Signature::deserialize(sig).map_err(|_| ())?;
    let pk = PublicKey::deserialize(key).map_err(|_| ())?;
    pk.verify(msg, &sig)
}
