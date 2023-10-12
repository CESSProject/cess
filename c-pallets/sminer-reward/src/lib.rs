#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	traits::{
		Currency, ReservableCurrency,
	},
	dispatch::{DispatchResult},
    pallet_prelude::{StorageValue, ValueQuery},
};
use codec::{Decode, Encode};
use scale_info::TypeInfo;
// use sp_std::prelude::*;
use sp_runtime::{
    SaturatedConversion,
	traits::{CheckedAdd, CheckedSub},
};
// use frame_system::pallet_prelude::*;

pub use pallet::*;

type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
	pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::config]
	pub trait Config: frame_system::Config {
        /// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
    }

    #[pallet::error]
	pub enum Error<T> {
        Overflow,
    }

    #[pallet::storage]
	#[pallet::getter(fn currency_reward)]
	pub(super) type CurrencyReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;
}

pub trait RewardPool<Balance> {
    fn get_reward() -> Balance;
    fn get_reward_128() -> u128;
	fn add_reward(amount: Balance) -> DispatchResult;
    fn sub_reward(amount: Balance) -> DispatchResult;
}

impl<T: Config> RewardPool<BalanceOf<T>> for Pallet<T> {
    fn get_reward() -> BalanceOf<T> {
		<CurrencyReward<T>>::get()
	}

	fn get_reward_128() -> u128 {
		<CurrencyReward<T>>::get().saturated_into()
	}

    fn add_reward(amount: BalanceOf<T>) -> DispatchResult {
        <CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			// The total issuance amount will not exceed u128::Max, so there is no overflow risk
			*v = v.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})?;

		Ok(())
    }

    fn sub_reward(amount: BalanceOf<T>) -> DispatchResult {
        <CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			// The total issuance amount will not exceed u128::Max, so there is no overflow risk
			*v = v.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})?;

		Ok(())
    }
}