#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::{fmt::Debug, str::FromStr};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

pub mod attestation;

// Types used in storage
pub use attestation::{AttestationProvider, AttestationReport};

pub type TeeSig = sp_core::sr25519::Signature;

type MachineId = Vec<u8>;
pub use sp_core::sr25519::{
	Public as WorkerPublicKey, Public as MasterPublicKey, Public as EcdhPublicKey, Signature as Sr25519Signature,
};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerIdentity {
	pub pubkey: WorkerPublicKey,
	pub ecdh_pubkey: EcdhPublicKey,
}

/// One-time Challenge for WorkerKey handover
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct HandoverChallenge<BlockNumber> {
	pub sgx_target_info: Vec<u8>,
	// The challenge is only considered valid within 150 blocks (~30 min)
	pub block_number: BlockNumber,
	pub now: u64,
	pub dev_mode: bool,
	pub nonce: [u8; 32],
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct ChallengeHandlerInfo<BlockNumber> {
	pub challenge: HandoverChallenge<BlockNumber>,
	pub sgx_local_report: Vec<u8>,
	pub ecdh_pubkey: EcdhPublicKey,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct EncryptedWorkerKey {
	pub genesis_block_hash: H256,
	pub dev_mode: bool,
	pub encrypted_key: EncryptedKey,
}

pub type AeadIV = [u8; 12];

/// Secret key encrypted with AES-256-GCM algorithm
///
/// The encryption key is generated with sr25519-based ECDH
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
pub struct EncryptedKey {
	/// The ecdh public key of key source
	pub ecdh_pubkey: EcdhPublicKey,
	/// Key encrypted with aead key
	pub encrypted_key: Vec<u8>,
	/// Aead IV
	pub iv: AeadIV,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerRegistrationInfo<AccountId> {
	pub version: u32,
	pub machine_id: MachineId,
	pub pubkey: WorkerPublicKey,
	pub ecdh_pubkey: EcdhPublicKey,
	pub stash_account: Option<AccountId>,
	pub genesis_block_hash: H256,
	pub features: Vec<u32>,
	pub role: WorkerRole,
	pub endpoint: String,
}

#[repr(u8)]
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub enum WorkerRole {
	Full,
	Verifier,
	Marker,
}

impl Default for WorkerRole {
	fn default() -> Self {
		WorkerRole::Full
	}
}

impl FromStr for WorkerRole {
	type Err = String;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.to_lowercase().as_str() {
			"full" => Ok(WorkerRole::Full),
			"verifier" => Ok(WorkerRole::Verifier),
			"marker" => Ok(WorkerRole::Marker),
			_ => Err(String::from("invalid worker-role value")),
		}
	}
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct MasterKeyApplyPayload {
	pub pubkey: WorkerPublicKey,
	pub ecdh_pubkey: EcdhPublicKey,
	pub signing_time: u64,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct MasterKeyDistributePayload {
	/// The source of master key for sharing
	pub distributor: WorkerPublicKey,
	/// The target to distribute
	pub target: WorkerPublicKey,
	/// The ecdh public key of master key source
	pub ecdh_pubkey: EcdhPublicKey,
	/// Master key encrypted with aead key
	pub encrypted_master_key: Vec<u8>,
	/// Aead IV
	pub iv: AeadIV,
	pub signing_time: u64,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct MasterKeyLaunchPayload {
	pub launcher: WorkerPublicKey,
	pub master_pubkey: MasterPublicKey,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemoryUsage {
	/// The current heap usage of Rust codes.
	pub rust_used: u64,
	/// The peak heap usage of Rust codes.
	pub rust_peak_used: u64,
	/// The entire peak heap memory usage of the enclave.
	pub total_peak_used: u64,
	/// The memory left.
	pub free: u64,
	/// The peak heap usage of Rust codes in a recent short-term.
	pub rust_spike: u64,
}

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum ChainNetwork {
	Dev,
	Devnet,
	Testnet,
}

impl Default for ChainNetwork {
	fn default() -> Self {
		ChainNetwork::Dev
	}
}

impl FromStr for ChainNetwork {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"testnet" => Ok(ChainNetwork::Testnet),
			"devnet" => Ok(ChainNetwork::Devnet),
			"dev" => Ok(ChainNetwork::Dev),
			_ => Err(String::from("Unsupported chain network")),
		}
	}
}
