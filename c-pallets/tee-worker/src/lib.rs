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
	dispatch::DispatchResult, traits::ReservableCurrency, transactional, BoundedVec, PalletId,
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
use cp_enclave_verify::*;
pub mod weights;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
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
		NotController,
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
			stash_account: AccountOf<T>,
			peer_id: PeerId,
			podr2_pbk: Podr2Key,
			sgx_attestation_report: SgxAttestationReport,
			end_point: EndPoint,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//Even if the primary key is not present here, panic will not be caused
			let acc = <pallet_cess_staking::Pallet<T>>::bonded(&stash_account)
				.ok_or(Error::<T>::NotBond)?;
			if sender != acc {
				Err(Error::<T>::NotController)?;
			}
			ensure!(!TeeWorkerMap::<T>::contains_key(&sender), Error::<T>::AlreadyRegistration);
			let mut identity = Vec::new();
			identity.append(&mut peer_id.to_vec());
			identity.append(&mut podr2_pbk.to_vec());
			identity.append(&mut end_point.to_vec());
			let identity_hashing = sp_io::hashing::sha2_256(&identity);
			let _ = verify_miner_cert(
				&sgx_attestation_report.sign,
				&sgx_attestation_report.cert_der,
				&sgx_attestation_report.report_json_raw,
				&identity_hashing,
			).ok_or(Error::<T>::VerifyCertFailed)?;

			let tee_worker_info = TeeWorkerInfo::<T> {
				controller_account: sender.clone(),
				peer_id: peer_id.clone(),
				stash_account: stash_account,
				end_point,
			};

			if TeeWorkerMap::<T>::count() == 0 {
				<TeePodr2Pk<T>>::put(podr2_pbk);
			}

			TeeWorkerMap::<T>::insert(&sender, tee_worker_info);

			Self::deposit_event(Event::<T>::RegistrationTeeWorker { acc: sender, peer_id: peer_id });

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
			stash_account: AccountOf<T>,
			controller_account: AccountOf<T>,
			peer_id: PeerId,
			end_point: EndPoint,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let tee_worker_info = TeeWorkerInfo::<T> {
				controller_account: controller_account.clone(),
				peer_id: peer_id.clone(),
				stash_account: stash_account,
				end_point,
			};

			TeeWorkerMap::<T>::insert(&controller_account, tee_worker_info);
			
			Ok(())
		}
	}
}

pub trait TeeWorkerHandler<AccountId> {
	fn contains_scheduler(acc: AccountId) -> bool;
	fn punish_scheduler(acc: AccountId) -> DispatchResult;
	fn get_first_controller() -> Result<AccountId, DispatchError>;
	fn get_controller_list() -> Vec<AccountId>;
	fn get_tee_publickey() -> Result<Podr2Key, DispatchError>;
}

impl<T: Config> TeeWorkerHandler<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn contains_scheduler(acc: <T as frame_system::Config>::AccountId) -> bool {
		TeeWorkerMap::<T>::contains_key(&acc)
	}

	fn punish_scheduler(acc: <T as frame_system::Config>::AccountId) -> DispatchResult {
		let tee_worker = TeeWorkerMap::<T>::try_get(&acc).map_err(|_| Error::<T>::NonTeeWorker)?;
		pallet_cess_staking::slashing::slash_scheduler::<T>(&tee_worker.stash_account);
		T::CreditCounter::record_punishment(&tee_worker.stash_account)?;

		Ok(())
	}

	fn get_first_controller() -> Result<<T as frame_system::Config>::AccountId, DispatchError> {
		let (controller_acc, _) = TeeWorkerMap::<T>::iter().next().ok_or(Error::<T>::NonTeeWorker)?;
		return Ok(controller_acc);
	}

	fn get_controller_list() -> Vec<AccountOf<T>> {
		let mut acc_list: Vec<AccountOf<T>> = Default::default();

		for (acc, _) in <TeeWorkerMap<T>>::iter() {
			acc_list.push(acc);
		}

		acc_list
	}

	fn get_tee_publickey() -> Result<Podr2Key, DispatchError> {
		let pk = TeePodr2Pk::<T>::try_get().map_err(|_| Error::<T>::TeePodr2PkNotInitialized)?;

		Ok(pk)
	}
}
