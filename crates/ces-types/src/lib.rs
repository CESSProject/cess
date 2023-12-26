#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

use alloc::{borrow::Cow, string::String, vec::Vec};
use core::fmt::Debug;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

pub mod attestation;

pub mod messaging {
	use super::{EcdhPublicKey, MasterPublicKey, WorkerIdentity, WorkerPublicKey};
	use alloc::{collections::btree_map::BTreeMap, vec::Vec};
	use core::fmt::Debug;
	use parity_scale_codec::{Decode, Encode};
	use scale_info::TypeInfo;

	pub use ces_mq::{bind_topic, types::*};

	// Messages: System
	#[derive(Encode, Decode, TypeInfo)]
	pub struct WorkerEventWithKey {
		pub pubkey: WorkerPublicKey,
		pub event: WorkerEvent,
	}

	impl Debug for WorkerEventWithKey {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			let pubkey = hex::encode(self.pubkey.0);
			f.debug_struct("WorkerEventWithKey")
				.field("pubkey", &pubkey)
				.field("event", &self.event)
				.finish()
		}
	}

	#[derive(Encode, Decode, Debug, TypeInfo)]
	pub struct WorkerInfo {
		pub confidence_level: u8,
	}

	#[derive(Encode, Decode, Debug, TypeInfo)]
	pub enum WorkerEvent {
		/// pallet-registry --> worker
		///  Indicate a worker register succeeded.
		Registered(WorkerInfo),
	}

	bind_topic!(SystemEvent, b"cess/system/event");
	#[derive(Encode, Decode, Debug, TypeInfo)]
	pub enum SystemEvent {
		WorkerEvent(WorkerEventWithKey),
	}

	impl SystemEvent {
		pub fn new_worker_event(pubkey: WorkerPublicKey, event: WorkerEvent) -> SystemEvent {
			SystemEvent::WorkerEvent(WorkerEventWithKey { pubkey, event })
		}
	}

	// Messages: Keyfairy launch
	bind_topic!(KeyfairyLaunch, b"cess/keyfairy/launch");
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub enum KeyfairyLaunch {
		FirstKeyfairy(NewKeyfairyEvent),
		MasterPubkeyOnChain(MasterPubkeyEvent),
		RotateMasterKey(RotateMasterKeyEvent),
		MasterPubkeyRotated(MasterPubkeyEvent),
	}

	impl KeyfairyLaunch {
		pub fn first_keyfairy(pubkey: WorkerPublicKey, ecdh_pubkey: EcdhPublicKey) -> KeyfairyLaunch {
			KeyfairyLaunch::FirstKeyfairy(NewKeyfairyEvent { pubkey, ecdh_pubkey })
		}

		pub fn master_pubkey_on_chain(master_pubkey: MasterPublicKey) -> KeyfairyLaunch {
			KeyfairyLaunch::MasterPubkeyOnChain(MasterPubkeyEvent { master_pubkey })
		}

		pub fn rotate_master_key(rotation_id: u64, gk_identities: Vec<WorkerIdentity>) -> KeyfairyLaunch {
			KeyfairyLaunch::RotateMasterKey(RotateMasterKeyEvent { rotation_id, gk_identities })
		}

		pub fn master_pubkey_rotated(master_pubkey: MasterPublicKey) -> KeyfairyLaunch {
			KeyfairyLaunch::MasterPubkeyRotated(MasterPubkeyEvent { master_pubkey })
		}
	}

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct NewKeyfairyEvent {
		/// The public key of registered keyfairy
		pub pubkey: WorkerPublicKey,
		/// The ecdh public key of registered keyfairy
		pub ecdh_pubkey: EcdhPublicKey,
	}

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct RemoveKeyfairyEvent {
		/// The public key of registered keyfairy
		pub pubkey: WorkerPublicKey,
	}

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct MasterPubkeyEvent {
		pub master_pubkey: MasterPublicKey,
	}

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct RotateMasterKeyEvent {
		pub rotation_id: u64,
		pub gk_identities: Vec<WorkerIdentity>,
	}

	// Messages: Keyfairy change
	bind_topic!(KeyfairyChange, b"cess/keyfairy/change");
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub enum KeyfairyChange {
		Registered(NewKeyfairyEvent),
		Unregistered(RemoveKeyfairyEvent),
	}

	impl KeyfairyChange {
		pub fn keyfairy_registered(pubkey: WorkerPublicKey, ecdh_pubkey: EcdhPublicKey) -> KeyfairyChange {
			KeyfairyChange::Registered(NewKeyfairyEvent { pubkey, ecdh_pubkey })
		}

		pub fn keyfairy_unregistered(pubkey: WorkerPublicKey) -> KeyfairyChange {
			KeyfairyChange::Unregistered(RemoveKeyfairyEvent { pubkey })
		}
	}

	// Messages: Distribution of master key
	bind_topic!(KeyDistribution<BlockNumber>, b"cess/keyfairy/key");
	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub enum KeyDistribution<BlockNumber> {
		/// Legacy single master key sharing, use `MasterKeyHistory` after we enable master key
		/// rotation
		///
		/// MessageOrigin::Keyfairy -> MessageOrigin::Worker
		MasterKeyDistribution(DispatchMasterKeyEvent),
		// TODO: a better way for real large batch key distribution
		/// MessageOrigin::Worker -> ALL
		///
		/// The origin cannot be Keyfairy, else the leakage of old master key will further leak
		/// the following keys
		MasterKeyRotation(BatchRotateMasterKeyEvent),
		/// MessageOrigin::Keyfairy -> MessageOrigin::Worker
		MasterKeyHistory(DispatchMasterKeyHistoryEvent<BlockNumber>),
	}

	impl<BlockNumber> KeyDistribution<BlockNumber> {
		pub fn master_key_distribution(
			dest: WorkerPublicKey,
			ecdh_pubkey: EcdhPublicKey,
			encrypted_master_key: Vec<u8>,
			iv: AeadIV,
		) -> KeyDistribution<BlockNumber> {
			KeyDistribution::MasterKeyDistribution(DispatchMasterKeyEvent {
				dest,
				ecdh_pubkey,
				encrypted_master_key,
				iv,
			})
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

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct DispatchMasterKeyHistoryEvent<BlockNumber> {
		/// The target to dispatch master key
		pub dest: WorkerPublicKey,
		pub encrypted_master_key_history: Vec<(u64, BlockNumber, EncryptedKey)>,
	}

	#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
	pub struct BatchRotateMasterKeyEvent {
		pub rotation_id: u64,
		pub secret_keys: BTreeMap<WorkerPublicKey, EncryptedKey>,
		pub sender: WorkerPublicKey,
		pub sig: Vec<u8>,
	}

	#[derive(Encode)]
	pub(crate) struct BatchRotateMasterKeyData<'a> {
		pub(crate) rotation_id: u64,
		pub(crate) secret_keys: &'a BTreeMap<WorkerPublicKey, EncryptedKey>,
		pub(crate) sender: WorkerPublicKey,
	}

	impl BatchRotateMasterKeyEvent {
		pub fn data_be_signed(&self) -> Vec<u8> {
			BatchRotateMasterKeyData {
				rotation_id: self.rotation_id,
				secret_keys: &self.secret_keys,
				sender: self.sender,
			}
			.encode()
		}
	}
}

// Types used in storage
pub use attestation::{AttestationReport, AttestationProvider};

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
	pub genesis_block_hash: H256,
	pub features: Vec<u32>,
	pub operator: Option<AccountId>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub enum VersionedWorkerEndpoints {
	V1(Vec<String>),
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq, TypeInfo)]
pub struct WorkerEndpointPayload {
	pub pubkey: WorkerPublicKey,
	pub versioned_endpoints: VersionedWorkerEndpoints,
	pub signing_time: u64,
}

#[repr(u8)]
pub enum SignedContentType {
	MqMessage = 0,
	RpcResponse = 1,
	EndpointInfo = 2,
	MasterKeyRotation = 3,
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
