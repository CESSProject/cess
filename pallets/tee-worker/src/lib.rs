//! # Tee Worker Module
#![cfg_attr(not(feature = "std"), no_std)]



#[cfg(test)]
mod tests;

mod mock;
mod attestation;
mod types;
pub use types::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult, traits::{ReservableCurrency, UnixTime}, transactional, BoundedVec, PalletId,
	pallet_prelude::*,
};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	DispatchError, RuntimeDebug,
};
use sp_std::{ 
	convert::TryInto,
	prelude::*,
};

use cp_scheduler_credit::SchedulerCreditCounter;
pub use weights::WeightInfo;
use cp_cess_common::*;
use frame_system::{ensure_signed, pallet_prelude::*};
pub mod weights;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use attestation::{Error as AttestationError, Attestation, AttestationValidator, IasValidator};
	use frame_support::{
		traits::Get,
		Blake2_128Concat,
	};

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
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//Scheduling registration method
		RegistrationTeeWorker { acc: AccountOf<T>, peer_id: PeerId },

		Exit { acc: AccountOf<T> },

		UpdatePeerId { acc: AccountOf<T> },
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
	}

	#[pallet::storage]
	#[pallet::getter(fn tee_worker_map)]
	pub(super) type TeeWorkerMap<T: Config> = CountedStorageMap<_, Blake2_128Concat, AccountOf<T>, TeeWorkerInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn bond_acc)]
	pub(super) type BondAcc<T: Config> =
		StorageValue<_, BoundedVec<AccountOf<T>, T::SchedulerMaximum>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn tee_podr2_pk)]
	pub(super) type TeePodr2Pk<T: Config> = StorageValue<_, Podr2Key>;

	#[pallet::storage]
	#[pallet::getter(fn mr_enclave_whitelist)]
	pub(super) type MrEnclaveWhitelist<T: Config> = StorageValue<_, BoundedVec<[u8; 64], T::MaxWhitelist>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn validation_type_list)]
	pub(super) type ValidationTypeList<T: Config> = StorageValue<_, BoundedVec<AccountOf<T>, T::SchedulerMaximum>, ValueQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a TEE Worker
		///
		/// This function allows a user to register a Trusted Execution Environment (TEE) worker by providing necessary information,
		/// including the TEE worker's public keys, Peer ID, and other details.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, representing the user's account.
		/// - `stash_account`: The stash account associated with the TEE worker, used for staking and governance.
		/// - `node_key`: The public key of the TEE node.
		/// - `peer_id`: The Peer ID of the TEE worker.
		/// - `podr2_pbk`: The public key used for Proofs of Data Replication 2 (PoDR2) operations.
		/// - `sgx_attestation_report`: The attestation report from the Intel Software Guard Extensions (SGX) enclave.
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::registration_scheduler())]
		pub fn register(
			origin: OriginFor<T>,
			worker_account: AccountOf<T>,
			stash_account: Option<AccountOf<T>>,
			peer_id: PeerId,
			podr2_pbk: Podr2Key,
			sgx_attestation_report: SgxAttestationReport,
			end_point: EndPoint,
			tee_type: TeeType,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//Even if the primary key is not present here, panic will not be caused
			match &stash_account {
				Some(acc) => {
					let _ = <pallet_cess_staking::Pallet<T>>::bonded(acc).ok_or(Error::<T>::NotBond)?;
					ensure!(tee_type == TeeType::Verifier || tee_type == TeeType::Full, Error::<T>::WrongTeeType);
				},
				None => ensure!(tee_type == TeeType::Marker, Error::<T>::WrongTeeType),
			};

			ensure!(!TeeWorkerMap::<T>::contains_key(&worker_account), Error::<T>::AlreadyRegistration);

			let attestation = Attestation::SgxIas {
				ra_report: sgx_attestation_report.report_json_raw.to_vec(),
				signature: base64::decode(sgx_attestation_report.sign)
					.map_err(|_|Error::<T>::InvalidReport)?,
				raw_signing_cert: base64::decode(sgx_attestation_report.cert_der)
					.map_err(|_|Error::<T>::InvalidReport)?,
			};
			
			// Validate RA report & embedded user data
			let worker_account_encode = worker_account.encode();
			let mut identity = Vec::new();
			identity.extend_from_slice(&peer_id);
			identity.extend_from_slice(&podr2_pbk);
			identity.extend_from_slice(&end_point);
			identity.extend_from_slice(&worker_account_encode);
			let now = T::UnixTime::now().as_secs();

			let _ = IasValidator::validate(
				&attestation,
				&sp_io::hashing::sha2_256(&identity),
				now,
				false,  //TODO: to implement the ceseal verify
				vec![]
			).map_err(Into::<Error<T>>::into)?;

			let tee_worker_info = TeeWorkerInfo::<T> {
				worker_account: worker_account.clone(),
				peer_id: peer_id.clone(),
				bond_stash: stash_account,
				end_point,
				tee_type: tee_type.clone(),
			};

			if tee_type == TeeType::Full || tee_type == TeeType::Verifier {
				ValidationTypeList::<T>::mutate(|tee_list| -> DispatchResult {
					tee_list.try_push(worker_account.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			}

			if TeeWorkerMap::<T>::count() == 0 {
				<TeePodr2Pk<T>>::put(podr2_pbk);
			}

			TeeWorkerMap::<T>::insert(&worker_account, tee_worker_info);

			Self::deposit_event(Event::<T>::RegistrationTeeWorker { acc: worker_account, peer_id: peer_id });

			Ok(())
		}

		/// Update the TEE Worker MR Enclave Whitelist
		///
		/// This function allows the root or superuser to update the whitelist of Trusted Execution Environment (TEE) Worker MR (Measurement and Report) Enclaves. 
		/// Each MR Enclave represents a specific instance of a TEE worker. By adding an MR Enclave to the whitelist, 
		/// the user ensures that the associated TEE worker can participate in network activities.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, representing the user's account. Only the root user is authorized to call this function.
		/// - `mr_enclave`: A fixed-size array of 64 bytes representing the MR Enclave of the TEE worker to be added to the whitelist.
        #[pallet::call_index(1)]
        #[transactional]
		#[pallet::weight(Weight::zero())]
        pub fn update_whitelist(origin: OriginFor<T>, mr_enclave: [u8; 64]) -> DispatchResult {
			let _ = ensure_root(origin)?;
			<MrEnclaveWhitelist<T>>::mutate(|list| -> DispatchResult {
				ensure!(!list.contains(&mr_enclave), Error::<T>::Existed);
                list.try_push(mr_enclave).unwrap();
                Ok(())
            })?;

			Ok(())
		}

		/// Exit a TEE Worker from the Network
		///
		/// This function allows a TEE (Trusted Execution Environment) Worker to voluntarily exit from the network. 
		/// When a TEE Worker exits, it will no longer participate in network activities and will be removed from the list of active TEE Workers.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, representing the account of the TEE Worker. This should be the controller account of the TEE Worker.
		#[pallet::call_index(2)]
        #[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn exit(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let tee_info = TeeWorkerMap::<T>::try_get(&sender).map_err(|_| Error::<T>::Overflow)?;
			if tee_info.tee_type == TeeType::Full || tee_info.tee_type == TeeType::Verifier {
				ValidationTypeList::<T>::mutate(|tee_list| {
					tee_list.retain(|tee_acc| *tee_acc != sender.clone());
				});
			}

			TeeWorkerMap::<T>::remove(&sender);

			if TeeWorkerMap::<T>::count() == 0 {
				<TeePodr2Pk<T>>::kill();
			}

			Self::deposit_event(Event::<T>::Exit { acc: sender });

			Ok(())
		}
		// FOR TESTING
		#[pallet::call_index(3)]
        #[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn update_podr2_pk(origin: OriginFor<T>, podr2_pbk: Podr2Key) -> DispatchResult {
			let _sender = ensure_root(origin)?;

			<TeePodr2Pk<T>>::put(podr2_pbk);

			Ok(())
		}

		// FOR TESTING
		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::registration_scheduler())]
		pub fn force_register(
			origin: OriginFor<T>,
			stash_account: Option<AccountOf<T>>,
			controller_account: AccountOf<T>,
			peer_id: PeerId,
			end_point: EndPoint,
			tee_type: TeeType,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let tee_worker_info = TeeWorkerInfo::<T> {
				worker_account: controller_account.clone(),
				peer_id: peer_id.clone(),
				bond_stash: stash_account,
				end_point,
				tee_type: tee_type,
			};

			TeeWorkerMap::<T>::insert(&controller_account, tee_worker_info);
			
			Ok(())
		}
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
	fn can_tag(acc: &AccountId) -> bool;
	fn can_verify(acc: &AccountId) -> bool;
	fn can_cert(acc: &AccountId) -> bool;
	fn contains_scheduler(acc: AccountId) -> bool;
	fn is_bonded(acc: &AccountId) -> bool;
	fn get_stash(acc: &AccountId) -> Result<AccountId, DispatchError>;
	fn punish_scheduler(acc: AccountId) -> DispatchResult;
	fn get_controller_list() -> Vec<AccountId>;
	fn get_tee_publickey() -> Result<Podr2Key, DispatchError>;
}

impl<T: Config> TeeWorkerHandler<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn can_tag(acc: &AccountOf<T>) -> bool {
		if let Ok(tee_info) = TeeWorkerMap::<T>::try_get(&acc) {
			if TeeType::Marker == tee_info.tee_type || TeeType::Full == tee_info.tee_type {
				return true;
			}
		}

		false
	}

	fn can_verify(acc: &AccountOf<T>) -> bool {
		if let Ok(tee_info) = TeeWorkerMap::<T>::try_get(acc) {
			if TeeType::Verifier == tee_info.tee_type || TeeType::Full == tee_info.tee_type {
				return true;
			}
		}

		false
	}

	fn can_cert(acc: &AccountOf<T>) -> bool {
		if let Ok(tee_info) = TeeWorkerMap::<T>::try_get(acc) {
			if TeeType::Marker == tee_info.tee_type || TeeType::Full == tee_info.tee_type {
				return true;
			}
		}

		false
	}

	fn contains_scheduler(acc: <T as frame_system::Config>::AccountId) -> bool {
		TeeWorkerMap::<T>::contains_key(&acc)
	}

	fn is_bonded(acc: &AccountOf<T>) -> bool {
		if let Ok(tee_info) = TeeWorkerMap::<T>::try_get(acc) {
			let result = tee_info.bond_stash.is_some();
			return result;
		}

		false
	}

	fn get_stash(acc: &AccountOf<T>) -> Result<AccountOf<T>, DispatchError> {
		let tee_info = TeeWorkerMap::<T>::try_get(acc).map_err(|_| Error::<T>::NonTeeWorker)?;

		if let Some(bond_stash) = tee_info.bond_stash {
			return Ok(bond_stash)
		}

		Err(Error::<T>::NonTeeWorker.into())
	}

	fn punish_scheduler(acc: <T as frame_system::Config>::AccountId) -> DispatchResult {
		let tee_worker = TeeWorkerMap::<T>::try_get(&acc).map_err(|_| Error::<T>::NonTeeWorker)?;
		if let Some(stash_account) = tee_worker.bond_stash {
			pallet_cess_staking::slashing::slash_scheduler::<T>(&stash_account);
			T::CreditCounter::record_punishment(&stash_account)?;
		}

		Ok(())
	}

	fn get_controller_list() -> Vec<AccountOf<T>> {
		let acc_list = <ValidationTypeList<T>>::get().to_vec();

		acc_list
	}

	fn get_tee_publickey() -> Result<Podr2Key, DispatchError> {
		let pk = TeePodr2Pk::<T>::try_get().map_err(|_| Error::<T>::TeePodr2PkNotInitialized)?;

		Ok(pk)
	}
}
