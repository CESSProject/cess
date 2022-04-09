#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{ReservableCurrency};
pub use pallet::*;


use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug
};

use codec::{Encode, Decode};
use frame_support::{dispatch::DispatchResult, PalletId};
use sp_std::prelude::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;


#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SchedulerInfo<T: pallet::Config> {
    ip: Vec<u8>,
    acc: AccountOf<T>,
}

#[frame_support::pallet]
pub mod pallet {


    use super::*;
	use frame_support::{

		pallet_prelude::*,
		traits::{Get},
	};
	use frame_system::{ensure_signed, pallet_prelude::*};


	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// pallet address.
		#[pallet::constant]
		type FileMapPalletId: Get<PalletId>;
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
    }

    #[pallet::storage]
    #[pallet::getter(fn scheduler_map)]
    pub(super) type SchedulerMap<T: Config> = StorageValue<_, Vec<SchedulerInfo<T>>, ValueQuery>;

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        //Scheduling registration method
        #[pallet::weight(1_000_000)]
        pub fn registration_scheduler(origin: OriginFor<T>, ip: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let mut s_vec = SchedulerMap::<T>::get();
            let scheduler = SchedulerInfo::<T>{
                ip: ip.clone(),
                acc: sender.clone(),
            };

            if s_vec.contains(&scheduler) {
                Err(Error::<T>::AlreadyRegistration)?;
            }
            s_vec.push(scheduler);
            SchedulerMap::<T>::put(s_vec);
            Self::deposit_event(Event::<T>::RegistrationScheduler{acc: sender, ip: ip});
            Ok(())
        }
    }
}