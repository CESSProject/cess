//! # File Map Module

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::{Decode, Encode};
use frame_support::{dispatch::DispatchResult, traits::ReservableCurrency, BoundedVec, PalletId};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{traits::SaturatedConversion, RuntimeDebug};
use sp_std::prelude::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{pallet_prelude::{*, ValueQuery}, traits::Get, Blake2_128Concat};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct SchedulerInfo<T: pallet::Config> {
		pub ip: BoundedVec<u8, T::StringLimit>,
		pub stash_user: AccountOf<T>,
		pub controller_user: AccountOf<T>,
	}

	#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct PublicKey<T: Config> {
		pub spk: BoundedVec<u8, T::StringLimit>,
		pub shared_params: BoundedVec<u8, T::StringLimit>,
		pub shared_g: BoundedVec<u8, T::StringLimit>,
	}

	#[derive(
		PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo,
	)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct ExceptionReport<T: pallet::Config> {
		count: u32,
		reporters: BoundedVec<AccountOf<T>, T::StringLimit>,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_cess_staking::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// pallet address.
		#[pallet::constant]
		type FileMapPalletId: Get<PalletId>;

		#[pallet::constant]
		type StringLimit: Get<u32> + PartialEq + Eq + Clone;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//Scheduling registration method
		RegistrationScheduler { acc: AccountOf<T>, ip: Vec<u8> },
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
		StorageValue<_, BoundedVec<SchedulerInfo<T>, T::StringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn scheduler_exception)]
	pub(super) type SchedulerException<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, ExceptionReport<T>>;

	#[pallet::storage]
	#[pallet::getter(fn scheduler_puk)]
	pub(super) type SchedulerPuk<T: Config> = StorageValue<_, PublicKey<T>>;

	#[pallet::storage]
	#[pallet::getter(fn bond_acc)]
	pub(super) type BondAcc<T: Config> = StorageValue<_, BoundedVec<AccountOf<T>, T::StringLimit>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		//Polling exception report
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let number: u128 = now.saturated_into();
			let count: usize = Self::scheduler_map().len();
			if number % 1200 == 0 {
				let alregister_list = SchedulerMap::<T>::get();
				let mut alregister_controller_list: BoundedVec<AccountOf<T>, T::StringLimit> = Default::default();
				for alregister in alregister_list.iter() {
					if let Err(e) =
						alregister_controller_list.try_push(alregister.controller_user.clone()).map_err(|_| Error::<T>::StorageLimitReached)
					{
						log::error!("FileMap error: {:?}", e);
					}
				}

				for v in BondAcc::<T>::get().iter() {
					if !alregister_controller_list.contains(&v) {
						for v2 in alregister_list.iter() {
							if v2.controller_user == v.clone() {
								pallet_cess_staking::slashing::slash_scheduler::<T>(&v2.stash_user);
							}
						}
					}
				}

				let mut ctl: BoundedVec<AccountOf<T>, T::StringLimit> = Default::default();
				for (_stash, controller) in pallet_cess_staking::Bonded::<T>::iter() {
					if let Err(e) = 
						ctl.try_push(controller.clone()).map_err(|_| Error::<T>::StorageLimitReached)
					{
						log::error!("FileMap error: {:?}", e);
					}
				}
				BondAcc::<T>::put(ctl);
				for (key, value) in <SchedulerException<T>>::iter() {
					if value.count > (count / 2) as u32 {
						pallet_cess_staking::slashing::slash_scheduler::<T>(&key);
					}

					<SchedulerException<T>>::remove(key);
				}
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		//Scheduling registration method
		#[pallet::weight(1_000_000)]
		pub fn registration_scheduler(
			origin: OriginFor<T>,
			stash_account: AccountOf<T>,
			ip: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//Even if the primary key is not present here, panic will not be caused
			let acc = <pallet_cess_staking::Pallet<T>>::bonded(&stash_account).ok_or(Error::<T>::NotBond)?;
			if sender != acc {
				Err(Error::<T>::NotController)?;
			}
			let mut s_vec = SchedulerMap::<T>::get();
			let ip_bound = ip.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			let scheduler = SchedulerInfo::<T> {
				ip: ip_bound,
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

		#[pallet::weight(1_000)]
		pub fn update_scheduler(
			origin: OriginFor<T>,
			ip: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			SchedulerMap::<T>::try_mutate(|s| -> DispatchResult {
				let mut count = 0;
				for i in s.iter() {
					if i.controller_user == sender {
						let scheduler = SchedulerInfo::<T> {
							ip: ip.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
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
		
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn scheduler_exception_report(
			origin: OriginFor<T>,
			account: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if !<SchedulerException<T>>::contains_key(&account) {
				<SchedulerException<T>>::insert(
					&account,
					ExceptionReport::<T> { count: 0, reporters: Default::default() },
				);
			}

			<SchedulerException<T>>::try_mutate(&account, |opt| -> DispatchResult {
				let o = opt.as_mut().unwrap();
				for value in &o.reporters.to_vec() {
					if &sender == value {
						Err(Error::<T>::AlreadyReport)?;
					}
				}
				o.count = o.count.checked_add(1).ok_or(Error::<T>::Overflow)?;
				o.reporters
					.try_push(account.clone())
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn init_public_key(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let spk: BoundedVec<u8, T::StringLimit> = vec![
				10, 220, 75, 195, 174, 36, 186, 176, 59, 223, 170, 199, 177, 143, 223, 147, 220,
				84, 132, 101, 54, 112, 120, 144, 219, 28, 230, 129, 240, 127, 161, 4, 193, 25, 118,
				181, 98, 3, 34, 200, 217, 50, 125, 125, 26, 120, 139, 11, 63, 0, 223, 99, 217, 72,
				24, 157, 225, 79, 157, 168, 219, 170, 73, 134, 74, 223, 196, 139, 171, 223, 110,
				21, 54, 36, 247, 187, 95, 40, 251, 4, 11, 92, 93, 105, 206, 67, 21, 31, 255, 227,
				9, 166, 11, 194, 117, 81, 227, 225, 25, 170, 140, 120, 254, 100, 174, 110, 180,
				158, 45, 0, 197, 150, 193, 71, 30, 34, 233, 90, 5, 64, 37, 163, 246, 121, 176, 26,
				201, 174,
			]
			.try_into()
			.map_err(|_e| Error::<T>::BoundedVecError)?;
			let shared_params: BoundedVec<u8, T::StringLimit> = vec![
				116, 121, 112, 101, 32, 97, 10, 113, 32, 54, 53, 56, 50, 55, 51, 49, 54, 52, 56,
				53, 50, 52, 55, 48, 50, 57, 55, 56, 52, 54, 54, 48, 54, 50, 53, 51, 48, 57, 53, 56,
				57, 50, 49, 51, 56, 53, 57, 50, 55, 57, 54, 55, 48, 54, 55, 48, 50, 51, 56, 48, 53,
				51, 55, 51, 53, 49, 49, 51, 51, 51, 55, 55, 51, 56, 51, 57, 49, 55, 57, 49, 55, 56,
				56, 53, 54, 48, 52, 49, 55, 51, 51, 54, 48, 51, 57, 55, 56, 51, 50, 55, 51, 48, 50,
				48, 56, 54, 49, 57, 52, 50, 56, 49, 56, 53, 50, 51, 49, 48, 49, 57, 56, 54, 48, 52,
				48, 55, 48, 48, 48, 50, 51, 55, 49, 57, 54, 57, 56, 50, 55, 50, 56, 57, 50, 49, 56,
				57, 53, 53, 55, 56, 52, 57, 49, 52, 51, 57, 51, 49, 52, 56, 48, 56, 51, 10, 104,
				32, 57, 48, 48, 56, 49, 55, 53, 53, 51, 55, 50, 52, 54, 49, 49, 52, 54, 50, 53, 55,
				51, 55, 56, 56, 57, 52, 50, 57, 52, 53, 53, 49, 57, 48, 57, 48, 57, 48, 48, 49, 52,
				53, 48, 49, 55, 51, 56, 54, 52, 49, 54, 56, 54, 56, 52, 48, 48, 53, 54, 53, 50, 54,
				55, 52, 48, 49, 49, 56, 49, 51, 55, 54, 51, 56, 57, 49, 56, 57, 49, 56, 57, 55, 55,
				54, 49, 55, 50, 55, 49, 52, 53, 56, 53, 56, 55, 55, 50, 49, 56, 54, 49, 50, 53, 54,
				52, 52, 10, 114, 32, 55, 51, 48, 55, 53, 48, 56, 49, 56, 54, 54, 53, 52, 53, 50,
				55, 53, 55, 49, 55, 54, 48, 53, 55, 48, 53, 48, 48, 54, 53, 48, 52, 56, 54, 52, 50,
				52, 53, 50, 48, 52, 56, 53, 55, 54, 53, 49, 49, 10, 101, 120, 112, 50, 32, 49, 53,
				57, 10, 101, 120, 112, 49, 32, 49, 49, 48, 10, 115, 105, 103, 110, 49, 32, 49, 10,
				115, 105, 103, 110, 48, 32, 45, 49, 10,
			]
			.try_into()
			.map_err(|_e| Error::<T>::BoundedVecError)?;
			let shared_g: BoundedVec<u8, T::StringLimit> = vec![
				6, 82, 21, 158, 104, 141, 100, 78, 98, 180, 126, 135, 86, 92, 214, 75, 221, 27,
				157, 4, 92, 203, 235, 234, 39, 170, 30, 218, 100, 100, 155, 185, 152, 19, 67, 73,
				171, 46, 16, 231, 150, 190, 83, 175, 106, 104, 182, 175, 58, 112, 114, 96, 155, 77,
				179, 139, 236, 226, 12, 9, 236, 20, 191, 94, 103, 130, 95, 226, 185, 125, 59, 33,
				243, 126, 130, 246, 152, 60, 57, 144, 29, 40, 248, 89, 176, 174, 34, 187, 149, 8,
				186, 232, 192, 164, 130, 21, 17, 145, 25, 151, 165, 105, 78, 11, 210, 212, 85, 243,
				54, 83, 190, 179, 6, 67, 145, 56, 123, 208, 75, 19, 183, 220, 98, 129, 37, 7, 81,
				243,
			]
			.try_into()
			.map_err(|_e| Error::<T>::BoundedVecError)?;

			let public_key = PublicKey::<T> { spk, shared_params, shared_g };
			<SchedulerPuk<T>>::put(public_key);

			Ok(())
		}
	}
}

pub trait ScheduleFind<AccountId> {
	fn contains_scheduler(acc: AccountId) -> bool;
	fn get_controller_acc(acc: AccountId) -> AccountId;
	fn punish_scheduler(acc: AccountId);
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

	fn punish_scheduler(acc: <T as frame_system::Config>::AccountId) {
		let scheduler_list = SchedulerMap::<T>::get();
		for v in scheduler_list {
			if v.controller_user == acc {
				pallet_cess_staking::slashing::slash_scheduler::<T>(&v.stash_user);
			}
		}
	}
}
