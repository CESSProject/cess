//! # Tee Worker Module
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

mod mock;
mod types;
pub use types::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use ces_types::{MasterPublicKey, WorkerPublicKey, WorkerRole};
use codec::{Decode, Encode};
use cp_cess_common::*;
use cp_scheduler_credit::SchedulerCreditCounter;
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
	traits::{Get, ReservableCurrency, StorageVersion, UnixTime},
	BoundedVec, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::*};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, RuntimeDebug, SaturatedConversion, Saturating};
use sp_std::{convert::TryInto, prelude::*};
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
	use frame_support::dispatch::DispatchResult;
	use scale_info::TypeInfo;
	use sp_core::H256;

	use ces_pallet_mq::MessageOriginInfo;
	use ces_types::{
		attestation::{self, Error as AttestationError},
		messaging::{bind_topic, DecodedMessage, MasterKeyApply, MasterKeyLaunch, MessageOrigin, WorkerEvent},
		wrap_content_to_sign, AttestationProvider, EcdhPublicKey, MasterKeyApplyPayload, SignedContentType,
		WorkerEndpointPayload, WorkerRegistrationInfo,
	};

	// Re-export
	pub use ces_types::AttestationReport;
	// TODO: Legacy
	pub use ces_types::attestation::legacy::{Attestation, AttestationValidator, IasFields, IasValidator};

	bind_topic!(MasterKeySubmission, b"^cess/masterkey/submit");
	#[derive(Encode, Decode, TypeInfo, Clone, Debug)]
	pub enum MasterKeySubmission {
		///	MessageOrigin::Worker -> Pallet
		///
		/// Only used for first master pubkey upload, the origin has to be worker identity since there is no master
		/// pubkey on-chain yet.
		MasterPubkey { master_pubkey: MasterPublicKey },
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

		#[pallet::constant]
		type AtLeastWorkBlock: Get<BlockNumberFor<Self>>;

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

		MasterKeyLaunching {
			holder: WorkerPublicKey,
		},

		MasterKeyLaunched,

		MasterKeyApplied {
			worker_pubkey: WorkerPublicKey,
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

		MinimumCesealVersionChangedTo(u32, u32, u32),

		NotBond,

		WrongTeeType,

		CesealAlreadyExists,

		ValidateError,

		BoundedVecError,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Boundedvec conversion error
		BoundedVecError,

		NotBond,

		NonTeeWorker,

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
		InvalidWorkerPubKey,
		MalformedSignature,
		InvalidSignatureLength,
		InvalidSignature,
		/// IAS related
		WorkerNotFound,
		/// master-key launch related
		MasterKeyLaunchRequire,
		InvalidMasterKeyFirstHolder,
		MasterKeyAlreadyLaunched,
		MasterKeyLaunching,
		MasterKeyMismatch,
		MasterKeyUninitialized,
		InvalidMasterKeyApplySigningTime,
		/// Ceseal related
		CesealAlreadyExists,
		CesealBinNotFound,

		CannotExitMasterKeyHolder,

		EmptyEndpoint,
		InvalidEndpointSigningTime,

		PayloadError,

		LastWorker,
	}

	#[pallet::storage]
	#[pallet::getter(fn validation_type_list)]
	pub(super) type ValidationTypeList<T: Config> =
		StorageValue<_, BoundedVec<WorkerPublicKey, T::SchedulerMaximum>, ValueQuery>;

	#[pallet::storage]
	pub type MasterKeyFirstHolder<T: Config> = StorageValue<_, WorkerPublicKey>;

	/// Master public key
	#[pallet::storage]
	pub type MasterPubkey<T: Config> = StorageValue<_, MasterPublicKey>;

	/// The block number and unix timestamp when the master-key is launched
	#[pallet::storage]
	pub type MasterKeyLaunchedAt<T: Config> = StorageValue<_, (BlockNumberFor<T>, u64)>;

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
	pub type CesealBinAllowList<T: Config> = StorageValue<_, Vec<H256>, ValueQuery>;

	/// The effective height of ceseal binary
	#[pallet::storage]
	pub type CesealBinAddedAt<T: Config> = StorageMap<_, Twox64Concat, H256, BlockNumberFor<T>>;

	/// Mapping from worker pubkey to CESS Network identity
	#[pallet::storage]
	pub type Endpoints<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, alloc::string::String>;

	#[pallet::storage]
	pub type LastWork<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, BlockNumberFor<T>, ValueQuery>;

	/// Ceseals whoes version less than MinimumCesealVersion would be forced to quit.
	#[pallet::storage]
	pub type MinimumCesealVersion<T: Config> = StorageValue<_, (u32, u32, u32), ValueQuery>;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let weight: Weight = Weight::zero();

			let least = T::AtLeastWorkBlock::get();
			if now % least == 0u32.saturated_into() {
				weight.saturating_add(Self::clear_mission(now));
			}

			weight
		}
	}

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
		// #[pallet::call_index(2)]
		// #[pallet::weight(Weight::zero())]
		// pub fn exit(
		// 	origin: OriginFor<T>,
		// 	payload: WorkerAction,
		// 	sig: BoundedVec<u8, ConstU32<64>>
		// ) -> DispatchResult {
		// 	ensure_signed(origin)?;

		// 	if let WorkerAction::Exit(payload) = payload {
		// 		ensure!(sig.len() == 64, Error::<T>::InvalidSignatureLength);
		// 		let sig =
		// 			sp_core::sr25519::Signature::try_from(sig.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
		// 		let encoded_data = payload.encode();
		// 		let data_to_sign = wrap_content_to_sign(&encoded_data, SignedContentType::EndpointInfo);
		// 		ensure!(
		// 			sp_io::crypto::sr25519_verify(&sig, &data_to_sign, &payload.pubkey),
		// 			Error::<T>::InvalidSignature
		// 		);

		// 		ensure!(<Workers<T>>::count() > 1, Error::<T>::LastWorker);
		// 		ensure!(<Workers<T>>::contains_key(&payload.pubkey), Error::<T>::WorkerNotFound);

		// 		Workers::<T>::remove(&payload.pubkey);
		// 		WorkerAddedAt::<T>::remove(&payload.pubkey);
		// 		Endpoints::<T>::remove(&payload.pubkey);

		// 		let mut keyfairys = Keyfairies::<T>::get();
		// 		ensure!(keyfairys.len() > 1, Error::<T>::CannotRemoveLastKeyfairy);
		// 		keyfairys.retain(|g| *g != payload.pubkey);
		// 		Keyfairies::<T>::put(keyfairys);

		// 		ensure!(
		// 			Self::check_time_unix(&payload.signing_time),
		// 			Error::<T>::InvalidEndpointSigningTime
		// 		);

		// 		Self::deposit_event(Event::<T>::Exit { tee: payload.pubkey });
		// 	} else {
		// 		return Err(Error::<T>::PayloadError)?
		// 	}

		// 	Ok(())
		// }

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
				stash_account,
				attestation_provider: Some(AttestationProvider::Root),
				confidence_level: 128u8,
				features: vec![1, 4],
				role: WorkerRole::Full,
			};
			Workers::<T>::insert(worker_info.pubkey, &worker_info);
			WorkerAddedAt::<T>::insert(worker_info.pubkey, frame_system::Pallet::<T>::block_number());
			Self::push_message(WorkerEvent::new_worker(pubkey));
			Self::deposit_event(Event::<T>::WorkerAdded {
				pubkey,
				attestation_provider: Some(AttestationProvider::Root),
				confidence_level: worker_info.confidence_level,
			});

			Ok(())
		}

		/// Launch master-key
		///
		/// Can only be called by `GovernanceOrigin`.
		#[pallet::call_index(13)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn launch_master_key(origin: OriginFor<T>, worker_pubkey: WorkerPublicKey) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(MasterPubkey::<T>::get().is_none(), Error::<T>::MasterKeyAlreadyLaunched);
			ensure!(MasterKeyFirstHolder::<T>::get().is_none(), Error::<T>::MasterKeyLaunching);

			let worker_info = Workers::<T>::get(worker_pubkey).ok_or(Error::<T>::WorkerNotFound)?;
			MasterKeyFirstHolder::<T>::put(worker_pubkey);
			Self::push_message(MasterKeyLaunch::launch_request(worker_pubkey, worker_info.ecdh_pubkey));
			// wait for the lead worker to upload the master pubkey
			Self::deposit_event(Event::<T>::MasterKeyLaunching { holder: worker_pubkey });
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
			.map_err({
				Self::deposit_event(Event::<T>::ValidateError);
				Into::<Error<T>>::into
			})?;

			// Update the registry
			let pubkey = ceseal_info.pubkey;

			if Workers::<T>::contains_key(&pubkey) {
				Self::deposit_event(Event::<T>::CesealAlreadyExists);
				return Err(Error::<T>::CesealAlreadyExists)?;
			}

			match &ceseal_info.operator {
				Some(acc) => {
					let result = <pallet_cess_staking::Pallet<T>>::bonded(acc).ok_or(Error::<T>::NotBond);
					if result.is_err() {
						Self::deposit_event(Event::<T>::NotBond);
						return Err(Error::<T>::NotBond)?;
					}

					// ensure!(ceseal_info.role == WorkerRole::Verifier || ceseal_info.role == WorkerRole::Full, Error::<T>::WrongTeeType);
					if ceseal_info.role != WorkerRole::Verifier && ceseal_info.role != WorkerRole::Full {
						Self::deposit_event(Event::<T>::WrongTeeType);
						return Err(Error::<T>::WrongTeeType)?;
					}
				},

				None => {
					if ceseal_info.role != WorkerRole::Marker {
						Self::deposit_event(Event::<T>::WrongTeeType);
						return Err(Error::<T>::WrongTeeType)?;
					}
					// ensure!(ceseal_info.role == WorkerRole::Marker, Error::<T>::WrongTeeType)
				},
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
			let now = <frame_system::Pallet<T>>::block_number();
			<LastWork<T>>::insert(&pubkey, now);

			if ceseal_info.role == WorkerRole::Full || ceseal_info.role == WorkerRole::Verifier {
				ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
					puk_list
						.try_push(pubkey)
						.map_err(|_| {Self::deposit_event(Event::<T>::BoundedVecError); Error::<T>::BoundedVecError})?;
					Ok(())
				})?;
			}

			Self::push_message(WorkerEvent::new_worker(pubkey));
			Self::deposit_event(Event::<T>::WorkerAdded {
				pubkey,
				attestation_provider: Some(AttestationProvider::Ias),
				confidence_level: fields.confidence_level,
			});
			WorkerAddedAt::<T>::insert(pubkey, frame_system::Pallet::<T>::block_number());

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
			.map_err({
				Self::deposit_event(Event::<T>::ValidateError);
				Into::<Error<T>>::into
			})?;

			// Update the registry
			let pubkey = ceseal_info.pubkey;

			if Workers::<T>::contains_key(&pubkey) {
				Self::deposit_event(Event::<T>::CesealAlreadyExists);
				return Err(Error::<T>::CesealAlreadyExists)?;
			}

			match &ceseal_info.operator {
				Some(acc) => {
					let result = <pallet_cess_staking::Pallet<T>>::bonded(acc).ok_or(Error::<T>::NotBond);
					if result.is_err() {
						Self::deposit_event(Event::<T>::NotBond);
						return Err(Error::<T>::NotBond)?;
					}

					// ensure!(ceseal_info.role == WorkerRole::Verifier || ceseal_info.role == WorkerRole::Full, Error::<T>::WrongTeeType);
					if ceseal_info.role != WorkerRole::Verifier && ceseal_info.role != WorkerRole::Full {
						Self::deposit_event(Event::<T>::WrongTeeType);
						return Err(Error::<T>::WrongTeeType)?;
					}
				},

				None => {
					if ceseal_info.role != WorkerRole::Marker {
						Self::deposit_event(Event::<T>::WrongTeeType);
						return Err(Error::<T>::WrongTeeType)?;
					}
					// ensure!(ceseal_info.role == WorkerRole::Marker, Error::<T>::WrongTeeType)
				},
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
			let now = <frame_system::Pallet<T>>::block_number();
			<LastWork<T>>::insert(&pubkey, now);

			if ceseal_info.role == WorkerRole::Full || ceseal_info.role == WorkerRole::Verifier {
				ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
					puk_list
						.try_push(pubkey)
						.map_err(|_| {Self::deposit_event(Event::<T>::BoundedVecError); Error::<T>::BoundedVecError})?;
					Ok(())
				})?;
			}

			Self::push_message(WorkerEvent::new_worker(pubkey));
			Self::deposit_event(Event::<T>::WorkerAdded {
				pubkey,
				attestation_provider: attestation_report.provider,
				confidence_level: attestation_report.confidence_level,
			});
			WorkerAddedAt::<T>::insert(pubkey, frame_system::Pallet::<T>::block_number());

			Ok(())
		}

		#[pallet::call_index(18)]
		#[pallet::weight({0})]
		pub fn update_worker_endpoint(
			origin: OriginFor<T>,
			endpoint_payload: WorkerEndpointPayload,
			signature: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			// Validate the signature
			ensure!(signature.len() == 64, Error::<T>::InvalidSignatureLength);
			let sig =
				sp_core::sr25519::Signature::try_from(signature.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
			let encoded_data = endpoint_payload.encode();
			let data_to_sign = wrap_content_to_sign(&encoded_data, SignedContentType::EndpointInfo);
			ensure!(
				sp_io::crypto::sr25519_verify(&sig, &data_to_sign, &endpoint_payload.pubkey),
				Error::<T>::InvalidSignature
			);

			let Some(endpoint) = endpoint_payload.endpoint else { return Err(Error::<T>::EmptyEndpoint.into()) };
			if endpoint.is_empty() {
				return Err(Error::<T>::EmptyEndpoint.into())
			}

			// Validate the time
			let expiration = 4 * 60 * 60 * 1000; // 4 hours
			let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
			ensure!(
				endpoint_payload.signing_time < now && now <= endpoint_payload.signing_time + expiration,
				Error::<T>::InvalidEndpointSigningTime
			);

			// Validate the public key
			ensure!(Workers::<T>::contains_key(endpoint_payload.pubkey), Error::<T>::InvalidWorkerPubKey);

			Endpoints::<T>::insert(endpoint_payload.pubkey, endpoint);

			Ok(())
		}

		#[pallet::call_index(21)]
		#[pallet::weight({0})]
		pub fn apply_master_key(
			origin: OriginFor<T>,
			payload: MasterKeyApplyPayload,
			signature: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			// Validate the signature
			ensure!(signature.len() == 64, Error::<T>::InvalidSignatureLength);
			let sig =
				sp_core::sr25519::Signature::try_from(signature.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
			let encoded_data = payload.encode();
			let data_to_sign = wrap_content_to_sign(&encoded_data, SignedContentType::MasterKeyApply);
			ensure!(sp_io::crypto::sr25519_verify(&sig, &data_to_sign, &payload.pubkey), Error::<T>::InvalidSignature);

			// Validate the time
			let expiration = 30 * 60 * 1000; // 30 minutes
			let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
			ensure!(
				payload.signing_time < now && now <= payload.signing_time + expiration,
				Error::<T>::InvalidMasterKeyApplySigningTime
			);

			// Validate the public key
			ensure!(Workers::<T>::contains_key(payload.pubkey), Error::<T>::InvalidWorkerPubKey);

			Self::push_message(MasterKeyApply::Apply(payload.pubkey.clone(), payload.ecdh_pubkey));
			Self::deposit_event(Event::<T>::MasterKeyApplied { worker_pubkey: payload.pubkey });
			Ok(())
		}

		/// Registers a ceseal binary to [`CesealBinAllowList`]
		///
		/// Can only be called by `GovernanceOrigin`.
		#[pallet::call_index(19)]
		#[pallet::weight({0})]
		pub fn add_ceseal(origin: OriginFor<T>, ceseal_hash: H256) -> DispatchResult {
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
		pub fn remove_ceseal(origin: OriginFor<T>, ceseal_hash: H256) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			let mut allowlist = CesealBinAllowList::<T>::get();
			ensure!(allowlist.contains(&ceseal_hash), Error::<T>::CesealBinNotFound);

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

		#[pallet::call_index(114)]
		#[pallet::weight({0})]
		pub fn migration_last_work(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let now = <frame_system::Pallet<T>>::block_number();
			for (puk, _) in Workers::<T>::iter() {
				<LastWork<T>>::insert(&puk, now);
			}

			Ok(())
		}
		// FOR TEST
		#[pallet::call_index(115)]
		#[pallet::weight({0})]
		pub fn patch_clear_invalid_tee(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
				puk_list.retain(|g| Endpoints::<T>::contains_key(g));
				Ok(())
			})?;

			Ok(())
		}
		// FOR TEST
		#[pallet::call_index(116)]
		#[pallet::weight({0})]
		pub fn patch_clear_not_work_tee(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			for (puk, _) in Workers::<T>::iter() {
				if !<LastWork<T>>::contains_key(&puk) {
					Self::execute_exit(puk)?;
				}
			}

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
		pub fn on_message_received(message: DecodedMessage<MasterKeySubmission>) -> DispatchResult {
			let worker_pubkey = match &message.sender {
				MessageOrigin::Worker(key) => key,
				_ => return Err(Error::<T>::InvalidSender.into()),
			};

			let holder = MasterKeyFirstHolder::<T>::get().ok_or(Error::<T>::MasterKeyLaunchRequire)?;
			match message.payload {
				MasterKeySubmission::MasterPubkey { master_pubkey } => {
					ensure!(worker_pubkey.0 == holder.0, Error::<T>::InvalidMasterKeyFirstHolder);
					match MasterPubkey::<T>::try_get() {
						Ok(saved_pubkey) => {
							ensure!(
								saved_pubkey.0 == master_pubkey.0,
								Error::<T>::MasterKeyMismatch // Oops, this is really bad
							);
						},
						_ => {
							MasterPubkey::<T>::put(master_pubkey);
							Self::push_message(MasterKeyLaunch::on_chain_launched(master_pubkey));
							Self::on_master_key_launched();
						},
					}
				},
			}
			Ok(())
		}

		fn on_master_key_launched() {
			let block_number = frame_system::Pallet::<T>::block_number();
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			MasterKeyLaunchedAt::<T>::put((block_number, now));
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
pub trait TeeWorkerHandler<AccountId, Block> {
	fn can_tag(pbk: &WorkerPublicKey) -> bool;
	fn can_verify(pbk: &WorkerPublicKey) -> bool;
	fn can_cert(pbk: &WorkerPublicKey) -> bool;
	fn contains_scheduler(pbk: WorkerPublicKey) -> bool;
	fn is_bonded(pbk: &WorkerPublicKey) -> bool;
	fn get_stash(pbk: &WorkerPublicKey) -> Result<AccountId, DispatchError>;
	fn punish_scheduler(pbk: WorkerPublicKey) -> DispatchResult;
	fn get_pubkey_list() -> Vec<WorkerPublicKey>; // get_controller_list
	fn get_master_publickey() -> Result<MasterPublicKey, DispatchError>;
	fn update_work_block(now: Block, pbk: &WorkerPublicKey) -> DispatchResult;
}

impl<T: Config> TeeWorkerHandler<AccountOf<T>, BlockNumberFor<T>> for Pallet<T> {
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
		let pk = MasterPubkey::<T>::try_get().map_err(|_| Error::<T>::MasterKeyUninitialized)?;

		Ok(pk)
	}

	fn update_work_block(now: BlockNumberFor<T>, pbk: &WorkerPublicKey) -> DispatchResult {
		<LastWork<T>>::try_mutate(pbk, |last_block| {
			*last_block = now;

			Ok(())
		})
	}
}
