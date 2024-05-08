#![cfg_attr(not(feature = "std"), no_std)]

mod types;

use codec::{Decode, Encode};
use frame_system::{
    ensure_signed,
    pallet_prelude::{OriginFor, *},
};
use frame_support::traits::Get;
use frame_support::pallet_prelude::*;


type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    #[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

        #[pallet::constant]
		/// The pallet id
		type MyPalletId: Get<PalletId>;
    }

    #[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {

    }

    #[pallet::error]
	pub enum Error<T> {

    }

    #[pallet::storage]
	#[pallet::getter(fn reservoir)]
    pub(super) type Reservoir<T: Config> = StorageValue<_, ReserviorInfo<BalanceOf<T>>, ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn borrow_list)]
    pub(super) type BorrowList<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, BorrowInfo<T>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn user_passbook)]
    pub(super) type UserPassbook<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>>;
    

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
        pub fn filling(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            return Ok(())
        }

        #[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
        pub fn store(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            return Ok(())
        }
    }

    // helper
    impl<T: Config> Pallet<T> {

    }
}