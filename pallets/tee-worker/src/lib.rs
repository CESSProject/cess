//! # Tee Worker Module
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

mod mock;
mod types;
pub use types::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult, pallet_prelude::*, traits::ReservableCurrency, BoundedVec, PalletId,
	traits::{Get, StorageVersion, UnixTime},
};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, RuntimeDebug, SaturatedConversion};
use sp_std::{convert::TryInto, prelude::*};

use cp_cess_common::*;
use cp_scheduler_credit::SchedulerCreditCounter;
use ces_types::{WorkerPublicKey, MasterPublicKey, WorkerRole};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use weights::WeightInfo;
pub mod weights;

mod functions;

extern crate alloc;

#[cfg(feature = "native")]
use sp_core::hashing;

#[cfg(not(feature = "native"))]
use sp_io::hashing;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::{Decode, Encode};
	use frame_support::{
		dispatch::DispatchResult,
	};
	use scale_info::TypeInfo;


	use ces_pallet_mq::MessageOriginInfo;
	use ces_types::{
		attestation::{self, Error as AttestationError},
		messaging::{
			self, bind_topic, DecodedMessage, KeyfairyChange, KeyfairyLaunch, MessageOrigin, SystemEvent, WorkerEvent,
		},
		wrap_content_to_sign, AttestationProvider, EcdhPublicKey,  SignedContentType,
		WorkerAction, WorkerIdentity, WorkerRegistrationInfo,
	};

	// Re-export
	pub use ces_types::AttestationReport;
	// TODO: Legacy
	pub use ces_types::attestation::legacy::{Attestation, AttestationValidator, IasFields, IasValidator};

	bind_topic!(RegistryEvent, b"^cess/registry/event");
	#[derive(Encode, Decode, TypeInfo, Clone, Debug)]
	pub enum RegistryEvent {
		///	MessageOrigin::Worker -> Pallet
		///
		/// Only used for first master pubkey upload, the origin has to be worker identity since there is no master
		/// pubkey on-chain yet.
		MasterPubkey { master_pubkey: MasterPublicKey },
	}

	bind_topic!(KeyfairyRegistryEvent, b"^cess/registry/kf_event");
	#[derive(Encode, Decode, TypeInfo, Clone, Debug)]
	pub enum KeyfairyRegistryEvent {
		RotatedMasterPubkey { rotation_id: u64, master_pubkey: MasterPublicKey },
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_cess_staking::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// pallet address.
		#[pallet::constant]
		type TeeWorkerPalletId: Get<PalletId>;

		#[pallet::constant]
		type SchedulerMaximum: Get<u32> + PartialEq + Eq + Clone;
		//the weights
		type WeightInfo: WeightInfo;

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;

		#[pallet::constant]
		type MaxWhitelist: Get<u32> + Clone + Eq + PartialEq;

		type LegacyAttestationValidator: AttestationValidator;

		/// Enable None Attestation, SHOULD BE SET TO FALSE ON PRODUCTION !!!
		#[pallet::constant]
		type NoneAttestationEnabled: Get<bool>;

		/// Verify attestation
		///
		/// SHOULD NOT SET TO FALSE ON PRODUCTION!!!
		#[pallet::constant]
		type VerifyCeseal: Get<bool>;

		/// Origin used to govern the pallet
		type GovernanceOrigin: EnsureOrigin<Self::RuntimeOrigin>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Exit {
			tee: WorkerPublicKey,
		},

		MasterKeyLaunched,
		/// A new Keyfairy is enabled on the blockchain
		KeyfairyAdded {
			pubkey: WorkerPublicKey,
		},
		KeyfairyRemoved {
			pubkey: WorkerPublicKey,
		},
		WorkerAdded {
			pubkey: WorkerPublicKey,
			attestation_provider: Option<AttestationProvider>,
			confidence_level: u8,
		},
		WorkerUpdated {
			pubkey: WorkerPublicKey,
			attestation_provider: Option<AttestationProvider>,
			confidence_level: u8,
		},
		MasterKeyRotated {
			rotation_id: u64,
			master_pubkey: MasterPublicKey,
		},
		MasterKeyRotationFailed {
			rotation_lock: Option<u64>,
			keyfairy_rotation_id: u64,
		},
		MinimumCesealVersionChangedTo(u32, u32, u32),
	}

	#[pallet::error]
	pub enum Error<T> {
		//Already registered
		AlreadyRegistration,
		//Not a controller account
		NotStash,
		//The scheduled error report has been reported once
		AlreadyReport,
		//Boundedvec conversion error
		BoundedVecError,
		//Storage reaches upper limit error
		StorageLimitReached,
		//data overrun error
		Overflow,

		NotBond,

		NotController,

		NonTeeWorker,

		VerifyCertFailed,

		TeePodr2PkNotInitialized,

		Existed,

		CesealRejected,

		InvalidIASSigningCert,

		InvalidReport,

		InvalidQuoteStatus,

		BadIASReport,

		OutdatedIASReport,

		UnknownQuoteBodyFormat,

		InvalidCesealInfoHash,

		NoneAttestationDisabled,

		WrongTeeType,

		InvalidSender,
		InvalidPubKey,
		MalformedSignature,
		InvalidSignatureLength,
		InvalidSignature,
		// IAS related
		WorkerNotFound,
		// Keyfairy related
		InvalidKeyfairy,
		InvalidMasterPubkey,
		MasterKeyMismatch,
		MasterKeyUninitialized,
		// Ceseal related
		CesealAlreadyExists,
		CesealNotFound,
		// Additional
		NotImplemented,
		CannotRemoveLastKeyfairy,
		MasterKeyInRotation,
		InvalidRotatedMasterPubkey,
		// Endpoint related
		EmptyEndpoint,
		InvalidEndpointSigningTime,

		PayloadError,

		LastWorker,
	}

	#[pallet::storage]
	#[pallet::getter(fn validation_type_list)]
	pub(super) type ValidationTypeList<T: Config> =
		StorageValue<_, BoundedVec<WorkerPublicKey, T::SchedulerMaximum>, ValueQuery>;

	/// Keyfairy pubkey list
	#[pallet::storage]
	pub type Keyfairies<T: Config> = StorageValue<_, Vec<WorkerPublicKey>, ValueQuery>;

	/// Master public key
	#[pallet::storage]
	pub type MasterPubkey<T: Config> = StorageValue<_, MasterPublicKey>;

	/// The block number and unix timestamp when the keyfairy is launched
	#[pallet::storage]
	pub type KeyfairyLaunchedAt<T: Config> = StorageValue<_, (BlockNumberFor<T>, u64)>;

	/// The rotation counter starting from 1, it always equals to the latest rotation id.
	/// The totation id 0 is reserved for the first master key before we introduce the rotation.
	#[pallet::storage]
	pub type RotationCounter<T> = StorageValue<_, u64, ValueQuery>;

	/// Current rotation info including rotation id
	///
	/// Only one rotation process is allowed at one time.
	/// Since the rotation request is broadcasted to all keyfairys, it should be finished only if there is one
	/// functional keyfairy.
	#[pallet::storage]
	pub type MasterKeyRotationLock<T: Config> = StorageValue<_, Option<u64>, ValueQuery>;

	/// Mapping from worker pubkey to WorkerInfo
	#[pallet::storage]
	pub type Workers<T: Config> = CountedStorageMap<_, Twox64Concat, WorkerPublicKey, WorkerInfo<T::AccountId>>;

	/// The first time registered block number for each worker.
	#[pallet::storage]
	pub type WorkerAddedAt<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, BlockNumberFor<T>>;

	/// Allow list of ceseal binary digest
	///
	/// Only ceseal within the list can register.
	#[pallet::storage]
	#[pallet::getter(fn ceseal_bin_allowlist)]
	pub type CesealBinAllowList<T: Config> = StorageValue<_, Vec<Vec<u8>>, ValueQuery>;

	/// The effective height of ceseal binary
	#[pallet::storage]
	pub type CesealBinAddedAt<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, BlockNumberFor<T>>;

	/// Mapping from worker pubkey to CESS Network identity
	#[pallet::storage]
	pub type Endpoints<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, alloc::string::String>;

	/// Ceseals whoes version less than MinimumCesealVersion would be forced to quit.
	#[pallet::storage]
	pub type MinimumCesealVersion<T: Config> = StorageValue<_, (u32, u32, u32), ValueQuery>;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T: ces_pallet_mq::Config,
	{
		/// Update the TEE Worker MR Enclave Whitelist
		///
		/// This function allows the root or superuser to update the whitelist of Trusted Execution Environment (TEE)
		/// Worker MR (Measurement and Report) Enclaves. Each MR Enclave represents a specific instance of a TEE worker.
		/// By adding an MR Enclave to the whitelist, the user ensures that the associated TEE worker can participate in
		/// network activities.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, representing the user's account. Only the root
		///   user is authorized to call this function.
		/// - `mr_enclave`: A fixed-size array of 64 bytes representing the MR Enclave of the TEE worker to be added to
		///   the whitelist.

		/// Exit a TEE Worker from the Network
		///
		/// This function allows a TEE (Trusted Execution Environment) Worker to voluntarily exit from the network.
		/// When a TEE Worker exits, it will no longer participate in network activities and will be removed from the
		/// list of active TEE Workers.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, representing the account of the TEE Worker. This
		///   should be the controller account of the TEE Worker.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::zero())]
		pub fn exit(
			origin: OriginFor<T>, 
			payload: WorkerAction, 
			sig: BoundedVec<u8, ConstU32<64>>
		) -> DispatchResult {
			ensure_signed(origin)?;

			if let WorkerAction::Exit(payload) = payload {
				ensure!(sig.len() == 64, Error::<T>::InvalidSignatureLength);
				let sig =
					sp_core::sr25519::Signature::try_from(sig.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
				let encoded_data = payload.encode();
				let data_to_sign = wrap_content_to_sign(&encoded_data, SignedContentType::EndpointInfo);
				ensure!(
					sp_io::crypto::sr25519_verify(&sig, &data_to_sign, &payload.pubkey),
					Error::<T>::InvalidSignature
				);

				ensure!(<Workers<T>>::count() > 1, Error::<T>::LastWorker);
				ensure!(<Workers<T>>::contains_key(&payload.pubkey), Error::<T>::WorkerNotFound);
				
				Workers::<T>::remove(&payload.pubkey);
				WorkerAddedAt::<T>::remove(&payload.pubkey);
				Endpoints::<T>::remove(&payload.pubkey);

				let mut keyfairys = Keyfairies::<T>::get();
				ensure!(keyfairys.len() > 1, Error::<T>::CannotRemoveLastKeyfairy);
				keyfairys.retain(|g| *g != payload.pubkey);
				Keyfairies::<T>::put(keyfairys);

				ensure!(
					Self::check_time_unix(&payload.signing_time),
					Error::<T>::InvalidEndpointSigningTime
				);
	
				Self::deposit_event(Event::<T>::Exit { tee: payload.pubkey });
			} else {
				return Err(Error::<T>::PayloadError)?
			}
			

			Ok(())
		}

		/// Force register a worker with the given pubkey with sudo permission
		///
		/// For test only.
		#[pallet::call_index(11)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn force_register_worker(
			origin: OriginFor<T>,
			pubkey: WorkerPublicKey,
			ecdh_pubkey: EcdhPublicKey,
			stash_account: Option<AccountOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let worker_info = WorkerInfo {
				pubkey,
				ecdh_pubkey,
				version: 0,
				last_updated: 1,
				stash_account: stash_account,
				attestation_provider: Some(AttestationProvider::Root),
				confidence_level: 128u8,
				features: vec![1, 4],
				role: WorkerRole::Full,
			};
			Workers::<T>::insert(worker_info.pubkey, &worker_info);
			WorkerAddedAt::<T>::insert(worker_info.pubkey, frame_system::Pallet::<T>::block_number());
			Self::push_message(SystemEvent::new_worker_event(
				pubkey,
				WorkerEvent::Registered(messaging::WorkerInfo { confidence_level: worker_info.confidence_level }),
			));
			Self::deposit_event(Event::<T>::WorkerAdded {
				pubkey,
				attestation_provider: Some(AttestationProvider::Root),
				confidence_level: worker_info.confidence_level,
			});

			Ok(())
		}

		/// Register a keyfairy.
		///
		/// Can only be called by `GovernanceOrigin`.
		#[pallet::call_index(13)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn register_keyfairy(origin: OriginFor<T>, keyfairy: WorkerPublicKey) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// disable keyfairy change during key rotation
			let rotating = MasterKeyRotationLock::<T>::get();
			ensure!(rotating.is_none(), Error::<T>::MasterKeyInRotation);

			let mut keyfairys = Keyfairies::<T>::get();
			// wait for the lead keyfairy to upload the master pubkey
			ensure!(keyfairys.is_empty() || MasterPubkey::<T>::get().is_some(), Error::<T>::MasterKeyUninitialized);

			if !keyfairys.contains(&keyfairy) {
				let worker_info = Workers::<T>::get(keyfairy).ok_or(Error::<T>::WorkerNotFound)?;
				keyfairys.push(keyfairy);
				let keyfairy_count = keyfairys.len() as u32;
				Keyfairies::<T>::put(keyfairys);

				if keyfairy_count == 1 {
					Self::push_message(KeyfairyLaunch::first_keyfairy(keyfairy, worker_info.ecdh_pubkey));
				} else {
					Self::push_message(KeyfairyChange::keyfairy_registered(keyfairy, worker_info.ecdh_pubkey));
				}
			}

			Self::deposit_event(Event::<T>::KeyfairyAdded { pubkey: keyfairy });
			Ok(())
		}

		/// Unregister a keyfairy
		///
		/// At least one keyfairy should be available
		#[pallet::call_index(14)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn unregister_keyfairy(origin: OriginFor<T>, keyfairy: WorkerPublicKey) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			// disable keyfairy change during key rotation
			let rotating = MasterKeyRotationLock::<T>::get();
			ensure!(rotating.is_none(), Error::<T>::MasterKeyInRotation);

			let mut keyfairys = Keyfairies::<T>::get();
			ensure!(keyfairys.contains(&keyfairy), Error::<T>::InvalidKeyfairy);
			ensure!(keyfairys.len() > 1, Error::<T>::CannotRemoveLastKeyfairy);

			keyfairys.retain(|g| *g != keyfairy);
			Keyfairies::<T>::put(keyfairys);
			Self::push_message(KeyfairyChange::keyfairy_unregistered(keyfairy));
			Ok(())
		}

		/// Rotate the master key
		#[pallet::call_index(15)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn rotate_master_key(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let rotating = MasterKeyRotationLock::<T>::get();
			ensure!(rotating.is_none(), Error::<T>::MasterKeyInRotation);

			let keyfairys = Keyfairies::<T>::get();
			let gk_identities = keyfairys
				.iter()
				.map(|gk| {
					let worker_info = Workers::<T>::get(gk).ok_or(Error::<T>::WorkerNotFound)?;
					Ok(WorkerIdentity { pubkey: worker_info.pubkey, ecdh_pubkey: worker_info.ecdh_pubkey })
				})
				.collect::<Result<Vec<WorkerIdentity>, Error<T>>>()?;

			let rotation_id = RotationCounter::<T>::mutate(|counter| {
				*counter += 1;
				*counter
			});

			MasterKeyRotationLock::<T>::put(Some(rotation_id));
			Self::push_message(KeyfairyLaunch::rotate_master_key(rotation_id, gk_identities));
			Ok(())
		}

		/// Registers a worker on the blockchain
		/// This is the legacy version that support EPID attestation type only.
		///
		/// Usually called by a bridging relayer program (`cifrost`). Can be called by
		/// anyone on behalf of a worker.
		#[pallet::call_index(16)]
		#[pallet::weight({0})]
		pub fn register_worker(
			origin: OriginFor<T>,
			ceseal_info: WorkerRegistrationInfo<T::AccountId>,
			attestation: Attestation,
		) -> DispatchResult {
			ensure_signed(origin)?;
			// Validate RA report & embedded user data
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			let runtime_info_hash = crate::hashing::blake2_256(&Encode::encode(&ceseal_info));
			let fields = T::LegacyAttestationValidator::validate(
				&attestation,
				&runtime_info_hash,
				now,
				T::VerifyCeseal::get(),
				CesealBinAllowList::<T>::get(),
			)
			.map_err(Into::<Error<T>>::into)?;

			// Update the registry
			let pubkey = ceseal_info.pubkey;

			ensure!(!Workers::<T>::contains_key(&pubkey), Error::<T>::CesealAlreadyExists);
			match &ceseal_info.operator {
				Some(acc) => {
					let _ = <pallet_cess_staking::Pallet<T>>::bonded(acc).ok_or(Error::<T>::NotBond)?;
					ensure!(ceseal_info.role == WorkerRole::Verifier || ceseal_info.role == WorkerRole::Full, Error::<T>::WrongTeeType);
				},
				None => ensure!(ceseal_info.role == WorkerRole::Marker, Error::<T>::WrongTeeType),
			};

			let worker_info = WorkerInfo {
				pubkey,
				ecdh_pubkey: ceseal_info.ecdh_pubkey,
				version: ceseal_info.version,
				last_updated: now,
				stash_account: ceseal_info.operator,
				attestation_provider: Some(AttestationProvider::Ias),
				confidence_level: fields.confidence_level,
				features: ceseal_info.features,
				role: ceseal_info.role.clone(),
			};

			Workers::<T>::insert(&pubkey, worker_info);

			if ceseal_info.role == WorkerRole::Full || ceseal_info.role == WorkerRole::Verifier {
				ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
					puk_list
						.try_push(pubkey)
						.map_err(|_| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			}

			Self::push_message(SystemEvent::new_worker_event(
				pubkey,
				WorkerEvent::Registered(messaging::WorkerInfo {
					confidence_level: fields.confidence_level,
				}),
			));
			Self::deposit_event(Event::<T>::WorkerAdded {
				pubkey,
				attestation_provider: Some(AttestationProvider::Ias),
				confidence_level: fields.confidence_level,
			});
			WorkerAddedAt::<T>::insert(pubkey, frame_system::Pallet::<T>::block_number());

			// If the master key has been created, register immediately as Keyfair
			if MasterPubkey::<T>::get().is_some() {
				let mut keyfairys = Keyfairies::<T>::get();
				if !keyfairys.contains(&pubkey) {
					keyfairys.push(pubkey);
					Keyfairies::<T>::put(keyfairys);

					Self::push_message(KeyfairyChange::keyfairy_registered(
						pubkey,
						ceseal_info.ecdh_pubkey,
					));
					Self::deposit_event(Event::<T>::KeyfairyAdded { pubkey });
				}
			}
			
			Ok(())
		}

		/// Registers a worker on the blockchain.
		/// This is the version 2 that both support DCAP attestation type.
		///
		/// Usually called by a bridging relayer program (`cifrost`). Can be called by
		/// anyone on behalf of a worker.
		#[pallet::call_index(17)]
		#[pallet::weight({0})]
		pub fn register_worker_v2(
			origin: OriginFor<T>,
			ceseal_info: WorkerRegistrationInfo<T::AccountId>,
			attestation: Box<Option<AttestationReport>>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			// Validate RA report & embedded user data
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			let runtime_info_hash = crate::hashing::blake2_256(&Encode::encode(&ceseal_info));
			let attestation_report = attestation::validate(
				*attestation,
				&runtime_info_hash,
				now,
				T::VerifyCeseal::get(),
				CesealBinAllowList::<T>::get(),
				T::NoneAttestationEnabled::get(),
			)
			.map_err(Into::<Error<T>>::into)?;

			// Update the registry
			let pubkey = ceseal_info.pubkey;

			ensure!(!Workers::<T>::contains_key(&pubkey), Error::<T>::CesealAlreadyExists);
			match &ceseal_info.operator {
				Some(acc) => {
					let _ = <pallet_cess_staking::Pallet<T>>::bonded(acc).ok_or(Error::<T>::NotBond)?;
					ensure!(ceseal_info.role == WorkerRole::Verifier || ceseal_info.role == WorkerRole::Full, Error::<T>::WrongTeeType);
				},
				None => ensure!(ceseal_info.role == WorkerRole::Marker, Error::<T>::WrongTeeType),
			};

			let worker_info = WorkerInfo {
				pubkey,
				ecdh_pubkey: ceseal_info.ecdh_pubkey,
				version: ceseal_info.version,
				last_updated: now,
				stash_account: ceseal_info.operator,
				attestation_provider: attestation_report.provider,
				confidence_level: attestation_report.confidence_level,
				features: ceseal_info.features,
				role: ceseal_info.role.clone(),
			};

			Workers::<T>::insert(&pubkey, worker_info);

			if ceseal_info.role == WorkerRole::Full || ceseal_info.role == WorkerRole::Verifier {
				ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
					puk_list
						.try_push(pubkey)
						.map_err(|_| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			}

			Self::push_message(SystemEvent::new_worker_event(
				pubkey,
				WorkerEvent::Registered(messaging::WorkerInfo {
					confidence_level: attestation_report.confidence_level,
				}),
			));
			Self::deposit_event(Event::<T>::WorkerAdded {
				pubkey,
				attestation_provider: attestation_report.provider,
				confidence_level: attestation_report.confidence_level,
			});
			WorkerAddedAt::<T>::insert(pubkey, frame_system::Pallet::<T>::block_number());

			// If the master key has been created, register immediately as Keyfair
			if MasterPubkey::<T>::get().is_some() {
				let mut keyfairys = Keyfairies::<T>::get();
				if !keyfairys.contains(&pubkey) {
					keyfairys.push(pubkey);
					Keyfairies::<T>::put(keyfairys);

					Self::push_message(KeyfairyChange::keyfairy_registered(
						pubkey,
						ceseal_info.ecdh_pubkey,
					));
					Self::deposit_event(Event::<T>::KeyfairyAdded { pubkey });
				}
			}
			Ok(())
		}
		
		#[pallet::call_index(18)]
		#[pallet::weight({0})]
		pub fn update_worker_endpoint(
			origin: OriginFor<T>,
			endpoint_payload: WorkerAction,
			signature: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			if let WorkerAction::UpdateEndpoint(endpoint_payload) = endpoint_payload {
				// Validate the signature
				ensure!(signature.len() == 64, Error::<T>::InvalidSignatureLength);
				let sig =
					sp_core::sr25519::Signature::try_from(signature.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
				let encoded_data = endpoint_payload.encode();
				let data_to_sign = wrap_content_to_sign(&encoded_data, SignedContentType::EndpointInfo);
				ensure!(
					sp_io::crypto::sr25519_verify(&sig, &data_to_sign, &endpoint_payload.base.pubkey),
					Error::<T>::InvalidSignature
				);

				let Some(endpoint) = endpoint_payload.endpoint else { return Err(Error::<T>::EmptyEndpoint.into()) };
				if endpoint.is_empty() {
					return Err(Error::<T>::EmptyEndpoint.into())
				}

				ensure!(
					Self::check_time_unix(&endpoint_payload.base.signing_time),
					Error::<T>::InvalidEndpointSigningTime
				);

				// Validate the public key
				ensure!(Workers::<T>::contains_key(endpoint_payload.base.pubkey), Error::<T>::InvalidPubKey);

				Endpoints::<T>::insert(endpoint_payload.base.pubkey, endpoint);
			} else {
				return Err(Error::<T>::PayloadError)?
			}
			

			Ok(())
		}

		/// Registers a ceseal binary to [`CesealBinAllowList`]
		///
		/// Can only be called by `GovernanceOrigin`.
		#[pallet::call_index(19)]
		#[pallet::weight({0})]
		pub fn add_ceseal(origin: OriginFor<T>, ceseal_hash: Vec<u8>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let mut allowlist = CesealBinAllowList::<T>::get();
			ensure!(!allowlist.contains(&ceseal_hash), Error::<T>::CesealAlreadyExists);

			allowlist.push(ceseal_hash.clone());
			CesealBinAllowList::<T>::put(allowlist);

			let now = frame_system::Pallet::<T>::block_number();
			CesealBinAddedAt::<T>::insert(&ceseal_hash, now);

			Ok(())
		}

		/// Removes a ceseal binary from [`CesealBinAllowList`]
		///
		/// Can only be called by `GovernanceOrigin`.
		#[pallet::call_index(110)]
		#[pallet::weight({0})]
		pub fn remove_ceseal(origin: OriginFor<T>, ceseal_hash: Vec<u8>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let mut allowlist = CesealBinAllowList::<T>::get();
			ensure!(allowlist.contains(&ceseal_hash), Error::<T>::CesealNotFound);

			allowlist.retain(|h| *h != ceseal_hash);
			CesealBinAllowList::<T>::put(allowlist);

			CesealBinAddedAt::<T>::remove(&ceseal_hash);

			Ok(())
		}

		/// Set minimum ceseal version. Versions less than MinimumCesealVersion would be forced to quit.
		///
		/// Can only be called by `GovernanceOrigin`.
		#[pallet::call_index(113)]
		#[pallet::weight({0})]
		pub fn set_minimum_ceseal_version(origin: OriginFor<T>, major: u32, minor: u32, patch: u32) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			MinimumCesealVersion::<T>::put((major, minor, patch));
			Self::deposit_event(Event::<T>::MinimumCesealVersionChangedTo(major, minor, patch));
			Ok(())
		}
	}

	impl<T: Config> ces_pallet_mq::MasterPubkeySupplier for Pallet<T> {
		fn try_get() -> Option<MasterPublicKey> {
			MasterPubkey::<T>::get()
		}
	}

	impl<T: Config> Pallet<T>
	where
		T: ces_pallet_mq::Config,
	{
		pub fn on_message_received(message: DecodedMessage<RegistryEvent>) -> DispatchResult {
			let worker_pubkey = match &message.sender {
				MessageOrigin::Worker(key) => key,
				_ => return Err(Error::<T>::InvalidSender.into()),
			};

			match message.payload {
				RegistryEvent::MasterPubkey { master_pubkey } => {
					let keyfairys = Keyfairies::<T>::get();
					ensure!(keyfairys.contains(worker_pubkey), Error::<T>::InvalidKeyfairy);
					match MasterPubkey::<T>::try_get() {
						Ok(saved_pubkey) => {
							ensure!(
								saved_pubkey.0 == master_pubkey.0,
								Error::<T>::MasterKeyMismatch // Oops, this is really bad
							);
						},
						_ => {
							MasterPubkey::<T>::put(master_pubkey);
							Self::push_message(KeyfairyLaunch::master_pubkey_on_chain(master_pubkey));
							Self::on_keyfairy_launched();
						},
					}
				},
			}
			Ok(())
		}

		pub fn on_keyfairy_message_received(message: DecodedMessage<KeyfairyRegistryEvent>) -> DispatchResult {
			if !message.sender.is_keyfairy() {
				return Err(Error::<T>::InvalidSender.into())
			}

			match message.payload {
				KeyfairyRegistryEvent::RotatedMasterPubkey { rotation_id, master_pubkey } => {
					let rotating = MasterKeyRotationLock::<T>::get();
					if rotating.is_none() || rotating.unwrap() != rotation_id {
						Self::deposit_event(Event::<T>::MasterKeyRotationFailed {
							rotation_lock: rotating,
							keyfairy_rotation_id: rotation_id,
						});
						return Err(Error::<T>::InvalidRotatedMasterPubkey.into())
					}

					MasterPubkey::<T>::put(master_pubkey);
					MasterKeyRotationLock::<T>::put(Option::<u64>::None);
					Self::deposit_event(Event::<T>::MasterKeyRotated { rotation_id, master_pubkey });
					Self::push_message(KeyfairyLaunch::master_pubkey_rotated(master_pubkey));
				},
			}
			Ok(())
		}

		fn on_keyfairy_launched() {
			let block_number = frame_system::Pallet::<T>::block_number();
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			KeyfairyLaunchedAt::<T>::put((block_number, now));
			Self::deposit_event(Event::<T>::MasterKeyLaunched);
		}
	}

	impl<T: Config + ces_pallet_mq::Config> MessageOriginInfo for Pallet<T> {
		type Config = T;
	}

	/// The basic information of a registered worker
	#[derive(Encode, Decode, TypeInfo, Debug, Clone)]
	pub struct WorkerInfo<AccountId> {
		/// The identity public key of the worker
		pub pubkey: WorkerPublicKey,
		/// The public key for ECDH communication
		pub ecdh_pubkey: EcdhPublicKey,
		/// The ceseal version of the worker upon registering
		pub version: u32,
		/// The unix timestamp of the last updated time
		pub last_updated: u64,
		/// The stake pool owner that can control this worker
		///
		/// When initializing ceseal, the user can specify an _operator account_. Then this field
		/// will be updated when the worker is being registered on the blockchain. Once it's set,
		/// the worker can only be added to a stake pool if the pool owner is the same as the
		/// operator. It ensures only the trusted person can control the worker.
		pub stash_account: Option<AccountId>,
		/// Who issues the attestation
		pub attestation_provider: Option<AttestationProvider>,
		/// The confidence level of the worker
		pub confidence_level: u8,
		/// Deprecated
		pub features: Vec<u32>,

		pub role: WorkerRole,
	}

	impl<T: Config> From<AttestationError> for Error<T> {
		fn from(err: AttestationError) -> Self {
			match err {
				AttestationError::CesealRejected => Self::CesealRejected,
				AttestationError::InvalidIASSigningCert => Self::InvalidIASSigningCert,
				AttestationError::InvalidReport => Self::InvalidReport,
				AttestationError::InvalidQuoteStatus => Self::InvalidQuoteStatus,
				AttestationError::BadIASReport => Self::BadIASReport,
				AttestationError::OutdatedIASReport => Self::OutdatedIASReport,
				AttestationError::UnknownQuoteBodyFormat => Self::UnknownQuoteBodyFormat,
				AttestationError::InvalidUserDataHash => Self::InvalidCesealInfoHash,
				AttestationError::NoneAttestationDisabled => Self::NoneAttestationDisabled,
			}
		}
	}
}
pub trait TeeWorkerHandler<AccountId> {
	fn can_tag(pbk: &WorkerPublicKey) -> bool;
	fn can_verify(pbk: &WorkerPublicKey) -> bool;
	fn can_cert(pbk: &WorkerPublicKey) -> bool;
	fn contains_scheduler(pbk: WorkerPublicKey) -> bool;
	fn is_bonded(pbk: &WorkerPublicKey) -> bool;
	fn get_stash(pbk: &WorkerPublicKey) -> Result<AccountId, DispatchError>;
	fn punish_scheduler(pbk: WorkerPublicKey) -> DispatchResult;
	fn get_pubkey_list() -> Vec<WorkerPublicKey>; // get_controller_list
	fn get_master_publickey() -> Result<MasterPublicKey, DispatchError>;
}

impl<T: Config> TeeWorkerHandler<AccountOf<T>> for Pallet<T> {
	fn can_tag(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			if WorkerRole::Marker == tee_info.role || WorkerRole::Full == tee_info.role {
				return true
			}
		}

		false
	}

	fn can_verify(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			if WorkerRole::Verifier == tee_info.role || WorkerRole::Full == tee_info.role {
				return true
			}
		}

		false
	}

	fn can_cert(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			if WorkerRole::Marker == tee_info.role || WorkerRole::Full == tee_info.role {
				return true
			}
		}

		false
	}

	fn contains_scheduler(pbk: WorkerPublicKey) -> bool {
		Workers::<T>::contains_key(&pbk)
	}

	fn is_bonded(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			let result = tee_info.stash_account.is_some();
			return result
		}

		false
	}

	fn get_stash(pbk: &WorkerPublicKey) -> Result<AccountOf<T>, DispatchError> {
		let tee_info = Workers::<T>::try_get(pbk).map_err(|_| Error::<T>::NonTeeWorker)?;

		if let Some(stash_account) = tee_info.stash_account {
			return Ok(stash_account)
		}

		Err(Error::<T>::NonTeeWorker.into())
	}

	fn punish_scheduler(pbk: WorkerPublicKey) -> DispatchResult {
		let tee_worker = Workers::<T>::try_get(&pbk).map_err(|_| Error::<T>::NonTeeWorker)?;
		if let Some(stash_account) = tee_worker.stash_account {
			pallet_cess_staking::slashing::slash_scheduler::<T>(&stash_account);
			T::CreditCounter::record_punishment(&stash_account)?;
		}

		Ok(())
	}

	fn get_pubkey_list() -> Vec<WorkerPublicKey> {
		let acc_list = <ValidationTypeList<T>>::get().to_vec();

		acc_list
	}

	fn get_master_publickey() -> Result<MasterPublicKey, DispatchError> {
		let pk = MasterPubkey::<T>::try_get().map_err(|_| Error::<T>::TeePodr2PkNotInitialized)?;

		Ok(pk)
	}
}
