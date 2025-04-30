//! # Tee Worker Module
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

mod mock;
mod types;
pub use types::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use alloc::string::{String, ToString};
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
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, RuntimeDebug, SaturatedConversion, Saturating};
use sp_std::{convert::TryInto, prelude::*};

pub use pallet::*;
pub use weights::WeightInfo;
pub mod weights;

mod functions;

extern crate alloc;

#[cfg(feature = "native")]
use sp_core::hashing;

#[cfg(not(feature = "native"))]
use sp_io::hashing;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

type SHA256 = [u8; 32];

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use codec::{Decode, Encode};
	use frame_support::dispatch::DispatchResult;
	use scale_info::TypeInfo;
	use sp_core::H256;

	use ces_types::{
		attestation::{self, Error as AttestationError},
		AttestationProvider, EcdhPublicKey, MasterKeyApplyPayload, MasterKeyDistributePayload, MasterKeyLaunchPayload,
		WorkerRegistrationInfo,
	};

	// Re-export
	pub use ces_types::AttestationReport;

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

		#[pallet::constant]
		type AtLeastWorkBlock: Get<BlockNumberFor<Self>>;

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

		MasterKeyAppling {
			applier: WorkerPublicKey,
			distributor: WorkerPublicKey,
		},

		MasterKeySubmitted {
			receiver: WorkerPublicKey,
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

		ChangeFirstHolder {
			pubkey: WorkerPublicKey,
		},

		ClearInvalidTee {
			pubkey: WorkerPublicKey,
		},

		RefreshStatus {
			pubkey: WorkerPublicKey,
			level: u8,
		},

		MinimumCesealVersionChangedTo(u32, u32, u32),

		CesealBinAdded(H256),

		CesealBinRemoved(H256),
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

		UnsupportedAttestationType,

		InvalidDCAPQuote,

		InvalidCertificate,
		CodecError,
		TCBInfoExpired,
		KeyLengthIsInvalid,
		PublicKeyIsInvalid,
		RsaSignatureIsInvalid,
		DerEncodingError,
		UnsupportedDCAPQuoteVersion,
		UnsupportedDCAPAttestationKeyType,
		UnsupportedQuoteAuthData,
		UnsupportedDCAPPckCertFormat,
		LeafCertificateParsingError,
		CertificateChainIsInvalid,
		CertificateChainIsTooShort,
		IntelExtensionCertificateDecodingError,
		IntelExtensionAmbiguity,
		CpuSvnLengthMismatch,
		CpuSvnDecodingError,
		PceSvnDecodingError,
		PceSvnLengthMismatch,
		FmspcLengthMismatch,
		FmspcDecodingError,
		FmspcMismatch,
		QEReportHashMismatch,
		IsvEnclaveReportSignatureIsInvalid,
		DerDecodingError,
		OidIsMissing,

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
		MasterKeyFirstHolderNotFound,
		MasterKeyAlreadyLaunched,
		MasterKeyLaunching,
		MasterKeyMismatch,
		MasterKeyUninitialized,
		InvalidMasterKeyApplySigningTime,

		CesealAlreadyExists,

		CesealBinAlreadyExists,
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

	#[pallet::storage]
	pub type OldMasterPubkey<T: Config> = StorageValue<_, MasterPublicKey>;

	#[pallet::storage]
	pub type OldMasterPubkeyList<T: Config> = StorageValue<_, Vec<MasterPublicKey>, ValueQuery>;

	/// The block number and unix timestamp when the master-key is launched
	#[pallet::storage]
	pub type MasterKeyLaunchedAt<T: Config> = StorageValue<_, (BlockNumberFor<T>, u64)>;

	#[pallet::storage]
	pub type MasterKeyPostation<T: Config> =
		StorageMap<_, Twox64Concat, WorkerPublicKey, Option<(BlockNumberFor<T>, MasterKeyDistributePayload)>>;

	#[pallet::storage]
	pub type MasterKeyDistributeNotify<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, WorkerPublicKey>;

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
	/// deprecated, use Workers element value.endpoint instead
	#[pallet::storage]
	pub type Endpoints<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, alloc::string::String>;

	#[pallet::storage]
	pub type LastWork<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	pub type LastRefresh<T: Config> = StorageMap<_, Twox64Concat, WorkerPublicKey, BlockNumberFor<T>, ValueQuery>;

	/// Ceseals whoes version less than MinimumCesealVersion would be forced to quit.
	#[pallet::storage]
	pub type MinimumCesealVersion<T: Config> = StorageValue<_, (u32, u32, u32), ValueQuery>;

	/// Is none-attestation enabled
	///
	/// SHOULD BE SET TO FALSE ON PRODUCTION !!!
	#[pallet::storage]
	pub type NoneAttestationEnabled<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Is `ceseal_bin` verification required
	///
	/// SHOULD BE SET TO TRUE ON PRODUCTION!!!
	#[pallet::storage]
	pub type CesealVerifyRequired<T: Config> = StorageValue<_, bool, ValueQuery>;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub none_attestation_enabled: bool,
		pub ceseal_verify_required: bool,
		_marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				// For Development network
				none_attestation_enabled: true,
				ceseal_verify_required: false,
				_marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			NoneAttestationEnabled::<T>::put(self.none_attestation_enabled);
			CesealVerifyRequired::<T>::put(self.ceseal_verify_required);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let weight: Weight = Weight::zero();

			let least = T::AtLeastWorkBlock::get();
			if now % least == 0u32.saturated_into() {
				weight.saturating_add(Self::clear_mission(now));
			}

			weight.saturating_add(Self::clean_expired_master_key_postation(now));

			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers a worker on the blockchain.
		///
		/// Both support DCAP and EPID attestation type.
		#[pallet::call_index(1)]
		#[pallet::weight({0})]
		pub fn register_worker(
			origin: OriginFor<T>,
			ceseal_info: WorkerRegistrationInfo<T::AccountId>,
			attestation: Box<Option<AttestationReport>>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			match &ceseal_info.stash_account {
				Some(acc) => {
					<pallet_cess_staking::Pallet<T>>::bonded(acc).ok_or(Error::<T>::NotBond)?;
					ensure!(
						ceseal_info.role == WorkerRole::Verifier || ceseal_info.role == WorkerRole::Full,
						Error::<T>::WrongTeeType
					);
				},
				None => ensure!(ceseal_info.role == WorkerRole::Marker, Error::<T>::WrongTeeType),
			};
			// Validate RA report & embedded user data
			let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
			let runtime_info_hash = crate::hashing::blake2_256(&Encode::encode(&ceseal_info));
			let attestation_report = attestation::validate(
				*attestation,
				&runtime_info_hash,
				now,
				CesealVerifyRequired::<T>::get(),
				CesealBinAllowList::<T>::get(),
				NoneAttestationEnabled::<T>::get(),
			)
			.map_err(Into::<Error<T>>::into)?;

			// Update the registry
			let pubkey = ceseal_info.pubkey;
			ensure!(!Workers::<T>::contains_key(&pubkey), Error::<T>::CesealAlreadyExists);

			if !Workers::<T>::contains_key(&pubkey) {
				let worker_info = WorkerInfo {
					pubkey,
					ecdh_pubkey: ceseal_info.ecdh_pubkey,
					version: ceseal_info.version,
					last_updated: now,
					stash_account: ceseal_info.stash_account,
					attestation_provider: attestation_report.provider,
					confidence_level: attestation_report.confidence_level,
					features: ceseal_info.features,
					role: ceseal_info.role.clone(),
					endpoint: ceseal_info.endpoint.clone(),
				};

				Workers::<T>::insert(&pubkey, worker_info);
				WorkerAddedAt::<T>::insert(&pubkey, frame_system::Pallet::<T>::block_number());
				Endpoints::<T>::insert(&pubkey, ceseal_info.endpoint.clone()); //will deprecated
				let now = <frame_system::Pallet<T>>::block_number();
				<LastWork<T>>::insert(&pubkey, now);
				<LastRefresh<T>>::insert(&pubkey, now);

				if ceseal_info.role == WorkerRole::Full || ceseal_info.role == WorkerRole::Verifier {
					ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
						puk_list.try_push(pubkey).map_err(|_| Error::<T>::BoundedVecError)?;
						Ok(())
					})?;
				}

				Self::deposit_event(Event::<T>::WorkerAdded {
					pubkey,
					attestation_provider: attestation_report.provider,
					confidence_level: attestation_report.confidence_level,
				});
			} else {
				Workers::<T>::try_mutate(&pubkey, |worker_opt| -> DispatchResult {
					let worker = worker_opt.as_mut().ok_or(Error::<T>::NonTeeWorker)?;
					worker.version = ceseal_info.version;
					worker.last_updated = now;
					worker.stash_account = ceseal_info.stash_account;
					worker.attestation_provider = attestation_report.provider;
					worker.confidence_level = attestation_report.confidence_level;
					worker.features = ceseal_info.features;
					worker.role = ceseal_info.role;
					worker.endpoint = ceseal_info.endpoint.clone();
					Ok(())
				})?;
				<LastRefresh<T>>::insert(&pubkey, <frame_system::Pallet<T>>::block_number());
				Endpoints::<T>::insert(&pubkey, ceseal_info.endpoint.clone());
				Self::deposit_event(Event::<T>::RefreshStatus { pubkey, level: attestation_report.confidence_level });
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
				stash_account,
				attestation_provider: Some(AttestationProvider::Root),
				confidence_level: 128u8,
				features: vec![1, 4],
				role: WorkerRole::Full,
				endpoint: "https://cess.network/".to_string(),
			};
			Workers::<T>::insert(worker_info.pubkey, &worker_info);
			WorkerAddedAt::<T>::insert(worker_info.pubkey, frame_system::Pallet::<T>::block_number());
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

			let _worker_info = Workers::<T>::get(worker_pubkey).ok_or(Error::<T>::WorkerNotFound)?;
			MasterKeyFirstHolder::<T>::put(worker_pubkey);
			// wait for the lead worker to upload the master pubkey
			Self::deposit_event(Event::<T>::MasterKeyLaunching { holder: worker_pubkey });
			Ok(())
		}

		#[pallet::call_index(31)]
		#[pallet::weight({0})]
		pub fn settle_master_key_launch(
			origin: OriginFor<T>,
			payload: MasterKeyLaunchPayload,
			_signature: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			let holder = MasterKeyFirstHolder::<T>::get().ok_or(Error::<T>::MasterKeyLaunchRequire)?;
			ensure!(payload.launcher.0 == holder.0, Error::<T>::InvalidMasterKeyFirstHolder);
			match MasterPubkey::<T>::try_get() {
				Ok(saved_pubkey) => {
					ensure!(
						saved_pubkey.0 == payload.master_pubkey.0,
						Error::<T>::MasterKeyMismatch // Oops, this is really bad
					);
				},
				_ => {
					MasterPubkey::<T>::put(payload.master_pubkey);
					let block_number = frame_system::Pallet::<T>::block_number();
					let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
					MasterKeyLaunchedAt::<T>::put((block_number, now));
					Self::deposit_event(Event::<T>::MasterKeyLaunched);
				},
			}
			Ok(())
		}

		#[pallet::call_index(14)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn clear_master_key(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(MasterPubkey::<T>::get().is_some(), Error::<T>::MasterKeyAlreadyLaunched);
			ensure!(MasterKeyFirstHolder::<T>::get().is_some(), Error::<T>::MasterKeyLaunching);

			let old_pubkey = MasterPubkey::<T>::get().ok_or(Error::<T>::WorkerNotFound)?;
			MasterKeyFirstHolder::<T>::kill();
			let old_pubkey2 = OldMasterPubkey::<T>::get().ok_or(Error::<T>::WorkerNotFound)?;
			OldMasterPubkeyList::<T>::mutate(|list| {
				list.push(old_pubkey);
				list.push(old_pubkey2);
			});
			MasterPubkey::<T>::kill();

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
			Self::verify_signature(&signature, &payload.encode(), &payload.pubkey)?;
			// Validate the signing time: 10 minutes expiration
			Self::verify_signing_time(payload.signing_time, 10 * 60)?;
			// Validate the public key
			ensure!(Workers::<T>::contains_key(payload.pubkey), Error::<T>::InvalidWorkerPubKey);

			let applier = &payload.pubkey;
			//TODO: round by origin nounce
			let distributor = {
				let mut i = Workers::<T>::iter();
				loop {
					let (worker_pubkey, _) = i.next().ok_or(Into::<DispatchError>::into(Error::<T>::WorkerNotFound))?; //TODO: introduce a new error
					if *applier == worker_pubkey {
						continue;
					}
					break worker_pubkey;
				}
			};

			MasterKeyPostation::<T>::insert(applier, None::<(BlockNumberFor<T>, MasterKeyDistributePayload)>);
			MasterKeyDistributeNotify::<T>::insert(distributor, applier.clone());

			Self::deposit_event(Event::<T>::MasterKeyAppling { applier: *applier, distributor });
			Ok(())
		}

		#[pallet::call_index(30)]
		#[pallet::weight({0})]
		pub fn distribute_master_key(
			origin: OriginFor<T>,
			payload: MasterKeyDistributePayload,
			signature: Vec<u8>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			// Validate the signature
			Self::verify_signature(&signature, &payload.encode(), &payload.distributor)?;
			// Validate the signing time: 10 minutes expiration
			Self::verify_signing_time(payload.signing_time, 10 * 60)?;
			let receiver = payload.target.clone();
			let distributor = payload.distributor.clone();
			let bn = <frame_system::Pallet<T>>::block_number();
			MasterKeyPostation::<T>::insert(&receiver, Some((bn, payload)));
			MasterKeyDistributeNotify::<T>::remove(distributor);
			Self::deposit_event(Event::<T>::MasterKeySubmitted { receiver });
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
			ensure!(!allowlist.contains(&ceseal_hash), Error::<T>::CesealBinAlreadyExists);

			allowlist.push(ceseal_hash.clone());
			CesealBinAllowList::<T>::put(allowlist);

			let now = frame_system::Pallet::<T>::block_number();
			CesealBinAddedAt::<T>::insert(&ceseal_hash, now);

			Self::deposit_event(Event::<T>::CesealBinAdded(ceseal_hash));
			Ok(())
		}

		#[pallet::call_index(22)]
		#[pallet::weight({0})]
		pub fn change_first_holder(origin: OriginFor<T>, pubkey: WorkerPublicKey) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			MasterKeyFirstHolder::<T>::try_mutate(|first_key_opt| -> DispatchResult {
				let first_key = first_key_opt.as_mut().ok_or(Error::<T>::MasterKeyFirstHolderNotFound)?;
				*first_key = pubkey;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::ChangeFirstHolder { pubkey });
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

			Self::deposit_event(Event::<T>::CesealBinRemoved(ceseal_hash));
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

			let now = <frame_system::Pallet<T>>::block_number();
			let _ = Self::clear_mission(now);

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

		#[pallet::call_index(117)]
		#[pallet::weight({0})]
		pub fn force_clear_tee(origin: OriginFor<T>, puk: WorkerPublicKey) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			Self::execute_exit(puk)?;

			Ok(())
		}

		#[pallet::call_index(118)]
		#[pallet::weight({0})]
		pub fn set_none_attestation_enabled(origin: OriginFor<T>, enabled: bool) -> DispatchResult {
			ensure_root(origin)?;
			NoneAttestationEnabled::<T>::put(enabled);
			Ok(())
		}

		#[pallet::call_index(119)]
		#[pallet::weight({0})]
		pub fn set_ceseal_verify_required(origin: OriginFor<T>, required: bool) -> DispatchResult {
			ensure_root(origin)?;
			CesealVerifyRequired::<T>::put(required);
			Ok(())
		}
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
		pub endpoint: String,
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
				AttestationError::UnsupportedAttestationType => Self::UnsupportedAttestationType,
				AttestationError::InvalidDCAPQuote(attestation_error) => match attestation_error {
					sgx_attestation::Error::InvalidCertificate => Self::InvalidCertificate,
					sgx_attestation::Error::InvalidSignature => Self::InvalidSignature,
					sgx_attestation::Error::CodecError => Self::CodecError,
					sgx_attestation::Error::TCBInfoExpired => Self::TCBInfoExpired,
					sgx_attestation::Error::KeyLengthIsInvalid => Self::KeyLengthIsInvalid,
					sgx_attestation::Error::PublicKeyIsInvalid => Self::PublicKeyIsInvalid,
					sgx_attestation::Error::RsaSignatureIsInvalid => Self::RsaSignatureIsInvalid,
					sgx_attestation::Error::DerEncodingError => Self::DerEncodingError,
					sgx_attestation::Error::UnsupportedDCAPQuoteVersion => Self::UnsupportedDCAPQuoteVersion,
					sgx_attestation::Error::UnsupportedDCAPAttestationKeyType => {
						Self::UnsupportedDCAPAttestationKeyType
					},
					sgx_attestation::Error::UnsupportedQuoteAuthData => Self::UnsupportedQuoteAuthData,
					sgx_attestation::Error::UnsupportedDCAPPckCertFormat => Self::UnsupportedDCAPPckCertFormat,
					sgx_attestation::Error::LeafCertificateParsingError => Self::LeafCertificateParsingError,
					sgx_attestation::Error::CertificateChainIsInvalid => Self::CertificateChainIsInvalid,
					sgx_attestation::Error::CertificateChainIsTooShort => Self::CertificateChainIsTooShort,
					sgx_attestation::Error::IntelExtensionCertificateDecodingError => {
						Self::IntelExtensionCertificateDecodingError
					},
					sgx_attestation::Error::IntelExtensionAmbiguity => Self::IntelExtensionAmbiguity,
					sgx_attestation::Error::CpuSvnLengthMismatch => Self::CpuSvnLengthMismatch,
					sgx_attestation::Error::CpuSvnDecodingError => Self::CpuSvnDecodingError,
					sgx_attestation::Error::PceSvnDecodingError => Self::PceSvnDecodingError,
					sgx_attestation::Error::PceSvnLengthMismatch => Self::PceSvnLengthMismatch,
					sgx_attestation::Error::FmspcLengthMismatch => Self::FmspcLengthMismatch,
					sgx_attestation::Error::FmspcDecodingError => Self::FmspcDecodingError,
					sgx_attestation::Error::FmspcMismatch => Self::FmspcMismatch,
					sgx_attestation::Error::QEReportHashMismatch => Self::QEReportHashMismatch,
					sgx_attestation::Error::IsvEnclaveReportSignatureIsInvalid => {
						Self::IsvEnclaveReportSignatureIsInvalid
					},
					sgx_attestation::Error::DerDecodingError => Self::DerDecodingError,
					sgx_attestation::Error::OidIsMissing => Self::OidIsMissing,
				},
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
	fn update_work_block(now: Block, pbk: &WorkerPublicKey) -> DispatchResult;
	fn verify_master_sig(sig: &sp_core::sr25519::Signature, hash: SHA256) -> bool;
}

impl<T: Config> TeeWorkerHandler<AccountOf<T>, BlockNumberFor<T>> for Pallet<T> {
	fn can_tag(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			if WorkerRole::Marker == tee_info.role || WorkerRole::Full == tee_info.role {
				return true;
			}
		}

		false
	}

	fn can_verify(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			if WorkerRole::Verifier == tee_info.role || WorkerRole::Full == tee_info.role {
				return true;
			}
		}

		false
	}

	fn can_cert(pbk: &WorkerPublicKey) -> bool {
		if let Ok(tee_info) = Workers::<T>::try_get(pbk) {
			if WorkerRole::Marker == tee_info.role || WorkerRole::Full == tee_info.role {
				return true;
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
			return result;
		}

		false
	}

	fn get_stash(pbk: &WorkerPublicKey) -> Result<AccountOf<T>, DispatchError> {
		let tee_info = Workers::<T>::try_get(pbk).map_err(|_| Error::<T>::NonTeeWorker)?;

		if let Some(stash_account) = tee_info.stash_account {
			return Ok(stash_account);
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

	fn verify_master_sig(sig: &sp_core::sr25519::Signature, hash: SHA256) -> bool {
		let cur_pubkey = match <MasterPubkey<T>>::try_get().map_err(|_| Error::<T>::MasterKeyUninitialized) {
			Ok(cur_pubkey) => cur_pubkey,
			Err(_) => return false,
		};

		let mut flag = false;
		let pubkey_list = OldMasterPubkeyList::<T>::get();
		for pubkey in pubkey_list {
			if sp_io::crypto::sr25519_verify(&sig, &hash, &pubkey) {
				flag = true;
				break;
			}
		}

		if sp_io::crypto::sr25519_verify(&sig, &hash, &cur_pubkey) || flag {
			return true;
		}

		false
	}

	fn update_work_block(now: BlockNumberFor<T>, pbk: &WorkerPublicKey) -> DispatchResult {
		<LastWork<T>>::try_mutate(pbk, |last_block| {
			*last_block = now;

			Ok(())
		})
	}
}
