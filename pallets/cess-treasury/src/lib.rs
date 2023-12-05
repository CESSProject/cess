#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	traits::{
		Currency, ReservableCurrency, WithdrawReasons, Imbalance,
		ExistenceRequirement::KeepAlive, OnUnbalanced,
	},
	dispatch::{DispatchResult}, PalletId,
    pallet_prelude::{Weight, StorageValue, ValueQuery, Get, IsType},
};
// use sp_std::prelude::*;
use sp_runtime::{
    SaturatedConversion,
	traits::{CheckedAdd, CheckedSub, AccountIdConversion},
};
use frame_system::{
	pallet_prelude::OriginFor,
	ensure_signed, ensure_root,
};

pub use pallet::*;

type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

pub type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::PositiveImbalance;

type NegativeImbalanceOf<T> = <<T as pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
	pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

    #[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type MinerRewardId: Get<PalletId>;

		type PunishTreasuryId: Get<PalletId>;

		type SpaceTreasuryId: Get<PalletId>;

		type BurnDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;
    }
	
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Deposit {
			balance: BalanceOf<T>,
		},
	}

    #[pallet::error]
	pub enum Error<T> {
        Overflow,
    }

    #[pallet::storage]
	#[pallet::getter(fn currency_reward)]
	pub(super) type CurrencyReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		// FIX ME!
		#[pallet::weight(Weight::zero())]
		pub fn send_funds_to_pid(
			origin: OriginFor<T>,
			funds: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::send_to_pid(sender, funds)?;

			Ok(())
		}

		#[pallet::call_index(1)]
		// FIX ME!
		#[pallet::weight(Weight::zero())]
		pub fn send_funds_to_sid(
			origin: OriginFor<T>,
			funds: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::send_to_sid(sender, funds)?;

			Ok(())
		}

		#[pallet::call_index(2)]
		// FIX ME!
		#[pallet::weight(Weight::zero())]
		pub fn pid_burn_funds(
			origin: OriginFor<T>,
			burn_amount: BalanceOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let pid = T::PunishTreasuryId::get().into_account_truncating();

			let (debit, credit) = T::Currency::pair(burn_amount);

			T::BurnDestination::on_unbalanced(credit);

			if let Err(problem) = T::Currency::settle(&pid, debit, WithdrawReasons::TRANSFER, KeepAlive) {
				drop(problem);
			}

			Ok(())
		}

		#[pallet::call_index(3)]
		// FIX ME!
		#[pallet::weight(Weight::zero())]
		pub fn sid_burn_funds(
			origin: OriginFor<T>,
			burn_amount: BalanceOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let sid = T::SpaceTreasuryId::get().into_account_truncating();

			let (debit, credit) = T::Currency::pair(burn_amount);

			T::BurnDestination::on_unbalanced(credit);

			if let Err(problem) = T::Currency::settle(&sid, debit, WithdrawReasons::TRANSFER, KeepAlive) {
				drop(problem);
			}

			Ok(())
		}

		#[pallet::call_index(4)]
		// FIX ME!
		#[pallet::weight(Weight::zero())]
		pub fn pid_send_funds(
			origin: OriginFor<T>,
			acc: AccountOf<T>,
			funds: BalanceOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let pid = T::PunishTreasuryId::get().into_account_truncating();

			<T as pallet::Config>::Currency::transfer(&pid, &acc, funds, KeepAlive)?;

			Ok(())
		}

		#[pallet::call_index(5)]
		// FIX ME!
		#[pallet::weight(Weight::zero())]
		pub fn sid_send_funds(
			origin: OriginFor<T>,
			acc: AccountOf<T>,
			funds: BalanceOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let sid = T::SpaceTreasuryId::get().into_account_truncating();

			<T as pallet::Config>::Currency::transfer(&sid, &acc, funds, KeepAlive)?;

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn add_miner_reward_pool(amount: BalanceOf<T>) -> DispatchResult {
		<CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			// The total issuance amount will not exceed u128::Max, so there is no overflow risk
			*v = v.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})
	} 

	pub fn send_to_pid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let pid = T::PunishTreasuryId::get().into_account_truncating();
		<T as pallet::Config>::Currency::transfer(&acc, &pid, amount, KeepAlive)
	}

	pub fn send_to_sid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let sid = T::SpaceTreasuryId::get().into_account_truncating();
		<T as pallet::Config>::Currency::transfer(&acc, &sid, amount, KeepAlive)
	}
}

pub trait RewardPool<AccountId, Balance> {
    fn get_reward() -> Balance;
    fn get_reward_128() -> u128;
	fn add_reward(amount: Balance) -> DispatchResult;
    fn sub_reward(amount: Balance) -> DispatchResult;
	fn send_reward_to_miner(miner: AccountId, amount: Balance) -> DispatchResult;
}

impl<T: Config> RewardPool<AccountOf<T>, BalanceOf<T>> for Pallet<T> {
    fn get_reward() -> BalanceOf<T> {
		<CurrencyReward<T>>::get()
	}

	fn get_reward_128() -> u128 {
		<CurrencyReward<T>>::get().saturated_into()
	}

    fn add_reward(amount: BalanceOf<T>) -> DispatchResult {
        Self::add_miner_reward_pool(amount)?;

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

	fn send_reward_to_miner(beneficiary: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let reward_acc = T::MinerRewardId::get().into_account_truncating();
		<T as pallet::Config>::Currency::transfer(&reward_acc, &beneficiary, amount, KeepAlive)
	}
}

pub trait TreasuryHandle<AccountId, Balance> {
	fn send_to_pid(acc: AccountId, amount: Balance) -> DispatchResult;
	fn send_to_sid(acc: AccountId, amount: Balance) -> DispatchResult;
}

impl<T: Config> TreasuryHandle<AccountOf<T>, BalanceOf<T>> for Pallet<T> {
	fn send_to_pid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		Self::send_to_pid(acc, amount)
	}

	fn send_to_sid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		Self::send_to_sid(acc, amount)
	}
}

impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&T::MinerRewardId::get().into_account_truncating(), amount);
		// The total issuance amount will not exceed u128::Max, so there is no overflow risk
		Self::add_miner_reward_pool(numeric_amount).unwrap();

		Self::deposit_event(Event::Deposit { balance: numeric_amount });
	}
}