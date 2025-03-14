#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
};
use frame_system::{
	ensure_root,
	pallet_prelude::OriginFor,
};

use pallet_tx_pause::{RuntimeCallNameOf, Pallet as TxPausePallet};

use sp_std::{marker::PhantomData};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	
	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_tx_pause::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Paused { full_name: RuntimeCallNameOf<T> },
	}

	// #[pallet::error]
	// pub enum Error<T> {

	// }

    #[pallet::storage]
    pub(super) type PausedPalletsHook<T: Config> = StorageMap<_, Blake2_128Concat, RuntimeCallNameOf<T>, bool>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn pause_pallet(origin: OriginFor<T>, full_name: RuntimeCallNameOf<T>) -> DispatchResult {
			let who = ensure_root(origin.clone())?;
			let (pallet_name, tx_name) = full_name.clone();
			if tx_name == "hook".to_string().as_bytes().to_vec() {
				PausedPalletsHook::<T>::insert(&full_name, true);
			} else {
				TxPausePallet::<T>::pause(origin, full_name.clone())?;
			}

			Self::deposit_event(Event::<T>::Paused{full_name});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn unpause_pallet(origin: OriginFor<T>, full_name: RuntimeCallNameOf<T>) -> DispatchResult {
			let who = ensure_root(origin.clone())?;
			let (pallet_name, tx_name) = full_name.clone();
			if tx_name == "hook".to_string().as_bytes().to_vec() {
				PausedPalletsHook::<T>::remove(&full_name.clone());
			} else {
				TxPausePallet::<T>::unpause(origin, full_name)?;
			}
			Ok(())
		}
	}
    
}