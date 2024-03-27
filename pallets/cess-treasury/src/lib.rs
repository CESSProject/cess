#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	traits::{
		Currency, ReservableCurrency, WithdrawReasons, Imbalance,
		ExistenceRequirement::KeepAlive, OnUnbalanced,
	},
	dispatch::{DispatchResult}, PalletId, Blake2_128Concat, ensure,
    pallet_prelude::{Weight, StorageValue, StorageMap, ValueQuery, Get, IsType},
};
// use sp_std::prelude::*;
use sp_runtime::{
    SaturatedConversion, Perbill,
	traits::{CheckedAdd, CheckedSub, CheckedDiv, AccountIdConversion},
};
use frame_system::{
	pallet_prelude::OriginFor,
	ensure_signed, ensure_root,
};

mod constants;
use constants::*;

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
    use frame_system::pallet_prelude::BlockNumberFor;

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

		type ReserveRewardId: Get<PalletId>;

		type BurnDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;

		type OneDay: Get<BlockNumberFor<Self>>;
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
		/// Data operation overflow
        Overflow,

		Unexpected,
    }

    #[pallet::storage]
	#[pallet::getter(fn currency_reward)]
	pub(super) type CurrencyReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn era_reward)]
	pub(super) type EraReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn reserve_reward)]
	pub(super) type ReserveReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn round_reward)]
	pub(super) type RoundReward<T: Config> = StorageMap<_, Blake2_128Concat, u32, (BalanceOf<T>, BalanceOf<T>), ValueQuery>;

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
	pub fn get_reward() -> BalanceOf<T> {
		<CurrencyReward<T>>::get()
	}

	pub fn get_reward_base() -> BalanceOf<T> {
		let era_reward = <EraReward<T>>::get();
		let base_reward = REWARD_BASE_MUTI.mul_floor(era_reward);

		base_reward
	}

	pub fn add_reward(amount: BalanceOf<T>) -> DispatchResult {
		<CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			*v = v.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn add_miner_reward_pool(amount: BalanceOf<T>) -> DispatchResult {
		<EraReward<T>>::put(amount);
		let now = frame_system::Pallet::<T>::block_number();
		let one_day = T::OneDay::get();
		let round: u32 = now
			.checked_div(&one_day).ok_or(Error::<T>::Overflow)?
			.try_into().map_err(|_| Error::<T>::Overflow)?;

		<RoundReward<T>>::mutate(round, |v| -> DispatchResult {
			v.0 = v.0.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})?;

		<CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			*v = v.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})?;

		Ok(())
	} 

	pub fn send_to_pid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let pid = T::PunishTreasuryId::get().into_account_truncating();
		<T as pallet::Config>::Currency::transfer(&acc, &pid, amount, KeepAlive)
	}

	pub fn send_to_sid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let sid = T::SpaceTreasuryId::get().into_account_truncating();
		<T as pallet::Config>::Currency::transfer(&acc, &sid, amount, KeepAlive)
	}

	pub fn send_to_rid(acc: AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
		let rid = T::ReserveRewardId::get().into_account_truncating();
		<ReserveReward<T>>::mutate(|v| -> DispatchResult {
			*v = v.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		<T as pallet::Config>::Currency::transfer(&acc, &rid, amount, KeepAlive)
	}

	pub fn reward_reserve(amount: BalanceOf<T>) -> DispatchResult {
		let mrid = T::MinerRewardId::get().into_account_truncating();
		Self::send_to_rid(mrid, amount)
	}

	pub fn sluice(amount: BalanceOf<T>) -> DispatchResult {
		<ReserveReward<T>>::mutate(|v| -> DispatchResult {
			*v = v.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		<CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			*v = v.checked_add(&amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		let rid = T::ReserveRewardId::get().into_account_truncating();
		let mrid = T::MinerRewardId::get().into_account_truncating();
		<T as pallet::Config>::Currency::transfer(&rid, &mrid, amount, KeepAlive)
	}
}

pub trait RewardPool<AccountId, Balance> {
	fn get_reward_base() -> Balance;
    fn get_reward() -> Balance;
    fn get_reward_128() -> u128;
	fn get_round_reward(round: u32) -> Balance;
	fn sub_round_reward(round: u32, reward: Balance) -> DispatchResult;
	fn reward_reserve(amount: Balance) -> DispatchResult;
	fn add_reward(amount: Balance) -> DispatchResult;
    fn sub_reward(amount: Balance) -> DispatchResult;
	fn send_reward_to_miner(miner: AccountId, amount: Balance) -> DispatchResult;
}

impl<T: Config> RewardPool<AccountOf<T>, BalanceOf<T>> for Pallet<T> {
	fn get_reward_base() -> BalanceOf<T> {
		Self::get_reward_base()
	}

    fn get_reward() -> BalanceOf<T> {
		Self::get_reward()
	}

	fn get_reward_128() -> u128 {
		<CurrencyReward<T>>::get().saturated_into()
	}

	fn get_round_reward(round: u32) -> BalanceOf<T> {
		<RoundReward<T>>::get(&round).0
	}

	fn sub_round_reward(round: u32, reward: BalanceOf<T>) -> DispatchResult {
		ensure!(<RoundReward<T>>::contains_key(round), Error::<T>::Unexpected);
		<RoundReward<T>>::mutate(round, |round_info| -> DispatchResult {
			round_info.1 = round_info.1.checked_add(&reward).ok_or(Error::<T>::Overflow)?;
			ensure!(round_info.1 <= round_info.0, Error::<T>::Unexpected);
			Ok(())
		})?;

		Ok(())
	}

	fn reward_reserve(amount: BalanceOf<T>) -> DispatchResult {
		Self::reward_reserve(amount)
	}

    fn add_reward(amount: BalanceOf<T>) -> DispatchResult {
        Self::add_reward(amount)?;

		Ok(())
    }

    fn sub_reward(amount: BalanceOf<T>) -> DispatchResult {
		let reward = Self::get_reward();
		if amount > reward {
			if let Err(e) = Self::sluice(amount) {
				log::info!("Insufficient reservoir reserves");
				return Err(e);
			}
		}
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