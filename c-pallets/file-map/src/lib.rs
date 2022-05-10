//! # File Map Module
//!



#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::traits::{ReservableCurrency};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug,
    traits::SaturatedConversion,
};
use codec::{Encode, Decode};
use frame_support::{dispatch::{DispatchResult}, PalletId};
use frame_support::BoundedVec;
use sp_std::prelude::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;


#[frame_support::pallet]
pub mod pallet {
    use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Get}, Blake2_128Concat
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

    #[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    #[codec(mel_bound())]
    pub struct SchedulerInfo<T: pallet::Config> {
        pub ip: BoundedVec<u8, T::StringLimit>,
        pub stash_user: AccountOf<T>,
        pub controller_user: AccountOf<T>,
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
        RegistrationScheduler{acc: AccountOf<T>, ip: Vec<u8>},
    }

    #[pallet::error]
    pub enum Error<T> {
        //Already registered
        AlreadyRegistration,

        NotController,

        AlreadyReport,
    }

    #[pallet::storage]
    #[pallet::getter(fn scheduler_map)]
    pub(super) type SchedulerMap<T: Config> = StorageValue<_, BoundedVec<SchedulerInfo<T>, T::StringLimit>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn scheduler_exception)]
    pub(super) type SchedulerException<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, ExceptionReport<T>>;

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
				for (key ,value) in <SchedulerException<T>>::iter() {
                    if value.count > ( count / 2 ) as u32 {
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
        pub fn registration_scheduler(origin: OriginFor<T>, stash_account: AccountOf<T>, ip: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let acc = <pallet_cess_staking::Pallet<T>>::bonded(&stash_account).unwrap();
            if sender != acc {
                Err(Error::<T>::NotController)?;
            }
            let mut s_vec = SchedulerMap::<T>::get();
            let ip_bound = ip.clone().try_into().expect("too long");
            let scheduler = SchedulerInfo::<T>{
                ip: ip_bound,
                stash_user: stash_account.clone(),
                controller_user: sender.clone(),
            };
 
            if s_vec.to_vec().contains(&scheduler) {
                Err(Error::<T>::AlreadyRegistration)?;
            }
            s_vec.try_push(scheduler).expect("Length exceeded");
            SchedulerMap::<T>::put(s_vec);
            Self::deposit_event(Event::<T>::RegistrationScheduler{acc: sender, ip: ip});
            Ok(())
        }

        #[pallet::weight(1_000_000)]
        pub fn scheduler_exception_report(origin: OriginFor<T>, account: AccountOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            if !<SchedulerException<T>>::contains_key(&account) {
                <SchedulerException<T>>::insert(&account, ExceptionReport::<T>{
                    count: 0,
                    reporters: Default::default(),
                });
            }

            <SchedulerException<T>>::try_mutate(&account, |opt| -> DispatchResult {
                let o = opt.as_mut().unwrap();
                for value in &o.reporters.to_vec() {
                    if &sender == value {
                        Err(Error::<T>::AlreadyReport)?;
                    }
                }
                o.count += 1;
                o.reporters.try_push(account.clone()).expect("Length exceeded");
                Ok(())
            })?; 

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::StringLimit>, DispatchError> {
		let result: BoundedVec<P, T::StringLimit> = param.try_into().expect("too long");
		Ok(result)
	}

    fn contains_scheduler(acc: AccountOf<T>) -> bool {
        for i in <SchedulerMap<T>>::get().to_vec().iter() {
            if i.controller_user == acc {
                return true;
            }
        }
        false
    }
}

pub trait ScheduleFind<AccountId>
{
    fn contains_scheduler(acc: AccountId) -> bool;

}

impl<T: Config> ScheduleFind<<T as frame_system::Config>::AccountId> for Pallet<T>
{
    fn contains_scheduler(acc: <T as frame_system::Config>::AccountId) -> bool {
        for i in <SchedulerMap<T>>::get().to_vec().iter() {
            if i.controller_user == acc {
                return true;
            }
        }
        false
    }
}