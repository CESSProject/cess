//! # Tee Worker Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod testing_utils;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult, traits::ReservableCurrency, transactional, BoundedVec, PalletId,
};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{DispatchError, RuntimeDebug};
use sp_std::{ convert::TryInto, prelude::* };
use cp_scheduler_credit::SchedulerCreditCounter;
pub use weights::WeightInfo;
use cp_cess_common::*;

pub mod weights;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
		traits::Get,
		Blake2_128Concat,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct SchedulerInfo<T: pallet::Config> {
		pub ip: IpAddress,
		pub stash_user: AccountOf<T>,
		pub controller_user: AccountOf<T>,
	}

	#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct PublicKey<T: Config> {
		pub spk: [u8; 128],
		pub shared_params: BoundedVec<u8, T::ParamsLimit>,
		pub shared_g: [u8; 128],
	}

	#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct ExceptionReport<T: pallet::Config> {
		count: u32,
		reporters: BoundedVec<AccountOf<T>, T::StringLimit>,
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
		type StringLimit: Get<u32> + PartialEq + Eq + Clone;

		#[pallet::constant]
		type ParamsLimit: Get<u32> + PartialEq + Eq + Clone;

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
		RegistrationScheduler { acc: AccountOf<T>, ip: IpAddress },

		UpdateScheduler { acc: AccountOf<T>, endpoint: IpAddress },
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
	}

	#[pallet::storage]
	#[pallet::getter(fn scheduler_map)]
	pub(super) type SchedulerMap<T: Config> =
		StorageValue<_, BoundedVec<SchedulerInfo<T>, T::SchedulerMaximum>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn scheduler_exception)]
	pub(super) type SchedulerException<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, ExceptionReport<T>>;

	#[pallet::storage]
	#[pallet::getter(fn bond_acc)]
	pub(super) type BondAcc<T: Config> =
		StorageValue<_, BoundedVec<AccountOf<T>, T::SchedulerMaximum>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn mr_enclave_whitelist)]
	pub(super) type MrEnclaveWhitelist<T: Config> = StorageValue<_, BoundedVec<[u8; 64], T::MaxWhitelist>, ValueQuery>;
	

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		//Scheduling registration method
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::registration_scheduler())]
		pub fn registration_scheduler(
			origin: OriginFor<T>,
			stash_account: AccountOf<T>,
			ip: IpAddress,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//Even if the primary key is not present here, panic will not be caused
			let acc = <pallet_cess_staking::Pallet<T>>::bonded(&stash_account)
				.ok_or(Error::<T>::NotBond)?;
			if sender != acc {
				Err(Error::<T>::NotController)?;
			}
			let mut s_vec = SchedulerMap::<T>::get();
			let scheduler = SchedulerInfo::<T> {
				ip: ip.clone(),
				stash_user: stash_account.clone(),
				controller_user: sender.clone(),
			};

			if s_vec.to_vec().contains(&scheduler) {
				Err(Error::<T>::AlreadyRegistration)?;
			}
			s_vec.try_push(scheduler).map_err(|_e| Error::<T>::StorageLimitReached)?;
			SchedulerMap::<T>::put(s_vec);
			Self::deposit_event(Event::<T>::RegistrationScheduler { acc: sender, ip });
			Ok(())
		}

		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_scheduler())]
		pub fn update_scheduler(origin: OriginFor<T>, ip: IpAddress) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			SchedulerMap::<T>::try_mutate(|s| -> DispatchResult {
				let mut count = 0;
				for i in s.iter() {
					if i.controller_user == sender {
						let scheduler = SchedulerInfo::<T> {
							ip: ip.clone(),
							stash_user: i.stash_user.clone(),
							controller_user: sender.clone(),
						};
						s.remove(count);
						s.try_push(scheduler).map_err(|_| Error::<T>::StorageLimitReached)?;
						return Ok(())
					}
					count = count.checked_add(1).ok_or(Error::<T>::Overflow)?;
				}
				Err(Error::<T>::NotController)?
			})?;
			Self::deposit_event(Event::<T>::UpdateScheduler { acc: sender, endpoint: ip });
			Ok(())
		}
		
        #[pallet::call_index(3)]
        #[transactional]
		#[pallet::weight(100_000_000)]
        pub fn update_whitelist(origin: OriginFor<T>, mr_enclave: [u8; 64]) -> DispatchResult {
			let _ = ensure_root(origin)?;
			<MrEnclaveWhitelist<T>>::mutate(|list| -> DispatchResult {
                list.try_push(mr_enclave).unwrap();
                Ok(())
            })?;

			Ok(())
		}
	}
}

pub trait ScheduleFind<AccountId> {
	fn contains_scheduler(acc: AccountId) -> bool;
	fn get_controller_acc(acc: AccountId) -> AccountId;
	fn punish_scheduler(acc: AccountId) -> DispatchResult;
	fn get_first_controller() -> Result<AccountId, DispatchError>;
}

impl<T: Config> ScheduleFind<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn contains_scheduler(acc: <T as frame_system::Config>::AccountId) -> bool {
		for i in <SchedulerMap<T>>::get().to_vec().iter() {
			if i.controller_user == acc {
				return true
			}
		}
		false
	}

	fn get_controller_acc(
		acc: <T as frame_system::Config>::AccountId,
	) -> <T as frame_system::Config>::AccountId {
		let scheduler_list = SchedulerMap::<T>::get();
		for v in scheduler_list {
			if v.stash_user == acc {
				return v.controller_user
			}
		}
		acc
	}

	fn punish_scheduler(acc: <T as frame_system::Config>::AccountId) -> DispatchResult {
		let scheduler_list = SchedulerMap::<T>::get();
		for v in scheduler_list {
			if v.controller_user == acc {
				pallet_cess_staking::slashing::slash_scheduler::<T>(&v.stash_user);
				T::CreditCounter::record_punishment(&v.controller_user)?;
			}
		}
		Ok(())
	}

	fn get_first_controller() -> Result<<T as frame_system::Config>::AccountId, DispatchError> {
		let s_vec = SchedulerMap::<T>::get();
		if s_vec.len() > 0 {
			return Ok(s_vec[0].clone().controller_user)
		}
		Err(Error::<T>::Overflow)?
	}
}
