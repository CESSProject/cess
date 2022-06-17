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

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

use frame_support::{
	storage::bounded_vec::BoundedVec,
	traits::{
		schedule::{DispatchTime, Named as ScheduleNamed},
		Currency,
		ExistenceRequirement::AllowDeath,
		Get, Imbalance, LockIdentifier, OnUnbalanced, ReservableCurrency,
	},
};
mod benchmarking;
mod types;
pub mod weights;
use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResult, Dispatchable},
	pallet_prelude::DispatchError,
	PalletId,
};
use frame_system::{self as system};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, SaturatedConversion},
	RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*};
use types::*;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
const M_BYTE: u128 = 1_048_576;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{ensure, pallet_prelude::{*, ValueQuery}, traits::Get};
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
		/// The Scheduler.
		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
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

		CollingNotOver,

		NotpositiveState,

		StorageLimitReached,

		BoundedVecError,

		DataNotExist,
	}

	#[pallet::storage]
	#[pallet::getter(fn miner_cooling)]
	pub(super) type MinerColling<T: Config> =
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
	pub(super) type TotalPower<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// The total storage space to fill of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_space)]
	pub(super) type TotalSpace<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// Store all miner information
	#[pallet::storage]
	#[pallet::getter(fn miner_info)]
	pub(super) type AllMiner<T: Config> = StorageValue<
		_,
		BoundedVec<AccountOf<T>, T::ItemLimit>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_space)]
	pub(super) type PurchasedSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn available_space)]
	pub(super) type AvailableSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn calculate_reward_order)]
	pub(super) type CalculateRewardOrderMap<T: Config> = StorageMap<
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
		StorageMap<_, Blake2_128Concat, T::AccountId, RewardClaim<T::AccountId, BalanceOf<T>>>;

	/// The hashmap for checking registered or not.
	#[pallet::storage]
	#[pallet::getter(fn faucet_record)]
	pub(super) type FaucetRecordMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, FaucetRecord<BlockNumberOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn currency_reward)]
	pub(super) type CurrencyReward<T: Config> = 
		StorageValue<_, BalanceOf<T>, ValueQuery>;

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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn regnstk(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
			ip: Vec<u8>,
			staking_val: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			T::Currency::reserve(&sender, staking_val.clone())?;
			let cur_idx = PeerIndex::<T>::get();
			let peerid = cur_idx.checked_add(1).ok_or(Error::<T>::Overflow)?;
			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>> {
					peerid,
					beneficiary: beneficiary.clone(),
					ip: Self::vec_to_bound::<u8>(ip.clone())?,
					collaterals: staking_val.clone(),
					state: Self::vec_to_bound::<u8>("positive".as_bytes().to_vec())?,
					power: 0,
					space: 0,
					reward_info: ReWardInfo::<BalanceOf<T>>{
						total_reward: BalanceOf::<T>::from(0u32),
						total_rewards_currently_available: BalanceOf::<T>::from(0u32),
						totald_not_receive: BalanceOf::<T>::from(0u32),
					}
				},
			);

			<PeerIndex<T>>::put(peerid);

			AllMiner::<T>::try_mutate(|s| -> DispatchResult {
				s.try_push(sender.clone()).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Registered {
				acc: sender.clone(),
				staking_val: staking_val.clone(),
			});
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn increase_collateral(
			origin: OriginFor<T>,
			#[pallet::compact] collaterals: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			T::Currency::reserve(&sender, collaterals)?;
			let mut balance: BalanceOf<T> = 0u32.saturated_into();
			<MinerItems<T>>::try_mutate(&sender, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.collaterals =
					s.collaterals.checked_add(&collaterals).ok_or(Error::<T>::Overflow)?;
				balance = s.collaterals;
				if s.state == "frozen".as_bytes().to_vec() ||
					s.state == "e_frozen".as_bytes().to_vec()
				{
					let limit = Self::check_collateral_limit(sender.clone())?;
					if s.collaterals > limit {
						if s.state.to_vec() == "frozen".as_bytes().to_vec() {
							s.state = Self::vec_to_bound("positive".as_bytes().to_vec())?;
						} else {
							s.state = Self::vec_to_bound("exit".as_bytes().to_vec())?;
						}
					}
				}

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseCollateral { acc: sender, balance });

			Ok(())
		}

		//Miner exit method, Irreversible process.
		#[pallet::weight(1_000_000)]
		pub fn exit_miner(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			let state = Self::check_state(sender.clone());
			if state != "positive".as_bytes().to_vec() {
				Err(Error::<T>::NotpositiveState)?;
			}

			MinerItems::<T>::try_mutate(&sender, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();

				Self::sub_space(sender.clone(), s.space)?;
				Self::sub_power(sender.clone(), s.power)?;

				s.state = Self::vec_to_bound("exit".as_bytes().to_vec())?;
				Ok(())
			})?;

			let now = <frame_system::Pallet<T>>::block_number();
			MinerColling::<T>::insert(&sender, now);

			Self::deposit_event(Event::<T>::MinerExit { acc: sender });
			Ok(())
		}

		//Method for miners to redeem deposit
		#[pallet::weight(200_000)]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			let state = Self::check_state(sender.clone());
			if state != "exit".as_bytes().to_vec() {
				Err(Error::<T>::NotExisted)?;
			}
			let now: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
			ensure!(MinerColling::<T>::contains_key(&sender), Error::<T>::CollingNotOver);
			let colling_line: u128 = MinerColling::<T>::get(&sender).unwrap().saturated_into();
			if colling_line + 57600 > now {
				Err(Error::<T>::CollingNotOver)?;
			}

			let collaterals = MinerItems::<T>::get(&sender).unwrap().collaterals;
			T::Currency::unreserve(&sender, collaterals);
			Self::delete_miner_info(sender.clone())?;
			MinerColling::<T>::remove(&sender);

			Self::deposit_event(Event::<T>::MinerClaim { acc: sender });
			Ok(())
		}
		// 	storage_info_vec.append(&mut info1);

		// 	<StorageInfoVec<T>>::put(storage_info_vec);
		// 	Self::deposit_event(Event::<T>::TimingStorageSpace());
		// 	Ok(())
		// }
		/// Add reward orders.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_increase_rewards())]
		pub fn timed_increase_rewards(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let total_power = <TotalPower<T>>::get();
			ensure!(total_power != 0, Error::<T>::DivideByZero);
			// let reward_pot = T::PalletId::get().into_account();

			for (acc, detail) in <MinerItems<T>>::iter() {
				if detail.power == 0 {
					continue
				}
				let mut award: u128 = <CurrencyReward<T>>::get().try_into().map_err(|_| Error::<T>::Overflow)?;
				if award > 1_306_849 {
					<CurrencyReward<T>>::try_mutate(|v| -> DispatchResult {
						*v = v.checked_sub(
							&1306849u128.try_into().map_err(|_| Error::<T>::Overflow)?
						).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?;
					award = 1306849;
				} else {
					<CurrencyReward<T>>::try_mutate(|v| -> DispatchResult {
						*v = 0u128.try_into().map_err(|_| Error::<T>::Overflow)?;
						Ok(())
					})?;
				}
				let tmp1: u128 = award
					.checked_mul(detail.power)
					.ok_or(Error::<T>::Overflow)?;
				let tmp2: u128 = tmp1.checked_div(total_power).ok_or(Error::<T>::Overflow)?;
				let _ = Self::add_reward_order1(acc, tmp2);

				// Give 20% reward to users in advance
				// let reward_20_percent: BalanceOf<T> =
				// (tmp2.checked_mul(2).ok_or(Error::<T>::Overflow)?.checked_div(10).ok_or(Error::
				// <T>::Overflow)?) .try_into()
				// .map_err(|_e| Error::<T>::ConversionError)?;

				// <T as pallet::Config>::Currency::transfer(&reward_pot, &detail.beneficiary,
				// reward_20_percent, AllowDeath)?;
			}

			Self::deposit_event(Event::<T>::TimedTask());
			Ok(())
		}
		/// Added timed tasks for reward orders.
		///
		/// The dispatch origin of this call must be _root_.
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
				60,
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
		/// Delete reward orders.
		///
		/// The dispatch origin of this call must be _root_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::del_reward_order())]
		pub fn del_reward_order(
			origin: OriginFor<T>,
			acc: AccountOf<T>,
			order_num: u128,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			ensure!(CalculateRewardOrderMap::<T>::contains_key(&acc), Error::<T>::NotExisted);
			let mut order_vec = CalculateRewardOrderMap::<T>::get(&acc);
			order_vec.remove(order_num.try_into().map_err(|_e| Error::<T>::BoundedVecError)?);
			<CalculateRewardOrderMap<T>>::insert(acc, order_vec);

			// Self::deposit_event(Event::<T>::Del(sender.clone()));
			Ok(())
		}

		/// Users receive rewards for scheduled tasks.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_user_receive_award1())]
		pub fn timed_user_receive_award1(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			for (sender, info) in <RewardClaimMap<T>>::iter() {
				let acc = info.clone().beneficiary;
				let state =
					<MinerItems<T>>::try_get(&sender).map_err(|_e| Error::<T>::NotMiner)?.state;

				if state == "frozen".as_bytes().to_vec() || state == "e_frozen".as_bytes().to_vec()
				{
					Self::deposit_event(Event::<T>::AlreadyFrozen { acc: acc.clone() });
					continue
				}

				let reward_pot = T::PalletId::get().into_account();

				let reward_claim1 =
					RewardClaimMap::<T>::try_get(&sender).map_err(|_e| Error::<T>::NotMiner)?;

				let award = reward_claim1.current_availability;
				let total = reward_claim1.total_reward;

				ensure!(
					reward_claim1
						.have_to_receive
						.checked_add(&award)
						.ok_or(Error::<T>::Overflow)? <=
						reward_claim1.total_reward,
					Error::<T>::BeyondClaim
				);

				<T as pallet::Config>::Currency::transfer(&reward_pot, &acc, award, AllowDeath)?;

				RewardClaimMap::<T>::try_mutate(&sender, |reward_claim_opt| -> DispatchResult {
					let reward_claim = reward_claim_opt.as_mut().unwrap();
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
				MinerItems::<T>::try_mutate(&sender, |miner_opt| -> DispatchResult {
					let miner = miner_opt.as_mut().unwrap();
					let total_not_receive = info.total_not_receive;
					miner.reward_info.totald_not_receive = total_not_receive;
					Ok(())
				})?;

				if Self::check_exist_miner_reward(sender.clone()) {
					Self::clean_reward_map(sender)
				}
			}
			Ok(())
		}
		/// Users receive rewards for scheduled tasks.
		///
		/// The dispatch origin of this call must be _root_.
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
				63,
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_task_award_table())]
		pub fn timed_task_award_table(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			for (acc, order_vec) in <CalculateRewardOrderMap<T>>::iter() {
				if !<MinerItems<T>>::contains_key(&acc) {
					Self::clean_reward_map(acc.clone());
					continue
				}

				let mut total: u128 = 0;

				let now = <frame_system::Pallet<T>>::block_number();
				let mut avail: BalanceOf<T> =
					0u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;

				for i in &order_vec.to_vec() {
					total = total.checked_add(i.calculate_reward).ok_or(Error::<T>::Overflow)?;
					if i.deadline > now {
						// let tmp1 =
						// TryInto::<u128>::try_into(now.checked_sub(&i.start_t).ok_or(Error::<T>::
						// Overflow)?).ok().unwrap(); let day:u128 = tmp/28800+1;
						// // test 5 minutes
						// let day:u128 = tmp/100+1;
						// test 6 hours
						// let day:u128 =
						// tmp1.checked_div(7200).ok_or(Error::<T>::Overflow)?.checked_add(1).
						// ok_or(Error::<T>::Overflow)?;
						let tmp2: BalanceOf<T> = (i.calculate_reward * 8 / 10 / 180)
							.try_into()
							.map_err(|_e| Error::<T>::ConversionError)?;
						avail = avail.checked_add(&tmp2).ok_or(Error::<T>::Overflow)?;
					}
					// else {
					// 	let tmp1:BalanceOf<T> = (i.calculate_reward*8/10).try_into().map_err(|_e|
					// Error::<T>::ConversionError)?; 	avail = avail.checked_add(&tmp1).ok_or(Error::
					// <T>::Overflow)?; 	// Call::del_order(_acc,i);
					// }
				}

				let mut order_clone = order_vec.clone();
				order_clone.retain(|item| if item.deadline > now { true } else { false });

				let total_20_percent: BalanceOf<T> = total
					.checked_mul(2)
					.ok_or(Error::<T>::Overflow)?
					.checked_div(10)
					.ok_or(Error::<T>::Overflow)?
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
						MinerItems::<T>::try_mutate(
							&acc,
							|miner_opt| -> DispatchResult {
								let miner = miner_opt.as_mut().unwrap();
								miner.reward_info.total_reward = reward2;
								miner.reward_info.total_rewards_currently_available =
									currently_available
										.checked_add(&total_20_percent)
										.ok_or(Error::<T>::Overflow)?;
								miner.reward_info.totald_not_receive = reward2;
								Ok(())
							},
						)?;
					}
				} else {
					RewardClaimMap::<T>::try_mutate(&acc, |reward_claim_opt| -> DispatchResult {
						//Convert balance to U128 for multiplication and division
						let reward_claim = reward_claim_opt.as_mut().unwrap();
						let diff = reward2
							.checked_sub(&reward_claim.total_reward)
							.ok_or(Error::<T>::Overflow)?;
						let diff128 =
							TryInto::<u128>::try_into(diff).map_err(|_e| Error::<T>::Overflow)?;
						let diff_20_percent: BalanceOf<T> = diff128
							.checked_mul(2)
							.ok_or(Error::<T>::Overflow)?
							.checked_div(10)
							.ok_or(Error::<T>::Overflow)?
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
						MinerItems::<T>::try_mutate(
							&acc,
							|miner_opt| -> DispatchResult {
								let miner = miner_opt.as_mut().unwrap();
								miner.reward_info.total_reward = reward2;

								let reward_claim_map = RewardClaimMap::<T>::try_get(&acc)
									.map_err(|_e| Error::<T>::NotMiner)?;
									miner.reward_info.total_rewards_currently_available = reward_claim_map
									.have_to_receive
									.checked_add(&reward_claim_map.current_availability)
									.ok_or(Error::<T>::Overflow)?;

								let total_not_receive = RewardClaimMap::<T>::try_get(&acc)
									.map_err(|_e| Error::<T>::NotMiner)?
									.total_not_receive;
									miner.reward_info.totald_not_receive = total_not_receive;
								Ok(())
							},
						)?;
					}
				}

				<CalculateRewardOrderMap<T>>::insert(acc, order_clone);
			}

			Self::deposit_event(Event::<T>::TimedTask());
			Ok(())
		}
		/// Update the user reward table for scheduled tasks.
		///
		/// The dispatch origin of this call must be _root_.
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
				61,
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::faucet_top_up())]
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::faucet())]
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
					10000000000000000u128.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					AllowDeath,
				)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> { last_claim_time: now },
				);
			} else {
				let faucet_record = FaucetRecordMap::<T>::try_get(&to).map_err(|e| {
					log::error!("faucet error is: {:?}", e);
					Error::<T>::DataNotExist
				})?;
				let now = <frame_system::Pallet<T>>::block_number();

				let mut flag: bool = true;
				if now >= BlockNumberOf::<T>::from(28800u32) {
					if !(faucet_record.last_claim_time <=
						now.checked_sub(&BlockNumberOf::<T>::from(28800u32))
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
					10000000000000000u128.try_into().map_err(|_e| Error::<T>::ConversionError)?,
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
	/// Use aid to get to peerid.
	///
	/// Parameters:
	/// - `aid`: aid.
	pub fn get_peerid(aid: &AccountOf<T>) -> u64 {
		if !<MinerItems<T>>::contains_key(&aid) {
			frame_support::print("UnregisteredAccountId");
		}
		let peerid = MinerItems::<T>::get(&aid).unwrap().peerid;
		peerid
	}
	/// Add computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_power(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(&acc) {
			return Ok(())
		}

		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		Self::add_available_space(increment.clone())?;
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = s.power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalPower::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// Sub computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_power(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(&acc) {
			return Ok(())
		}

		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		Self::sub_available_space(increment.clone())?;
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = s.power.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalPower::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	/// Add space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(&acc) {
			return Ok(())
		}

		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.space = s.space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		TotalSpace::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// Sub space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(&acc) {
			return Ok(())
		}

		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.space = s.space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		TotalSpace::<T>::mutate(|s| -> DispatchResult {
			*s = s.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// According to aid to punish.
	///
	/// Parameters:
	/// - `aid`: aid.
	pub fn punish(aid: AccountOf<T>, file_size: u128) -> DispatchResult {
		if !<MinerItems<T>>::contains_key(&aid) {
			return Ok(())
		}
		//There is a judgment on whether the primary key exists above
		let mr = MinerItems::<T>::get(&aid).unwrap();
		let acc = T::PalletId::get().into_account();
		let growth: u128 = file_size
			.checked_div(1_048_576)
			.ok_or(Error::<T>::Overflow)?
			.checked_div(2)
			.ok_or(Error::<T>::Overflow)?;
		let punish_amount: BalanceOf<T> = 400_000_000_000_000u128
			.checked_add(growth)
			.ok_or(Error::<T>::Overflow)?
			.try_into()
			.map_err(|_e| Error::<T>::ConversionError)?;

		if mr.collaterals < punish_amount {
			T::Currency::unreserve(&aid, mr.collaterals);
			Self::delete_miner_info(aid.clone())?;
			return Ok(())
		}
		T::Currency::unreserve(&aid, punish_amount);
		MinerItems::<T>::try_mutate(&aid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.collaterals =
				s.collaterals.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			let limit = Self::check_collateral_limit(aid.clone())?;
			if mr.collaterals < limit &&
				s.state != "frozen".as_bytes().to_vec() &&
				s.state != "e_frozen".as_bytes().to_vec()
			{
				if s.state.to_vec() == "positive".as_bytes().to_vec() {
					s.state = Self::vec_to_bound::<u8>("frozen".as_bytes().to_vec())?;
				} else if s.state.to_vec() == "exit".as_bytes().to_vec() {
					s.state = Self::vec_to_bound::<u8>("e_frozen".as_bytes().to_vec())?;
				}
			}
			Ok(())
		})?;

		T::Currency::transfer(&aid, &acc, punish_amount, AllowDeath)?;

		Ok(())
	}

	fn delete_miner_info(acc: AccountOf<T>) -> DispatchResult {
		//There is a judgment on whether the primary key exists above
		let miner = <MinerItems<T>>::get(&acc).unwrap();
		TotalPower::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(miner.power).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalSpace::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(miner.space).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		AvailableSpace::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(miner.power).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| if *s == acc.clone() { false } else { true });
		AllMiner::<T>::put(miner_list);

		<MinerItems<T>>::remove(&acc);
		Ok(())
	}

	//Check whether the rewards for exited miners have been paid out.
	//true is Distribution completed. false is Unfinished
	fn check_exist_miner_reward(acc: AccountOf<T>) -> bool {
		if !<MinerItems<T>>::contains_key(&acc) {
			let order_vec = <CalculateRewardOrderMap<T>>::get(&acc);
			if order_vec.len() == 0 {
				let reward_map = <RewardClaimMap<T>>::get(&acc).unwrap();
				if reward_map.have_to_receive == reward_map.total_reward {
					return true
				}
			}
		}

		false
	}

	fn clean_reward_map(acc: AccountOf<T>) {
		<CalculateRewardOrderMap<T>>::remove(&acc);
		<RewardClaimMap<T>>::remove(&acc);
	}
	/// Add reward orders for corresponding accounts.
	///
	/// Parameters:
	/// - `acc`: Rewards account.
	/// - `calculate_reward`: Calculate the reward.
	pub fn add_reward_order1(acc: AccountOf<T>, calculate_reward: u128) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();
		// With block timing, 180 days =5184000 blocks
		// let deadline = now + T::BlockNumber::from(5184000u32);
		// // test 5 minutes
		// let deadline = now + T::BlockNumber::from(18000u32);
		// test 6 hours
		// test 1 hours
		let deadline =
			now.checked_add(&T::BlockNumber::from(5184000u32)).ok_or(Error::<T>::Overflow)?;

		if !<CalculateRewardOrderMap<T>>::contains_key(&acc) {
			let order: CalculateRewardOrder<T> =
				CalculateRewardOrder::<T> { calculate_reward, start_t: now, deadline };
			let mut order_vec: Vec<CalculateRewardOrder<T>> = Vec::new();
			order_vec.push(order);
			let bounded_order_vec = Self::vec_to_bound::<CalculateRewardOrder<T>>(order_vec)?;
			<CalculateRewardOrderMap<T>>::insert(acc, bounded_order_vec);
		} else {
			let order1: CalculateRewardOrder<T> =
				CalculateRewardOrder::<T> { calculate_reward, start_t: now, deadline };
			let mut order_vec = CalculateRewardOrderMap::<T>::get(&acc);
			order_vec.try_push(order1).map_err(|_e| Error::<T>::StorageLimitReached)?;
			<CalculateRewardOrderMap<T>>::insert(acc, order_vec);
		}
		Ok(())
	}

	//Get the available space on the current chain.
	pub fn get_space() -> Result<u128, DispatchError> {
		let purchased_space = <PurchasedSpace<T>>::get();
		let total_space = <AvailableSpace<T>>::get();
		//If the total space on the current chain is less than the purchased space, 0 will be
		// returned.
		if total_space < purchased_space {
			return Ok(0)
		}
		//Calculate available space.
		let value = total_space.checked_sub(purchased_space).ok_or(Error::<T>::Overflow)?;
		return Ok(value)
	}

	pub fn add_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|s| -> DispatchResult {
			let available_space = <AvailableSpace<T>>::get();
			if *s + size > available_space {
				Err(<Error<T>>::InsufficientAvailableSpace)?;
			}
			*s = s.checked_add(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}
	pub fn sub_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}
	pub fn add_available_space(size: u128) -> DispatchResult {
		<AvailableSpace<T>>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_add(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}
	pub fn sub_available_space(size: u128) -> DispatchResult {
		<AvailableSpace<T>>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}

	fn check_collateral_limit(acc: AccountOf<T>) -> Result<BalanceOf<T>, Error<T>> {
		let power = <MinerItems<T>>::try_get(&acc)
			.map_err(|_e| Error::<T>::DataNotExist)?
			.power;
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

	fn check_state(acc: AccountOf<T>) -> Vec<u8> {
		<MinerItems<T>>::get(&acc).unwrap().state.to_vec()
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
	fn add_power(acc: AccountId, power: u128) -> DispatchResult;
	fn sub_power(acc: AccountId, power: u128) -> DispatchResult;
	fn add_space(acc: AccountId, power: u128) -> DispatchResult;
	fn sub_space(acc: AccountId, power: u128) -> DispatchResult;
	fn get_power_and_space(acc: AccountId) -> Result<(u128, u128), DispatchError>;
	fn punish_miner(acc: AccountId, file_size: u64) -> DispatchResult;
	fn miner_is_exist(acc: AccountId) -> bool;
}

impl<T: Config> MinerControl<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn add_power(acc: <T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::add_power(acc, power)?;
		Ok(())
	}

	fn sub_power(acc: <T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::sub_power(acc, power)?;
		Ok(())
	}

	fn add_space(acc: <T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::add_space(acc, power)?;
		Ok(())
	}

	fn sub_space(acc: <T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::sub_space(acc, power)?;
		Ok(())
	}

	fn punish_miner(acc: <T as frame_system::Config>::AccountId, file_size: u64) -> DispatchResult {
		Pallet::<T>::punish(acc, file_size.into())?;
		Ok(())
	}

	fn get_power_and_space(acc: <T as frame_system::Config>::AccountId) -> Result<(u128, u128), DispatchError> {
		if !<MinerItems<T>>::contains_key(&acc) {
			Err(Error::<T>::NotMiner)?;
		}
		//There is a judgment on whether the primary key exists above
		let miner = <MinerItems<T>>::try_get(&acc).map_err(|_| Error::<T>::NotMiner)?;
		Ok((miner.power, miner.space))
	}

	fn miner_is_exist(acc: <T as frame_system::Config>::AccountId) -> bool {
		if <MinerItems<T>>::contains_key(&acc) {
			return true
		}
		false
	}
}
