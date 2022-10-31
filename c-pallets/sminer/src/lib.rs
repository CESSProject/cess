//! # Sminer Module
//!
//! Contain operations related storage miners.
//!
//! ### Terminology
//!
//! * **Collateral:** The Staking amount when registering storage miner.
//! * **Earnings:** Store the storage miner's earnings during mining.
//! * **Locked:** Store the locked amount of the storage miner during mining.
//!
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `regnstk` - Staking and register for storage miner.
//! * `redeem` - Redeem and exit for storage miner.
//! * `claim` - Claim the rewards from storage miner's earnings.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
	transactional,
	storage::bounded_vec::BoundedVec,
	traits::{
		schedule::{Anon as ScheduleAnon, DispatchTime, Named as ScheduleNamed},
		Currency,
		ExistenceRequirement::AllowDeath,
		Get, Imbalance, LockIdentifier, OnUnbalanced, ReservableCurrency,
	},
};
use cp_cess_common::IpAddress;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResult, Dispatchable},
	pallet_prelude::DispatchError,
	PalletId,
	traits::schedule,
};
use frame_system::{self as system};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, CheckedMul, SaturatedConversion},
	RuntimeDebug, Perbill
};
use sp_std::{convert::TryInto, prelude::*};
use types::*;
pub mod weights;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

const M_BYTE: u128 = 1_048_576;
const STATE_POSITIVE: &str = "positive";
const STATE_FROZEN: &str = "frozen";
const STATE_EXIT_FROZEN: &str = "e_frozen";
const STATE_EXIT: &str = "exit";
const FAUCET_VALUE: u128 = 10000000000000000;
const DOUBLE: u8 = 2;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure,
		pallet_prelude::{ValueQuery, *},
		traits::Get,
	};
	use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
	const DEMOCRACY_IDA: LockIdentifier = *b"msminerA";
	const DEMOCRACY_IDB: LockIdentifier = *b"msminerB";
	const DEMOCRACY_IDC: LockIdentifier = *b"msminerC";

	// const DEMOCRACY_IDD: LockIdentifier = *b"msminerD";

	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// The treasury's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type ItemLimit: Get<u32>;
		#[pallet::constant]
		type MultipleFines: Get<u8>;
		#[pallet::constant]
		type DepositBufferPeriod: Get<u32>;
		#[pallet::constant]
		type OneDayBlock: Get<BlockNumberOf<Self>>;
		#[pallet::constant]
		type LockInPeriod: Get<u8>;
		#[pallet::constant]
		type MaxAward: Get<u128>;
		/// The Scheduler.
		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;
		/// The WeightInfo.
		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new account was set.
		Registered {
			acc: AccountOf<T>,
			staking_val: BalanceOf<T>,
		},
		/// An account was redeemed.
		Redeemed {
			acc: AccountOf<T>,
			deposit: BalanceOf<T>,
		},
		/// An account was claimed.
		Claimed {
			acc: AccountOf<T>,
			deposit: BalanceOf<T>,
		},
		/// Storage space is triggered periodically.
		TimingStorageSpace(),
		/// Scheduled Task Execution
		TimedTask(),
		/// Users to withdraw faucet money
		DrawFaucetMoney(),
		/// User recharges faucet
		FaucetTopUpMoney {
			acc: AccountOf<T>,
		},
		/// Prompt time
		LessThan24Hours {
			last: BlockNumberOf<T>,
			now: BlockNumberOf<T>,
		},
		//The miners have been frozen
		AlreadyFrozen {
			acc: AccountOf<T>,
		},
		//Miner exit event
		MinerExit {
			acc: AccountOf<T>,
		},

		MinerClaim {
			acc: AccountOf<T>,
		},

		IncreaseCollateral {
			acc: AccountOf<T>,
			balance: BalanceOf<T>,
		},
		/// Some funds have been deposited. \[deposit\]
		Deposit {
			balance: BalanceOf<T>,
		},
		UpdataBeneficiary {
			acc: AccountOf<T>,
			new: AccountOf<T>,
		},
		UpdataIp {
			acc: AccountOf<T>,
			old: IpAddress,
			new: IpAddress,
		},
		StartOfBufferPeriod {
			when: BlockNumberOf<T>,
		},
		EndOfBufferPeriod {
			when: BlockNumberOf<T>,
		},
	}

	/// Error for the sminer pallet.
	#[pallet::error]
	pub enum Error<T> {
		NotMiner,
		/// An account doesn't registered.
		UnregisteredAccountId,
		/// An account has locked balances.
		LockedNotEmpty,
		/// An account already registered.
		AlreadyRegistered,
		/// An account's earnings is empty.
		EarningsIsEmpty,
		/// An operation would lead to an overflow.
		Overflow,
		/// No owner.
		// NotOwner,
		/// User does not exist.
		NotExisted,
		/// Lack of permissions.
		LackOfPermissions,
		/// Beyond the requirements.
		BeyondClaim,
		/// The duration is less than 24 hours.
		LessThan24Hours,
		/// Numerical conversion error.
		ConversionError,
		/// You can't divide by zero
		DivideByZero,

		InsufficientAvailableSpace,
		//The account has been frozen
		AlreadyFrozen,

		LockInNotOver,

		NotpositiveState,

		StorageLimitReached,

		BoundedVecError,

		DataNotExist,
		//haven't bought space at all
		NotPurchasedPackage,
	}

	#[pallet::storage]
	#[pallet::getter(fn miner_lock_in)]
	pub(super) type MinerLockIn<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberOf<T>>;

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn miner_items)]
	pub(super) type MinerItems<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		MinerInfo<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>>,
	>;

	/// The hashmap for index of storage miners, it's unique to whole system.
	#[pallet::storage]
	#[pallet::getter(fn peer_index)]
	pub(super) type PeerIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// The total power of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_power)]
	pub(super) type TotalIdleSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// The total storage space to fill of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_space)]
	pub(super) type TotalServiceSpace<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// Store all miner information
	#[pallet::storage]
	#[pallet::getter(fn miner_info)]
	pub(super) type AllMiner<T: Config> =
		StorageValue<_, BoundedVec<AccountOf<T>, T::ItemLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_space)]
	pub(super) type PurchasedSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn dad_miner)]
	pub(super) type BadMiner<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn buffer_period)]
	pub(super) type BufferPeriod<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberOf<T>,
		BoundedVec<AccountOf<T>, T::ItemLimit>,
		ValueQuery,
	>;

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn calculate_reward_order)]
	pub(super) type CalculateRewardOrderMap<T: Config> = CountedStorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<CalculateRewardOrder<T>, T::ItemLimit>,
		ValueQuery,
	>;

	/// The hashmap for checking registered or not.
	#[pallet::storage]
	#[pallet::getter(fn reward_claim)]
	pub(super) type RewardClaimMap<T: Config> =
		CountedStorageMap<_, Blake2_128Concat, T::AccountId, RewardClaim<T::AccountId, BalanceOf<T>>>;

	/// The hashmap for checking registered or not.
	#[pallet::storage]
	#[pallet::getter(fn faucet_record)]
	pub(super) type FaucetRecordMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, FaucetRecord<BlockNumberOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn currency_reward)]
	pub(super) type CurrencyReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Staking and register for storage miner.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `beneficiary`: The beneficiary related to signer account.
		/// - `ip`: The registered IP of storage miner.
		/// - `staking_val`: The number of staking.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn regnstk(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
			ip: IpAddress,
			staking_val: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			T::Currency::reserve(&sender, staking_val.clone())?;
			let cur_idx = PeerIndex::<T>::get();
			let peer_id = cur_idx.checked_add(1).ok_or(Error::<T>::Overflow)?;
			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>> {
					peer_id,
					beneficiary: beneficiary.clone(),
					ip: ip,
					collaterals: staking_val.clone(),
					state: Self::vec_to_bound::<u8>(STATE_POSITIVE.as_bytes().to_vec())?,
					power: 0,
					space: 0,
					reward_info: RewardInfo::<BalanceOf<T>> {
						total_reward: BalanceOf::<T>::from(0u32),
						total_rewards_currently_available: BalanceOf::<T>::from(0u32),
						total_not_receive: BalanceOf::<T>::from(0u32),
					},
				},
			);
			<PeerIndex<T>>::put(peer_id);
			AllMiner::<T>::try_mutate(|all_miner| -> DispatchResult {
				all_miner
					.try_push(sender.clone())
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Registered {
				acc: sender.clone(),
				staking_val: staking_val.clone(),
			});
			Ok(())
		}

		/// Increase the miner collateral.
		///
		/// Parameters:
		/// - `collaterals`: Miner's TCESS.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::increase_collateral())]
		pub fn increase_collateral(
			origin: OriginFor<T>,
			#[pallet::compact] collaterals: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			T::Currency::reserve(&sender, collaterals)?;
			let mut balance: BalanceOf<T> = 0u32.saturated_into();
			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				miner_info.collaterals =
					miner_info.collaterals.checked_add(&collaterals).ok_or(Error::<T>::Overflow)?;
				balance = miner_info.collaterals;
				if miner_info.state == STATE_FROZEN.as_bytes().to_vec()
					|| miner_info.state == STATE_EXIT_FROZEN.as_bytes().to_vec()
				{
					let limit = Self::check_collateral_limit(miner_info.power)?;
					if miner_info.collaterals > limit {
						if miner_info.state.to_vec() == STATE_FROZEN.as_bytes().to_vec() {
							miner_info.state =
								Self::vec_to_bound(STATE_POSITIVE.as_bytes().to_vec())?;
						} else {
							miner_info.state = Self::vec_to_bound(STATE_EXIT.as_bytes().to_vec())?;
						}
						BadMiner::<T>::remove(&sender);
					}
				}
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseCollateral { acc: sender, balance });
			Ok(())
		}

		/// updata miner beneficiary.
		///
		/// Parameters:
		/// - `beneficiary`: The beneficiary related to signer account.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_beneficiary())]
		pub fn update_beneficiary(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				miner_info.beneficiary = beneficiary.clone();
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::UpdataBeneficiary { acc: sender, new: beneficiary });
			Ok(())
		}

		/// updata miner IP.
		///
		/// Parameters:
		/// - `ip`: The registered IP of storage miner.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_ip())]
		pub fn update_ip(origin: OriginFor<T>, ip: IpAddress) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let old = <MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> Result<IpAddress, DispatchError> {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				let old = miner_info.ip.clone();
				miner_info.ip = ip.clone();
				Ok(old)
			})?;

			Self::deposit_event(Event::<T>::UpdataIp { acc: sender, old, new: ip });
			Ok(())
		}

		//Miner exit method, Irreversible process.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::exit_miner())]
		pub fn exit_miner(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let state = Self::check_state(&sender)?;
			if state != STATE_POSITIVE.as_bytes().to_vec() {
				Err(Error::<T>::NotpositiveState)?;
			}
			MinerItems::<T>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;

				Self::sub_space(&sender, miner_info.space)?;
				Self::sub_power(&sender, miner_info.power)?;
				miner_info.state = Self::vec_to_bound(STATE_EXIT.as_bytes().to_vec())?;
				Ok(())
			})?;
			let now_block = <frame_system::Pallet<T>>::block_number();
			MinerLockIn::<T>::insert(&sender, now_block);

			Self::deposit_event(Event::<T>::MinerExit { acc: sender });
			Ok(())
		}

		//Method for miners to redeem deposit
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw())]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let state = Self::check_state(&sender)?;
			if state != STATE_EXIT.as_bytes().to_vec() {
				Err(Error::<T>::NotExisted)?;
			}
			let now_block: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let lock_in_strat: u128 = MinerLockIn::<T>::try_get(&sender)
				.map_err(|_e| Error::<T>::LockInNotOver)?
				.saturated_into();
			let mut lock_in_period: u128 = T::OneDayBlock::get().saturated_into();
			let day = T::LockInPeriod::get();
			lock_in_period = lock_in_period * day as u128;
			// let mut lock_in_period: u128 = 50;
			if lock_in_strat + lock_in_period > now_block {
				Err(Error::<T>::LockInNotOver)?;
			}
			let collaterals = MinerItems::<T>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotMiner)?
				.collaterals;
			T::Currency::unreserve(&sender, collaterals);
			Self::delete_miner_info(&sender)?;
			MinerLockIn::<T>::remove(&sender);

			Self::deposit_event(Event::<T>::MinerClaim { acc: sender });
			Ok(())
		}
		// 	storage_info_vec.append(&mut info1);

		// 	<StorageInfoVec<T>>::put(storage_info_vec);
		// 	Self::deposit_event(Event::<T>::TimingStorageSpace());
		// 	Ok(())
		// }
		/// Add reward orders.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_increase_rewards())]
		pub fn timed_increase_rewards(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let total_power = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
			ensure!(total_power != 0, Error::<T>::DivideByZero);

			// let reward_pot = T::PalletId::get().into_account();
			let mut total_award: u128 =
				<CurrencyReward<T>>::get().try_into().map_err(|_| Error::<T>::Overflow)?;
			let max_award = T::MaxAward::get();
			if total_award > max_award {
				<CurrencyReward<T>>::try_mutate(|currency_reward| -> DispatchResult {
					*currency_reward = currency_reward
						.checked_sub(&max_award.try_into().map_err(|_| Error::<T>::Overflow)?)
						.ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?;
				total_award = max_award;
			} else {
				<CurrencyReward<T>>::try_mutate(|currency_reward| -> DispatchResult {
					*currency_reward = 0u128.try_into().map_err(|_| Error::<T>::Overflow)?;
					Ok(())
				})?;
			}
			for (acc, detail) in <MinerItems<T>>::iter() {
				let miner_total_power = detail.power.checked_add(detail.space).ok_or(Error::<T>::Overflow)?;
				if miner_total_power == 0 {
					continue;
				}
				let miner_award: u128 = total_award
					.checked_mul(miner_total_power).ok_or(Error::<T>::Overflow)?
					.checked_div(total_power).ok_or(Error::<T>::Overflow)?;
				let _ = Self::add_reward_order1(&acc, miner_award);
			}

			Self::deposit_event(Event::<T>::TimedTask());
			Ok(())
		}

		/// Added timed tasks for reward orders.
		///
		/// The dispatch origin of this call must be _root_.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timing_task_increase_power_rewards())]
		pub fn timing_task_increase_power_rewards(
			origin: OriginFor<T>,
			when: BlockNumberOf<T>,
			cycle: BlockNumberOf<T>,
			degree: u32,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if T::SScheduler::schedule_named(
				(DEMOCRACY_IDA).encode(),
				DispatchTime::At(when),
				Some((cycle, degree)),
				schedule::HIGHEST_PRIORITY,
				frame_system::RawOrigin::Root.into(),
				Call::timed_increase_rewards {}.into(),
			)
			.is_err()
			{
				frame_support::print("LOGIC ERROR: timed_increase_rewards/schedule_named failed");
			}

			// Self::deposit_event(Event::<T>::Add(sender.clone()));
			Ok(())
		}

		/// Users receive rewards for scheduled tasks.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_user_receive_award1(<RewardClaimMap<T>>::count()))]
		pub fn timed_user_receive_award1(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			for (sender, info) in <RewardClaimMap<T>>::iter() {
				let acc = info.clone().beneficiary;
				let state =
					<MinerItems<T>>::try_get(&sender).map_err(|_e| Error::<T>::NotMiner)?.state;
				if state == STATE_FROZEN.as_bytes().to_vec()
					|| state == STATE_EXIT_FROZEN.as_bytes().to_vec()
				{
					Self::deposit_event(Event::<T>::AlreadyFrozen { acc: acc.clone() });
					continue;
				}

				let reward_pot = T::PalletId::get().into_account();

				let award = info.current_availability;
				let total = info.total_reward;

				ensure!(
					info
						.have_to_receive
						.checked_add(&award)
						.ok_or(Error::<T>::Overflow)?
						<= info.total_reward,
					Error::<T>::BeyondClaim
				);

				<T as pallet::Config>::Currency::transfer(&reward_pot, &acc, award, AllowDeath)?;

				RewardClaimMap::<T>::try_mutate(&sender, |reward_claim_opt| -> DispatchResult {
					let reward_claim =
						reward_claim_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
					let have_to_receive = reward_claim
						.have_to_receive
						.checked_add(&award)
						.ok_or(Error::<T>::Overflow)?;
					reward_claim.have_to_receive = have_to_receive;
					reward_claim.current_availability = 0u32.into();
					reward_claim.total_not_receive =
						total.checked_sub(&award).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?;
				MinerItems::<T>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
					let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
					let total_not_receive = info.total_not_receive;
					miner_info.reward_info.total_not_receive = total_not_receive;
					Ok(())
				})?;

				if Self::check_exist_miner_reward(&sender)? {
					Self::clean_reward_map(&sender)
				}
			}

			Ok(())
		}
		/// Users receive rewards for scheduled tasks.
		///
		/// The dispatch origin of this call must be _root_.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timing_user_receive_award())]
		pub fn timing_user_receive_award(
			origin: OriginFor<T>,
			when: BlockNumberOf<T>,
			cycle: BlockNumberOf<T>,
			degree: u32,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if T::SScheduler::schedule_named(
				(DEMOCRACY_IDC).encode(),
				DispatchTime::At(when),
				Some((cycle, degree)),
				schedule::HIGHEST_PRIORITY,
				frame_system::RawOrigin::Root.into(),
				Call::timed_user_receive_award1 {}.into(),
			)
			.is_err()
			{
				frame_support::print(
					"LOGIC ERROR: timed_user_receive_award1/schedule_named failed",
				);
			}

			// Self::deposit_event(Event::<T>::Add(sender.clone()));
			Ok(())
		}
		/// Update the user reward table for scheduled tasks.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_task_award_table(<CalculateRewardOrderMap<T>>::count()))]
		pub fn timed_task_award_table(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			for (acc, order_vec) in <CalculateRewardOrderMap<T>>::iter() {
				if !<MinerItems<T>>::contains_key(&acc) {
					Self::clean_reward_map(&acc);
					continue;
				}

				let mut total: u128 = 0;

				let now = <frame_system::Pallet<T>>::block_number();
				let mut avail: BalanceOf<T> =
					0u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;

				for i in &order_vec.to_vec() {
					total = total.checked_add(i.calculate_reward).ok_or(Error::<T>::Overflow)?;
					if i.deadline > now {
						//div 225 = * 80% / 180
						let this_avail: BalanceOf<T> = i.calculate_reward
							.checked_div(225).ok_or(Error::<T>::Overflow)?
							.try_into().map_err(|_e| Error::<T>::Overflow)?;
						avail = avail.checked_add(&this_avail).ok_or(Error::<T>::Overflow)?;
					}
				}

				let total_20_percent: BalanceOf<T> = Perbill::from_percent(20).mul_floor(total)
					.try_into()
					.map_err(|_e| Error::<T>::ConversionError)?;

				let currently_available: BalanceOf<T> = avail;

				let reward2: BalanceOf<T> =
					total.try_into().map_err(|_e| Error::<T>::ConversionError)?;

				let miner = MinerItems::<T>::try_get(&acc).map_err(|_e| Error::<T>::NotMiner)?;

				if !<RewardClaimMap<T>>::contains_key(&acc) {
					<RewardClaimMap<T>>::insert(
						&acc,
						RewardClaim::<T::AccountId, BalanceOf<T>> {
							beneficiary: miner.beneficiary,
							total_reward: reward2,
							have_to_receive: 0u32.into(),
							current_availability: currently_available
								.checked_add(&total_20_percent)
								.ok_or(Error::<T>::Overflow)?,
							total_not_receive: reward2,
						},
					);

					if <MinerItems<T>>::contains_key(&acc) {
						MinerItems::<T>::try_mutate(&acc, |miner_info_opt| -> DispatchResult {
							let miner_info =
								miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
							miner_info.reward_info.total_reward = reward2;
							miner_info.reward_info.total_rewards_currently_available =
								currently_available
									.checked_add(&total_20_percent)
									.ok_or(Error::<T>::Overflow)?;
							miner_info.reward_info.total_not_receive = reward2;
							Ok(())
						})?;
					}
				} else {
					RewardClaimMap::<T>::try_mutate(&acc, |reward_claim_opt| -> DispatchResult {
						//Convert balance to U128 for multiplication and division
						let reward_claim =
							reward_claim_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
						let diff = reward2
							.checked_sub(&reward_claim.total_reward)
							.ok_or(Error::<T>::Overflow)?;
						let diff128 =
							TryInto::<u128>::try_into(diff).map_err(|_e| Error::<T>::Overflow)?;
						let diff_20_percent: BalanceOf<T> = Perbill::from_percent(20).mul_floor(diff128)
							.try_into()
							.map_err(|_e| Error::<T>::ConversionError)?;
						//Before switching back to balance
						//Plus 20% of the new share
						reward_claim.total_reward = reward2;
						reward_claim.current_availability = reward_claim
							.current_availability
							.checked_add(&currently_available)
							.ok_or(Error::<T>::Overflow)?
							.checked_add(&diff_20_percent)
							.ok_or(Error::<T>::Overflow)?;
						reward_claim.total_not_receive = reward2
							.checked_sub(&reward_claim.have_to_receive)
							.ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?;

					if <MinerItems<T>>::contains_key(&acc) {
						MinerItems::<T>::try_mutate(&acc, |miner_info_opt| -> DispatchResult {
							let miner_info =
								miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
							miner_info.reward_info.total_reward = reward2;

							let reward_claim_map = RewardClaimMap::<T>::try_get(&acc)
								.map_err(|_e| Error::<T>::NotMiner)?;
							miner_info.reward_info.total_rewards_currently_available =
								reward_claim_map
									.have_to_receive
									.checked_add(&reward_claim_map.current_availability)
									.ok_or(Error::<T>::Overflow)?;

							let total_not_receive = RewardClaimMap::<T>::try_get(&acc)
								.map_err(|_e| Error::<T>::NotMiner)?
								.total_not_receive;
							miner_info.reward_info.total_not_receive = total_not_receive;
							Ok(())
						})?;
					}
				}
			}

			Self::deposit_event(Event::<T>::TimedTask());
			Ok(())
		}
		/// Update the user reward table for scheduled tasks.
		///
		/// The dispatch origin of this call must be _root_.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timing_task_award_table())]
		pub fn timing_task_award_table(
			origin: OriginFor<T>,
			when: BlockNumberOf<T>,
			cycle: BlockNumberOf<T>,
			degree: u32,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if T::SScheduler::schedule_named(
				(DEMOCRACY_IDB).encode(),
				DispatchTime::At(when),
				Some((cycle, degree)),
				schedule::HIGHEST_PRIORITY,
				frame_system::RawOrigin::Root.into(),
				Call::timed_task_award_table {}.into(),
			)
			.is_err()
			{
				frame_support::print("LOGIC ERROR: timed_task_receive_award/schedule_named failed");
			}

			// Self::deposit_event(Event::<T>::Add(sender.clone()));
			Ok(())
		}

		/// A buffer period has expired.
		///
		/// Parameters:
		/// - `when`: The block when the buffer period starts.
		#[transactional]
		#[pallet::weight(1_000_000)]
		pub fn buffer_period_end(origin: OriginFor<T>, when: BlockNumberOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let miner_vec = <BufferPeriod<T>>::get(&when);
			for dad_miner in miner_vec.iter() {
				if MinerItems::<T>::contains_key(&dad_miner)
					&& BadMiner::<T>::contains_key(&dad_miner)
				{
					let mr =
						MinerItems::<T>::try_get(&dad_miner).map_err(|_e| Error::<T>::NotMiner)?;
					T::Currency::unreserve(&dad_miner, mr.collaterals);
					Self::delete_miner_info(&dad_miner)?;
					Self::clean_reward_map(&dad_miner);
				}
			}
			Self::deposit_event(Event::<T>::EndOfBufferPeriod { when });

			Ok(())
		}

		/// Punish offline miners.
		///
		/// The dispatch origin of this call must be _root_.
		///
		/// Parameters:
		/// - `acc`: miner .
		/// The faucet top up.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `acc`: Top-up account .
		/// - `acc`: Top-up amount .
		#[transactional]
		#[pallet::weight(100_000)]
		pub fn faucet_top_up(origin: OriginFor<T>, award: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let reward_pot = T::PalletId::get().into_account();
			<T as pallet::Config>::Currency::transfer(&sender, &reward_pot, award, AllowDeath)?;

			Self::deposit_event(Event::<T>::FaucetTopUpMoney { acc: sender.clone() });
			Ok(())
		}

		/// Users receive money through the faucet.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `acc`: Withdraw money account.
		#[transactional]
		#[pallet::weight(100_000)]
		pub fn faucet(origin: OriginFor<T>, to: AccountOf<T>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			if !<FaucetRecordMap<T>>::contains_key(&to) {
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> {
						last_claim_time: BlockNumberOf::<T>::from(0u32),
					},
				);

				let now = <frame_system::Pallet<T>>::block_number();
				let reward_pot = T::PalletId::get().into_account();

				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					AllowDeath,
				)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> { last_claim_time: now },
				);
			} else {
				let one_day: u32 = T::OneDayBlock::get().saturated_into();
				let faucet_record = FaucetRecordMap::<T>::try_get(&to).map_err(|e| {
					log::error!("faucet error is: {:?}", e);
					Error::<T>::DataNotExist
				})?;
				let now = <frame_system::Pallet<T>>::block_number();

				let mut flag: bool = true;
				if now >= BlockNumberOf::<T>::from(one_day) {
					if !(faucet_record.last_claim_time
						<= now
							.checked_sub(&BlockNumberOf::<T>::from(one_day))
							.ok_or(Error::<T>::Overflow)?)
					{
						Self::deposit_event(Event::<T>::LessThan24Hours {
							last: faucet_record.last_claim_time,
							now,
						});
						flag = false;
					}
				} else {
					if !(faucet_record.last_claim_time <= BlockNumberOf::<T>::from(0u32)) {
						Self::deposit_event(Event::<T>::LessThan24Hours {
							last: faucet_record.last_claim_time,
							now,
						});
						flag = false;
					}
				}
				ensure!(flag, Error::<T>::LessThan24Hours);

				let reward_pot = T::PalletId::get().into_account();
				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					AllowDeath,
				)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> { last_claim_time: now },
				);
			}

			Self::deposit_event(Event::<T>::DrawFaucetMoney());
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Add computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_power(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			Err(Error::<T>::NotMiner)?;
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.power =
				miner_info.power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
			*total_power = total_power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// Sub computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_power(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?; //read 1
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.power =
				miner_info.power.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?; //read 1 write 1

		TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
			*total_power = total_power.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?; //read 1 write 1

		Ok(())
	}

	/// Add space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.space =
				miner_info.space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// Sub space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.space =
				miner_info.space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		TotalServiceSpace::<T>::mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// According to aid to punish.
	///
	/// Parameters:
	/// - `aid`: aid.
	/// - `failure_num`: Times miner failed to submit the proof in one challenge.
	/// - `total_proof`: The number of proofs a miner needs.
	/// - `consecutive_fines`: Number of successive penalties in multiple challenges.
	pub fn punish(
		aid: AccountOf<T>,
		failure_num: u32,
		total_proof: u32,
		consecutive_fines: u8,
	) -> DispatchResult {
		if !<MinerItems<T>>::contains_key(&aid) {
			Err(Error::<T>::NotMiner)?;
		}

		//There is a judgment on whether the primary key exists above
		let mr = MinerItems::<T>::try_get(&aid).map_err(|_e| Error::<T>::NotMiner)?; //read 1
		let acc = T::PalletId::get().into_account();

		let calcu_failure_fee =
			Self::calcu_failure_fee(aid.clone(), failure_num, total_proof)?; // read 1

		if consecutive_fines >= T::MultipleFines::get() {
			calcu_failure_fee.checked_mul(DOUBLE as u128).ok_or(Error::<T>::Overflow)?;
		}

		let mut punish_amount: BalanceOf<T> = 0u128
			.checked_add(calcu_failure_fee.into())
			.ok_or(Error::<T>::Overflow)?
			.try_into()
			.map_err(|_e| Error::<T>::ConversionError)?;

		if mr.collaterals < punish_amount {
			punish_amount = mr.collaterals;
		}

		T::Currency::unreserve(&aid, punish_amount);
		MinerItems::<T>::try_mutate(&aid, |miner_info_opt| -> DispatchResult { // read 1 write 1
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.collaterals =
				miner_info.collaterals.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		let miner_info = <MinerItems<T>>::try_get(&aid).map_err(|_e| Error::<T>::NotMiner)?; // read 1
		let limit = Self::check_collateral_limit(miner_info.power)?;
		if miner_info.collaterals < limit {
			Self::join_buffer_pool(aid.clone())?; // read 2 write 3
		}
		T::Currency::transfer(&aid, &acc, punish_amount, AllowDeath)?;

		Ok(())
	}

	/// A buffer period begins and miners are required to make a sufficient deposit before the buffer period ends.
	///
	pub fn start_buffer_period_schedule() -> DispatchResult {
		let now_block = <frame_system::Pallet<T>>::block_number();
		if BufferPeriod::<T>::contains_key(&now_block) {
			let mut period: u32 = T::OneDayBlock::get().saturated_into();
			period = period * T::DepositBufferPeriod::get();
			let buffer_period =
				now_block.checked_add(&period.saturated_into()).ok_or(Error::<T>::Overflow)?;
			T::AScheduler::schedule(
				DispatchTime::At(buffer_period),
				None,
				schedule::LOWEST_PRIORITY,
				frame_system::RawOrigin::Root.into(),
				Call::buffer_period_end { when: now_block.clone() }.into(),
			)?;
		}
		Self::deposit_event(Event::<T>::StartOfBufferPeriod { when: now_block });
		Ok(())
	}

	/// Add miners with insufficient deposits to the buffer pool.
	///
	/// Parameters:
	/// - `acc`: miner account.
	fn join_buffer_pool(acc: AccountOf<T>) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();

		<BadMiner<T>>::insert(&acc, &now); // write 1
		if BufferPeriod::<T>::contains_key(&now) {
			BufferPeriod::<T>::try_mutate(&now, |bad_miner_vec| -> DispatchResult { //read 1 write 1
				bad_miner_vec
					.try_push(acc.clone())
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;
		} else {
			let mut new_vec: Vec<AccountOf<T>> = Vec::new();
			new_vec.push(acc.clone());
			let new_dad_vec = Self::vec_to_bound::<AccountOf<T>>(new_vec)?;
			<BufferPeriod<T>>::insert(now, new_dad_vec);
		}

		MinerItems::<T>::try_mutate(&acc, |miner_info_opt| -> DispatchResult { //read 1 write 1
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			if miner_info.state != STATE_FROZEN.as_bytes().to_vec()
				&& miner_info.state != STATE_EXIT_FROZEN.as_bytes().to_vec()
			{
				if miner_info.state.to_vec() == STATE_POSITIVE.as_bytes().to_vec() {
					miner_info.state = STATE_FROZEN.as_bytes().to_vec().try_into().map_err(|_e| Error::<T>::Overflow)?;
				} else if miner_info.state.to_vec() == STATE_EXIT.as_bytes().to_vec() {
					miner_info.state =
						Self::vec_to_bound::<u8>(STATE_EXIT_FROZEN.as_bytes().to_vec())?;
				}
			}
			Ok(())
		})?;

		Ok(())
	}

	fn delete_miner_info(acc: &AccountOf<T>) -> DispatchResult {
		//There is a judgment on whether the primary key exists above
		let miner = MinerItems::<T>::try_get(&acc).map_err(|_e| Error::<T>::NotMiner)?;
		TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
			*total_power = total_power.checked_sub(miner.power).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_sub(miner.space).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| *s != acc.clone());
		AllMiner::<T>::put(miner_list);

		BadMiner::<T>::remove(acc);
		<MinerItems<T>>::remove(acc);

		Ok(())
	}

	//Check whether the rewards for exited miners have been paid out.
	//true is Distribution completed. false is Unfinished
	fn check_exist_miner_reward(acc: &AccountOf<T>) -> Result<bool, Error<T>> {
		if !<MinerItems<T>>::contains_key(acc) {
			let reward_map =
				<RewardClaimMap<T>>::try_get(acc).map_err(|_e| Error::<T>::NotMiner)?;
			if reward_map.have_to_receive == reward_map.total_reward {
				return Ok(true);
			}
		}

		Ok(false)
	}

	fn clean_reward_map(acc: &AccountOf<T>) {
		<CalculateRewardOrderMap<T>>::remove(acc);
		<RewardClaimMap<T>>::remove(acc);
	}

	/// Add reward orders for corresponding accounts.
	///
	/// Parameters:
	/// - `acc`: Rewards account.
	/// - `calculate_reward`: Calculate the reward.
	pub fn add_reward_order1(acc: &AccountOf<T>, calculate_reward: u128) -> DispatchResult {
		let now_block = <frame_system::Pallet<T>>::block_number();
		// With block timing, 180 days =5184000 blocks
		// let deadline = now + T::BlockNumber::from(5184000u32);
		// // test 5 minutes
		// let deadline = now + T::BlockNumber::from(18000u32);
		// test 6 hours
		// test 1 hours
		let th_day = T::OneDayBlock::get()
			.checked_mul(&180u32.saturated_into())
			.ok_or(Error::<T>::Overflow)?;
		let deadline = now_block
			.checked_add(&th_day)
			.ok_or(Error::<T>::Overflow)?;

		if !<CalculateRewardOrderMap<T>>::contains_key(acc) {
			let order: CalculateRewardOrder<T> =
				CalculateRewardOrder::<T> { calculate_reward, start_t: now_block, deadline };
			let mut order_vec: Vec<CalculateRewardOrder<T>> = Vec::new();
			order_vec.push(order);
			let bounded_order_vec = Self::vec_to_bound::<CalculateRewardOrder<T>>(order_vec)?;
			<CalculateRewardOrderMap<T>>::insert(acc, bounded_order_vec);
		} else {
			let order1: CalculateRewardOrder<T> =
				CalculateRewardOrder::<T> { calculate_reward, start_t: now_block, deadline };
			let mut order_vec = CalculateRewardOrderMap::<T>::get(acc);
			order_vec.try_push(order1).map_err(|_e| Error::<T>::StorageLimitReached)?;
			<CalculateRewardOrderMap<T>>::insert(acc, order_vec);
		}

		Ok(())
	}

	//Get the available space on the current chain.
	pub fn get_space() -> Result<u128, DispatchError> {
		let purchased_space = <PurchasedSpace<T>>::get();
		let total_space = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
		//If the total space on the current chain is less than the purchased space, 0 will be
		// returned.
		if total_space < purchased_space {
			return Ok(0);
		}
		//Calculate available space.
		let value = total_space.checked_sub(purchased_space).ok_or(Error::<T>::Overflow)?;

		Ok(value)
	}

	pub fn add_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|purchased_space| -> DispatchResult {
			let total_space = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
			if *purchased_space + size > total_space {
				Err(<Error<T>>::InsufficientAvailableSpace)?;
			}
			*purchased_space = purchased_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn sub_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|purchased_space| -> DispatchResult {
			*purchased_space = purchased_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	fn check_collateral_limit(power: u128) -> Result<BalanceOf<T>, Error<T>> {
		let mut current_power_num: u128 = 1;
		current_power_num += power.checked_div(1024 * 1024 * M_BYTE).ok_or(Error::<T>::Overflow)?;
		//2000TCESS/TB(space)
		let limit: BalanceOf<T> = (current_power_num
			.checked_mul(2_000_000_000_000_000u128)
			.ok_or(Error::<T>::Overflow)?)
		.try_into()
		.map_err(|_e| Error::<T>::ConversionError)?;

		Ok(limit)
	}

	fn check_state(acc: &AccountOf<T>) -> Result<Vec<u8>, Error<T>> {
		Ok(<MinerItems<T>>::try_get(acc).map_err(|_e| Error::<T>::NotMiner)?.state.to_vec())
	}

	fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::ItemLimit>, DispatchError> {
		let result: BoundedVec<P, T::ItemLimit> =
			param.try_into().map_err(|_e| Error::<T>::StorageLimitReached)?;

		Ok(result)
	}
}

impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&T::PalletId::get().into_account(), amount);
		<CurrencyReward<T>>::mutate(|v| {
			*v = *v + numeric_amount;
		});

		Self::deposit_event(Event::Deposit { balance: numeric_amount });
	}
}

pub trait MinerControl<AccountId> {
	fn add_power(acc: &AccountId, power: u128) -> DispatchResult;
	fn sub_power(acc: AccountId, power: u128) -> DispatchResult;
	fn add_space(acc: &AccountId, power: u128) -> DispatchResult;
	fn sub_space(acc: &AccountId, power: u128) -> DispatchResult;
	fn get_power_and_space(acc: AccountId) -> Result<(u128, u128), DispatchError>;
	fn get_miner_id(acc: AccountId) -> Result<u64, DispatchError>;
	fn punish_miner(
		acc: AccountId,
		failure_num: u32,
		total_proof: u32,
		consecutive_fines: u8,
	) -> DispatchResult;
	fn miner_is_exist(acc: AccountId) -> bool;
	fn get_miner_state(acc: AccountId) -> Result<Vec<u8>, DispatchError>;
	fn add_purchased_space(size: u128) -> DispatchResult;
	fn sub_purchased_space(size: u128) -> DispatchResult;
	fn start_buffer_period_schedule() -> DispatchResult;
	fn get_space() -> Result<u128, DispatchError>;
}

impl<T: Config> MinerControl<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn add_power(acc: &<T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::add_power(acc, power)?;
		Ok(())
	}

	fn sub_power(acc: <T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::sub_power(&acc, power)?;
		Ok(())
	}

	fn add_space(acc: &<T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::add_space(acc, power)?;
		Ok(())
	}

	fn sub_space(acc: &<T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::sub_space(&acc, power)?;
		Ok(())
	}

	fn punish_miner(
		acc: <T as frame_system::Config>::AccountId,
		failure_num: u32,
		total_proof: u32,
		consecutive_fines: u8,
	) -> DispatchResult {
		Pallet::<T>::punish(acc, failure_num.into(), total_proof.into(), consecutive_fines.into())?;
		Ok(())
	}

	fn start_buffer_period_schedule() -> DispatchResult {
		Pallet::<T>::start_buffer_period_schedule()?;
		Ok(())
	}

	fn get_power_and_space(
		acc: <T as frame_system::Config>::AccountId,
	) -> Result<(u128, u128), DispatchError> {
		if !<MinerItems<T>>::contains_key(&acc) {
			Err(Error::<T>::NotMiner)?;
		}
		//There is a judgment on whether the primary key exists above
		let miner = <MinerItems<T>>::try_get(&acc).map_err(|_| Error::<T>::NotMiner)?;
		Ok((miner.power, miner.space))
	}

	fn miner_is_exist(acc: <T as frame_system::Config>::AccountId) -> bool {
		if <MinerItems<T>>::contains_key(&acc) {
			return true;
		}
		false
	}

	fn get_miner_id(acc: AccountOf<T>) -> Result<u64, DispatchError> {
		let miner = <MinerItems<T>>::try_get(&acc).map_err(|_| Error::<T>::NotMiner)?;
		Ok(miner.peer_id)
	}

	fn get_miner_state(acc: AccountOf<T>) -> Result<Vec<u8>, DispatchError> {
		let miner = <MinerItems<T>>::try_get(&acc).map_err(|_| Error::<T>::NotMiner)?;
		Ok(miner.state.to_vec())
	}

	fn add_purchased_space(size: u128) -> DispatchResult {
		Self::add_purchased_space(size)?;
		Ok(())
	}

	fn sub_purchased_space(size: u128) -> DispatchResult {
		Self::sub_purchased_space(size)?;
		Ok(())
	}

	fn get_space() -> Result<u128, DispatchError> {
		let size = Self::get_space()?;
		Ok(size)
	}
}

pub trait CalculFailureFee<T: Config> {
	fn calcu_failure_fee(
		acc: AccountOf<T>,
		failure_num: u32,
		total_proof: u32,
	) -> Result<u128, Error<T>>;
}

impl<T: Config> CalculFailureFee<T> for Pallet<T> {
	fn calcu_failure_fee(
		acc: AccountOf<T>,
		failure_num: u32,
		total_proof: u32,
	) -> Result<u128, Error<T>> {
		let order_vec =
			<CalculateRewardOrderMap<T>>::try_get(&acc).map_err(|_e| Error::<T>::NotExisted)?;

		match order_vec.len() {
			0 => Err(Error::<T>::NotExisted),
			n => {
				let calculate_reward = order_vec[n - 1].calculate_reward;
				let failure_rate = failure_num
					.checked_mul(100)
					.ok_or(Error::<T>::Overflow)?
					.checked_div(total_proof)
					.ok_or(Error::<T>::Overflow)?;
				Ok(calculate_reward
					.checked_mul(failure_rate.into())
					.ok_or(Error::<T>::Overflow)?
					.checked_div(100)
					.ok_or(Error::<T>::Overflow)?)
			},
		}
	}
}
