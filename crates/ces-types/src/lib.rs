#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{borrow::Cow, string::String, vec::Vec};
use core::fmt::Debug;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

pub mod attestation;

pub mod messaging {
	use super::{EcdhPublicKey, MasterPublicKey, WorkerPublicKey};
	use alloc::vec::Vec;
	use core::fmt::Debug;
	use parity_scale_codec::{Decode, Encode};
	use scale_info::TypeInfo;

	pub use ces_mq::{bind_topic, types::*};

	bind_topic!(WorkerEvent, b"cess/teeworker/event");
	#[derive(Encode, Decode, Debug, TypeInfo)]
	pub enum WorkerEvent {
		/// pallet-tee-worker --> worker
		/// Indicate a worker register succeeded.
		Registered(WorkerPublicKey),
	}

	impl WorkerEvent {
		pub fn new_worker(worker_pubkey: WorkerPublicKey) -> WorkerEvent {
			WorkerEvent::Registered(worker_pubkey)
		}
	}

	// Messages: MasterKey launch
	bind_topic!(MasterKeyLaunch, b"cess/masterkey/launch");
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub enum MasterKeyLaunch {
		LaunchRequest(WorkerPublicKey, EcdhPublicKey),
		OnChainLaunched(MasterPublicKey),
	}

	impl MasterKeyLaunch {
		pub fn launch_request(pubkey: WorkerPublicKey, ecdh_pubkey: EcdhPublicKey) -> MasterKeyLaunch {
			MasterKeyLaunch::LaunchRequest(pubkey, ecdh_pubkey)
		}

		pub fn on_chain_launched(master_pubkey: MasterPublicKey) -> MasterKeyLaunch {
			MasterKeyLaunch::OnChainLaunched(master_pubkey)
		}
	}

	// Messages: Distribution of master key
	bind_topic!(MasterKeyDistribution, b"cess/masterkey/dist");
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub enum MasterKeyDistribution {
		/// Master key sharing
		///
		/// MessageOrigin::Keyfairy -> MessageOrigin::Worker
		Distribution(DispatchMasterKeyEvent),
	}

	impl MasterKeyDistribution {
		pub fn distribution(
			dest: WorkerPublicKey,
			ecdh_pubkey: EcdhPublicKey,
			encrypted_master_key: Vec<u8>,
			iv: AeadIV,
		) -> MasterKeyDistribution {
			MasterKeyDistribution::Distribution(DispatchMasterKeyEvent { dest, ecdh_pubkey, encrypted_master_key, iv })
		}
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

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct DispatchMasterKeyEvent {
		/// The target to dispatch master key
		pub dest: WorkerPublicKey,
		/// The ecdh public key of master key source
		pub ecdh_pubkey: EcdhPublicKey,
		/// Master key encrypted with aead key
		pub encrypted_master_key: Vec<u8>,
		/// Aead IV
		pub iv: AeadIV,
	}

	bind_topic!(MasterKeyApply, b"cess/masterkey/apply");
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub enum MasterKeyApply {
		Apply(WorkerPublicKey, EcdhPublicKey),
	}
}

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
	pub encrypted_key: messaging::EncryptedKey,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerRegistrationInfo<AccountId> {
	pub version: u32,
	pub machine_id: MachineId,
	pub pubkey: WorkerPublicKey,
	pub ecdh_pubkey: EcdhPublicKey,
	pub operator: Option<AccountId>,
	pub genesis_block_hash: H256,
	pub features: Vec<u32>,
	pub role: WorkerRole,
}

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

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerEndpointPayload {
	pub pubkey: WorkerPublicKey,
	pub endpoint: Option<String>,
	pub signing_time: u64,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct MasterKeyApplyPayload {
	pub pubkey: WorkerPublicKey,
	pub ecdh_pubkey: EcdhPublicKey,
	pub signing_time: u64,
}

#[repr(u8)]
pub enum SignedContentType {
	MqMessage = 0,
	RpcResponse = 1,
	EndpointInfo = 2,
	MasterKeyApply = 3,
	MasterKeyStore = 4,
}

pub fn wrap_content_to_sign(data: &[u8], sigtype: SignedContentType) -> Cow<[u8]> {
	match sigtype {
		// We don't wrap mq messages for backward compatibility.
		SignedContentType::MqMessage => data.into(),
		_ => {
			let mut wrapped: Vec<u8> = Vec::new();
			// MessageOrigin::Reserved.encode() == 0xff
			wrapped.push(0xff);
			wrapped.push(sigtype as u8);
			wrapped.extend_from_slice(data);
			wrapped.into()
		},
	}
}
