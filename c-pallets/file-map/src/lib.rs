#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement::AllowDeath};
pub use pallet::*;
use sp_std::convert::TryInto;
use sp_std::fmt::Debug;

use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug,
	traits::{AccountIdConversion,SaturatedConversion}
};
use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{dispatch::DispatchResult, PalletId};


type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SchedulerInfo<T: pallet::Config> {
    ip: Vec<u8>,
    acc: AccountOf<T>,
}

#[frame_support::pallet]
pub mod pallet {
    use std::sync::mpsc::Sender;

    use super::*;
	use frame_support::{
		ensure,
		pallet_prelude::*,
		traits::{Get, schedule},
	};
	use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};


	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_sminer::Config + pallet_file_bank::Config + pallet_segment_book::Config {
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
        RegistrationScheduler{acc: AccountOf<T>, ip: Vec<u8>}
    }

    #[pallet::error]
    pub enum Error<T> {

    }

    #[pallet::storage]
    #[pallet::getter(fn scheduler_map)]
    pub(super) type SchedulerMap<T: Config> = StorageValue<_, Vec<SchedulerInfo<T>>, ValueQuery>;

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000_000)]
        pub fn registration_scheduler(origin: OriginFor<T>, ip: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let mut s_vec = SchedulerMap::<T>::get();
            let scheduler = SchedulerInfo::<T>{
                ip: ip.clone(),
                acc: sender.clone(),
            };
            s_vec.push(scheduler);
            SchedulerMap::<T>::put(s_vec);
            Self::deposit_event(Event::<T>::RegistrationScheduler{acc: sender, ip: ip});
            Ok(())
        }
    }
}